//! 涂层性能预测及优化专家系统
//!
//! 这是一个完整的多 Agent 编排示例，展示如何使用 qwen-plus 构建复杂的工业应用系统
//!
//! 系统架构：
//! 1. 需求提取 Agent - 收集和验证输入参数
//! 2. 性能预测 Agent - 预测涂层性能
//! 3. 优化建议 Agent - 提供分类优化方案
//! 4. 迭代优化 Agent - 管理优化迭代流程
//! 5. 主编排 Agent - 协调整个流程
//!
//! 涂层性能预测及优化专家系统（支持流式输出的手动编排版本）
//!
//! 这个版本使用手动编排方式，而不是 agent-as-tool 模式，
//! 这样可以确保每个子 agent 的响应都能流式输出，提供更好的用户体验。

use super::history::HistoryManager;
use crate::server::database::DatabaseConnection;
use crate::server::middleware::auth::AuthUser;
use crate::server::models::{ChatRequest, ChatResponse, ErrorResponse};
use crate::server::request::handle_chat_request;
use axum::response::IntoResponse;
use futures_util::StreamExt;
use rig::agent::AgentBuilder;
use rig::prelude::*;
use rig::streaming::StreamingChat;
// ============= 错误类型定义 =============

pub async fn coating_optimization(
    db: DatabaseConnection,
    request: ChatRequest,
    _auth_user: AuthUser,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let qwen_client = rig::providers::qwen::Client::new_with_api_key(api_key);
    let model = qwen_client.completion_model("qwen-plus");
    // let qwen_client = rig::providers::ollama::Client::new();
    // let model = qwen_client.completion_model("llama3.2");

    println!("=== 涂层性能预测及优化专家系统（流式编排版本） ===\n");
    println!("正在初始化 Agent 系统...\n");

    // 1. 需求提取 Agent
    let requirement_agent = AgentBuilder::new(model.clone())
        .name("需求提取专家")
        .preamble(
            "
            你是涂层需求提取专家。负责收集和整理涂层成分信息（Al、Ti、N、X元素含量）、
            记录工艺参数（气压、流量、偏压、温度）、确认涂层结构信息（厚度、分层）、
            明确应用场景和性能需求，验证数据完整性和合理性。
            输出结构化的JSON格式数据。
        ",
        )
        .temperature(0.3)
        .build();

    // 2. 性能预测 Agent
    let prediction_agent = AgentBuilder::new(model.clone())
        .name("性能预测专家")
        .preamble(
            "
            你是涂层性能预测专家。负责调用 TopPhi 模拟器预测沉积形貌、
            使用 ML 模型预测性能指标、查询历史数据进行对比、进行根因分析、评估预测置信度。
        ",
        )
        .tool(crate::server::mcp::tools::simulation::TopPhiSimulator)
        .tool(crate::server::mcp::tools::simulation::MLPerformancePredictor)
        .tool(crate::server::mcp::tools::simulation::HistoricalDataQuery)
        .temperature(0.3)
        .build();

    // 3. 成分优化 Agent
    let composition_optimizer = AgentBuilder::new(model.clone())
        .name("成分优化专家")
        .preamble(
            "
            你是涂层成分优化专家（P1优化）。分析当前成分配比的优缺点、
            基于性能目标提出成分调整建议、考虑元素间协同效应、预测成分调整后的性能变化。
            输出具体的成分调整方案和理由。
            ",
        )
        .temperature(0.4)
        .build();

    // 4. 结构优化 Agent
    let structure_optimizer = AgentBuilder::new(model.clone())
        .name("结构优化专家")
        .preamble(
            "
            你是涂层结构优化专家（P2优化）。设计多层结构方案、优化各层厚度和占比、
            设计梯度或纳米多层结构、考虑应力释放和界面结合。
            输出详细的结构设计方案。
            
        ",
        )
        .temperature(0.4)
        .build();

    // 5. 工艺优化 Agent
    let process_optimizer = AgentBuilder::new(model.clone())
        .name("工艺优化专家")
        .preamble(
            "
            你是涂层工艺优化专家（P3优化）。优化沉积工艺参数、调整气体流量比例、
            优化偏压和温度、预测工艺参数对性能的影响。
            输出具体的工艺优化方案。
        ",
        )
        .temperature(0.4)
        .build();

    // 6. 迭代优化 Agent
    let iteration_agent = AgentBuilder::new(model.clone())
        .name("迭代优化管理专家")
        .preamble(
            "
            你是迭代优化流程管理专家。管理优化迭代流程、比对预测值与实测值、
            分析偏差原因、决定下一步优化方向、生成试验工单。
            输出明确的下一步行动方案。
        ",
        )
        .tool(crate::server::mcp::tools::ExperimentalDataReader)
        .temperature(0.3)
        .build();

    // ============= 交互式单步编排流程 =============
    let accumulating_history = if let Some(cvid) = &request.conversation_id {
        HistoryManager::new(db.clone())
            .get_context(cvid)
            .await
            .ok()
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let stage_names = [
        "需求提取专家",
        "性能预测专家",
        "成分优化专家",
        "结构优化专家",
        "工艺优化专家",
        "迭代优化管理专家",
    ];

    // 通过扫描历史记录中的专家标题来确定最后完成的阶段
    let mut last_stage_index: i32 = -1;
    for message in accumulating_history.iter().rev() {
        if let rig::message::Message::Assistant { content, .. } = message {
            let text = content
                .iter()
                .filter_map(|c| match c {
                    rig::message::AssistantContent::Text(t) => Some(t.text.as_str()),
                    _ => None,
                })
                .collect::<String>();

            for (idx, name) in stage_names.iter().enumerate() {
                if text.contains(&format!("【{}】", name)) {
                    last_stage_index = idx as i32;
                    break;
                }
            }
            if last_stage_index != -1 {
                break;
            }
        }
    }

    // 检查用户意图：进入下一阶段、保持原位（反馈）、还是退回
    let user_message_lower = request.message.trim().to_lowercase();
    let is_continue = ["继续", "continue", "next", "ok", "确认", "好的"]
        .iter()
        .any(|&s| user_message_lower.contains(s));

    let is_back = ["退回", "返回", "上一步", "back", "previous", "return"]
        .iter()
        .any(|&s| user_message_lower.contains(s));

    let target_stage_index = if last_stage_index == -1 {
        // 初始状态
        0
    } else if is_back {
        // 尝试解析用户想要退回到哪个阶段
        let mut target_back_idx = (last_stage_index - 1).max(0); // 默认退回一步
        for (idx, name) in stage_names.iter().enumerate() {
            if user_message_lower.contains(&name.replace("专家", "").to_lowercase()) {
                target_back_idx = idx as i32;
                break;
            }
        }
        target_back_idx
    } else if is_continue {
        // 推进到下一阶段
        (last_stage_index + 1).min((stage_names.len() - 1) as i32)
    } else {
        // 保持在当前阶段（反馈模式）
        last_stage_index
    };

    let (agent, next_prompt_hint) = match target_stage_index {
        0 => (
            requirement_agent,
            "### 当前阶段：需求提取\n如果您确认以上信息，请输入“继续”以进行【性能预测】。",
        ),
        1 => (
            prediction_agent,
            "### 当前阶段：性能预测\n如果您确认以上信息，请输入“继续”以进行【成分优化 (P1)】。",
        ),
        2 => (
            composition_optimizer,
            "### 当前阶段：成分优化\n如果您确认以上信息，请输入“继续”以进行【结构优化 (P2)】。",
        ),
        3 => (
            structure_optimizer,
            "### 当前阶段：结构优化\n如果您确认以上信息，请输入“继续”以进行【工艺优化 (P3)】。",
        ),
        4 => (
            process_optimizer,
            "### 当前阶段：工艺优化\n如果您确认以上信息，请输入“继续”以进行【迭代优化】。",
        ),
        _ => (
            iteration_agent,
            "### 当前阶段：迭代优化\n优化建议已生成，您可以根据建议进行实验，或提出进一步的修改意见。",
        ),
    };

    let model_name = request.model.clone();
    let conversation_id = request
        .conversation_id
        .clone()
        .unwrap_or_else(crate::server::models::generate_conversation_id);
    let user_message = request.message.clone();

    let event_stream = async_stream::stream! {
        use crate::server::models::StreamChunk;
        use rig::streaming::StreamedAssistantContent;
        use rig::agent::MultiTurnStreamItem;

        let agent_name = agent.name.clone().unwrap_or_else(|| "专家".to_string());

        // 发送阶段标记
        let phase_chunk = StreamChunk::Text {
            text: format!("\n\n### 【{}】正在分析...\n\n", agent_name),
            finished: false,
        };
        if let Ok(data) = serde_json::to_string(&phase_chunk) {
            yield Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data(data));
        }

        // 获取流
        let mut stream = agent
            .stream_chat(&user_message, accumulating_history.clone())
            .multi_turn(20)
            .await;

        let mut agent_output = String::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::Text(text))) => {
                    agent_output.push_str(&text.text);
                    let chunk = StreamChunk::Text {
                        text: text.text,
                        finished: false,
                    };
                    if let Ok(data) = serde_json::to_string(&chunk) {
                        yield Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data(data));
                    }
                }
                Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::Reasoning(reasoning))) => {
                    let chunk = StreamChunk::Reasoning {
                        reasoning: reasoning.reasoning.join("\n"),
                    };
                    if let Ok(data) = serde_json::to_string(&chunk) {
                        yield Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data(data));
                    }
                }
                Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::ToolCall(tool_call))) => {
                    let chunk = StreamChunk::ToolCall {
                        id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        arguments: serde_json::to_value(&tool_call.function).unwrap_or_default(),
                    };
                    if let Ok(data) = serde_json::to_string(&chunk) {
                        yield Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data(data));
                    }
                }
                Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::ToolResult { id, result })) => {
                     let chunk = StreamChunk::ToolResult {
                        id: id.clone(),
                        result: serde_json::from_str(&result).unwrap_or_else(|_| serde_json::Value::String(result.clone())),
                    };
                    if let Ok(data) = serde_json::to_string(&chunk) {
                        yield Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data(data));
                    }
                }
                Ok(MultiTurnStreamItem::StreamItem(_)) => {
                    // 忽略 ToolCallDelta 和 Final 等中间/最终提供商特定的内容
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    // 当前 Agent 执行结束
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    let chunk = StreamChunk::Error { message: format!("Agent [{}] 发生错误: {}", agent_name, e) };
                    if let Ok(data) = serde_json::to_string(&chunk) {
                        yield Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data(data));
                    }
                    break;
                }
            }
        }

        // 发送后续操作指引
        let hint_chunk = StreamChunk::Text {
            text: format!("\n\n---\n\n{}", next_prompt_hint),
            finished: false,
        };
        if let Ok(data) = serde_json::to_string(&hint_chunk) {
            yield Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data(data));
        }

        // 发送最终完成标记
        let final_chunk = StreamChunk::Text { text: "".to_string(), finished: true };
        if let Ok(data) = serde_json::to_string(&final_chunk) {
             yield Ok::<axum::response::sse::Event, std::convert::Infallible>(axum::response::sse::Event::default().data(data));
        }
    };

    let sse_response = axum::response::Sse::new(event_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(10))
            .text("keepalive"),
    );

    let chat_response = ChatResponse {
        content: None,
        reasoning_content: None,
        tool_calls: None,
        model: model_name,
        usage: None,
        conversation_id,
        timestamp: chrono::Local::now(),
        metadata: std::collections::HashMap::new(),
    };

    Ok((sse_response.into_response(), chat_response))
}
