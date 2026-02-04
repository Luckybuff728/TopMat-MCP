//! 智能体间协作 (A2A) 流式处理器
//!
//! 本智能体演示了多个专门化智能体之间的协作流程：
//! 1. 研究智能体 (Researcher)：负责收集基础信息。
//! 2. 分析智能体 (Analyst)：负责处理和分析收集到的信息。
//! 3. 审查智能体 (Reviewer)：负责对分析结果进行最终审查。
//!
//! 支持子智能体链式调用的实时流式输出。

use super::history::HistoryManager;
use crate::server::database::DatabaseConnection;
use crate::server::middleware::auth::AuthUser;
use crate::server::models::*;
use crate::server::request::handle_chat_request;
use rig::agent::AgentBuilder;
use rig::prelude::*;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 基础调研工具
#[derive(Deserialize, Serialize)]
struct BaseResearchTool;

#[derive(Debug)]
struct ResearchError;

impl std::fmt::Display for ResearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "调研工具错误")
    }
}

impl std::error::Error for ResearchError {}

#[derive(Deserialize)]
struct ResearchArgs {
    topic: String,
}

impl Tool for BaseResearchTool {
    const NAME: &'static str = "base_research";
    type Error = ResearchError;
    type Args = ResearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "获取指定主题的基础调研数据".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "研究的主题"
                    }
                },
                "required": ["topic"],
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!("[research-tool] 正在获取主题数据: {}", args.topic);
        Ok(format!(
            "这是关于'{}'的基础调研原始数据：[数据点A, 数据点B, 数据点C]。",
            args.topic
        ))
    }
}

/// 智能体间协作流式处理器 (多智能体流程版)
pub async fn a2a_agent(
    db: DatabaseConnection,
    request: ChatRequest,
    _auth_user: AuthUser,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let client = rig::providers::qwen::Client::new_with_api_key(api_key);
    let model = client.completion_model("qwen-plus");

    // 1. 研究智能体 (Researcher) - 最底层，带基础工具
    let researcher = AgentBuilder::new(model.clone())
        .name("researcher")
        .description("负责收集和整理基础信息的智能体")
        .preamble(
            "你是一个专业的调研员。请使用基础调研工具 (base_research) 收集信息并进行初级整理。",
        )
        .max_tokens(512)
        .tool(BaseResearchTool)
        .build();

    // 2. 分析智能体 (Analyst) - 中间层，使用研究智能体作为工具
    let analyst = AgentBuilder::new(model.clone())
        .name("analyst")
        .description("负责深入分析研究数据的智能体")
        .preamble("你是一个数据分析专家。你会调用 researcher 智能体获取原始数据，然后对其进行深入的技术分析和趋势判断。")
        .max_tokens(512)
        .tool(researcher)
        .build();

    // 3. 审查智能体 (Reviewer) - 最高层，使用分析智能体作为工具
    // 或者主智能体直接按需调用它们。为了体现“流程”，我们建立一个更有趣的结构：
    // 主智能体 (Orchestrator) 拥有 Analyst 和 Reviewer 作为工具。

    let reviewer = AgentBuilder::new(model.clone())
        .name("reviewer")
        .description("负责对报告进行质量把控和纠错的智能体")
        .preamble("你是一个资深的主编和审查专家。你会对收到的分析报告进行审查，提出改进意见或进行最后的纠错。")
        .max_tokens(512)
        .build();

    // 4. 总控智能体 (Main Orchestrator)
    let main_agent = AgentBuilder::new(model.clone())
        .name("orchestrator")
        .preamble("
            你是一个高级协作主管，负责编排调研、分析和审查的完整流程。
            
            **强制性执行规则（Human-in-the-Loop）**：
            1. 在你准备调用 `analyst` 进行深度分析之前，你必须先向用户简要描述你打算研究的重点和分析的方向，并明确询问：'我是否应该开始进行深度分析？'。
            2. **严禁**在未获得用户明确肯定答复（如“同意”、“执行”、“开始”等）的情况下调用 `analyst` 工具。
            3. 在获取 `analyst` 的报告后，你也必须先告知用户分析已完成，并询问：'是否需要提交给审查员 (reviewer) 进行最后把关？'。
            4. 只有在用户再次确认后，才能调用 `reviewer` 工具。
            5. 如果用户对某个环节有修改意见，你应当先根据意见调整，再次确认后执行。
            
            你的目标是让用户完全掌控流程的进度。
        ")
        .max_tokens(4096)
        .tool(analyst)
        .tool(reviewer)
        .build();

    // 获取对话历史
    let history = if let Some(cvid) = &request.conversation_id {
        HistoryManager::new(db).get_context(cvid).await.ok()
    } else {
        None
    };

    handle_chat_request(main_agent, request, history).await
}
