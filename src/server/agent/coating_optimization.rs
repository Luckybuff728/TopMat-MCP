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

use crate::server::middleware::auth::AuthUser;
use crate::server::models::{ChatRequest, ChatResponse, ErrorResponse};
use crate::server::request::handle_chat_request;
use rig::agent::AgentBuilder;
use rig::prelude::*;
// ============= 错误类型定义 =============

pub async fn coating_optimization(
    request: ChatRequest,
    _auth_user: AuthUser, // 目前暂不使用，但为了统一接口
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

    // ============= 手动编排流程（支持流式输出） =============

    let _ = handle_chat_request(requirement_agent, request.clone()).await;
    // 【阶段二：性能预测】
    let _ = handle_chat_request(prediction_agent, request.clone()).await;
    // 【阶段三：优化建议】
    let _ = handle_chat_request(composition_optimizer, request.clone()).await;
    // P2: 结构优化
    println!("\n--- P2: 结构优化 ---\n");
    let _ = handle_chat_request(structure_optimizer, request.clone()).await;
    // P3: 工艺优化

    let _ = handle_chat_request(process_optimizer, request.clone()).await;
    // 【阶段四：迭代优化】

    handle_chat_request(iteration_agent, request).await
}
