use rig::prelude::*;
use rig::client::{ProviderClient, CompletionClient};
use std::sync::Arc;
use crate::server::models::*;
use crate::server::request::handle_chat_request;
use crate::server::mcp::McpAgent;
use crate::server::middleware::auth::AuthUser;

use rmcp::{model::{ClientInfo, ClientCapabilities, Implementation}, ServiceExt};
use rmcp::transport::{StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig};

/// 处理通义千问请求并返回ChatResponse (qwen-plus)
pub async fn qwen_plus(
    request: ChatRequest,
    _auth_user: AuthUser,  // 目前暂不使用，但为了统一接口
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let model = &request.model;
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let system_prompt = request.system_prompt.as_deref().unwrap_or("");
    let temperature = request.temperature.unwrap_or(0.5);
    let agent = rig::providers::qwen::Client::new_with_api_key(&api_key)
        .agent(model)
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    handle_chat_request(agent, request).await
}

/// 处理通义千问请求并返回ChatResponse (qwen-turbo)
pub async fn qwen_turbo(
    request: ChatRequest,
    _auth_user: AuthUser,  // 目前暂不使用，但为了统一接口
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let system_prompt = request.system_prompt.as_deref().unwrap_or("");
    let temperature = request.temperature.unwrap_or(0.8);
    let agent = rig::providers::qwen::Client::new_with_api_key(&api_key)
        .agent("qwen-turbo")
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    handle_chat_request(agent, request).await
}

/// 处理通义千问请求并返回ChatResponse (qwen-max)
pub async fn qwen_max(
    request: ChatRequest,
    _auth_user: AuthUser,  // 目前暂不使用，但为了统一接口
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let system_prompt = request.system_prompt.as_deref().unwrap_or("");
    let temperature = request.temperature.unwrap_or(0.8);
    let agent = rig::providers::qwen::Client::new_with_api_key(&api_key)
        .agent("qwen-max")
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    handle_chat_request(agent, request).await
}

// 处理通义千问请求并返回ChatResponse (qwen-flash)
pub async fn qwen_flash(
    request: ChatRequest,
    _auth_user: AuthUser,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let system_prompt = request.system_prompt.as_deref().unwrap_or("");
    let temperature = request.temperature.unwrap_or(0.8);
    let agent = rig::providers::qwen::Client::new_with_api_key(&api_key)
        .agent("qwen-flash")
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    handle_chat_request(agent, request).await
}

/// 处理通义千问请求并返回ChatResponse (qwq-plus)
pub async fn qwq_plus(
    request: ChatRequest,
    _auth_user: AuthUser,  // 目前暂不使用，但为了统一接口
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let system_prompt = request.system_prompt.as_deref().unwrap_or("");
    let temperature = request.temperature.unwrap_or(0.8);
    let agent = rig::providers::qwen::Client::new_with_api_key(&api_key)
        .agent("qwq-plus")
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    handle_chat_request(agent, request).await
}


pub async fn CalphaMesh(
    request: ChatRequest,
    _auth_user: AuthUser,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let user_api_key = _auth_user.api_key.clone();
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let qwen_client = rig::providers::qwen::Client::new_with_api_key(&api_key);
    let prompt = format!(
        "你是一个专业的材料热力学计算助手，专门负责 CalphaMesh 模拟任务的管理。
你的主要职责包括：
1. **任务提交**：辅助用户提交点计算 (SubmitPointTask)、线计算 (SubmitLineTask) 和 Scheil 凝固模拟 (SubmitScheilTask)。请确保用户提供的组分 (composition) 总和为 1。
2. **状态追踪**：使用 GetTaskStatus 查询特定任务的进度和结果。
3. **任务管理**：使用 ListTasks 列出用户的所有模拟任务。
4. **密钥**：当调用任何 calphamesh 工具时，**必须**在参数中包含 'api_key' 字段,值为：{}。
请为用户提供准确的计算建议，在调用工具前验证参数的合理性，并以专业、简洁的方式反馈模拟结果。",
        user_api_key
    );

    let mut agent_builder = qwen_client
        .agent("qwen-flash")
        .preamble(&prompt)
        .temperature(0.8)
        .tool(crate::server::mcp::tools::SubmitPointTask)
        .tool(crate::server::mcp::tools::SubmitLineTask)
        .tool(crate::server::mcp::tools::SubmitScheilTask)
        .tool(crate::server::mcp::tools::GetTaskStatus)
        .tool(crate::server::mcp::tools::ListTasks);

    let raw_agent = agent_builder.build();
    handle_chat_request(raw_agent, request).await
}

pub async fn PhaseField(
    request: ChatRequest,
    _auth_user: AuthUser,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let user_api_key = _auth_user.api_key.clone();
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let qwen_client = rig::providers::qwen::Client::new_with_api_key(&api_key);
    let prompt = format!(
        "你是一个专业的相场模拟 (Phase Field) 专家助手，负责材料微观组织演化的数值模拟管理。
你的主要职责包括：
1. **模拟启动**：协助用户提交自发分解 (SubmitSpinodalDecompositionTask) 或物理气相沉积 (SubmitPvdSimulationTask) 模拟。
2. **任务监控**：实时获取任务列表 (GetTaskList)、查询特定任务状态 (PhaseFieldGetTaskStatus) 或在必要时终止任务 (StopTask)。
3. **数据检索**：对模拟生成的文件进行探测 (ProbeTaskFiles) 并下载关键结果文件 (RetrieveFile)。
4. **密钥**：当调用任何 Phase Field 工具时，**必须**在参数中包含 'api_key' 字段,值为：{}。
请引导用户正确配置模拟参数，确保模拟流程的完整性。在讨论模拟结果时，请结合物理背景提供深入的见解。",
        user_api_key
    );
    let mut agent = qwen_client
        .agent("qwen-flash")
        .preamble(&prompt)
        .temperature(0.8)
        .tool(crate::server::mcp::tools::SubmitSpinodalDecompositionTask)
        .tool(crate::server::mcp::tools::SubmitPvdSimulationTask)
        .tool(crate::server::mcp::tools::GetTaskList)
        .tool(crate::server::mcp::tools::PhaseFieldGetTaskStatus)
        .tool(crate::server::mcp::tools::StopTask)
        .tool(crate::server::mcp::tools::ProbeTaskFiles)
        .tool(crate::server::mcp::tools::RetrieveFile)
        .build();

    // let raw_agent = agent_builder.build();
    handle_chat_request(agent, request).await
}

pub async fn ML_Server(
    request: ChatRequest,
    _auth_user: AuthUser,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {

    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let qwen_client = rig::providers::qwen::Client::new_with_api_key(&api_key);
    let mut agent_builder = qwen_client
        .agent("qwen-flash")
        .preamble("你是一个专业的机器学习模型管理助手，专门负责 ONNX 模型的运维与推理服务。
你的主要职责包括：
1. **模型管理**：通过工具列出 (OnnxModelsList)、扫描 (OnnxScanModels) 和卸载 (OnnxUnloadModel) 服务器上的模型。
2. **配置查询**：使用 OnnxGetModelConfig 获取特定模型的输入输出节点信息及详细配置。
3. **推理执行**：根据用户输入，调用 OnnxModelInference 进行模型预测。在执行推理前，请确保已正确解析用户提供的参数并匹配模型输入要求。
请保持严谨，使用工具获取实时数据，不要提供未经证实的模型信息。如果操作失败，请清晰地向用户反馈错误原因。")
        .temperature(0.8)
        .tool(crate::server::mcp::tools::OnnxModelsList)
        .tool(crate::server::mcp::tools::OnnxScanModels)
        .tool(crate::server::mcp::tools::OnnxUnloadModel)
        .tool(crate::server::mcp::tools::OnnxGetModelConfig)
        .tool(crate::server::mcp::tools::OnnxModelInference);

    let raw_agent = agent_builder.build();
    handle_chat_request(raw_agent, request).await
}

// pub async fn qwen_flash(
//     request: ChatRequest,
//     _auth_user: AuthUser,  // 目前暂不使用，但为了统一接口
// ) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
//     // 1. 连接到 MCP 服务器
//     let mcp_server_url = "http://127.0.0.1:10001/mcp".to_string();

//     let mcp_api_key = "tk_mAeBQyrp8MvPDBD4OxR4JbM9IyN8qvml".to_string();

//     // 使用StreamableHttpClientTransportConfig添加Authorization头
//     tracing::info!("Connecting to MCP server at: {}", mcp_server_url);
//     tracing::info!("Using MCP Authorization Bearer token");

//     let config = StreamableHttpClientTransportConfig::with_uri(mcp_server_url)
//         .auth_header(mcp_api_key);

//     let transport = StreamableHttpClientTransport::from_config(config);
    
//     let client_info = ClientInfo {
//         protocol_version: Default::default(),
//         capabilities: ClientCapabilities::default(),
//         client_info: Implementation {
//             name: "TopMat-LLM client".to_string(),
//             title: None,
//             version: "0.0.1".to_string(),
//             website_url: None,
//             icons: None,
//         },
//     };

//     // 连接到 MCP 服务器
//     let mcp_client = client_info.serve(transport).await.map_err(|e| {
//         ErrorResponse {
//             error: "INTERNAL_SERVER_ERROR".to_string(),
//             message: format!("Failed to connect to MCP server: {}", e),
//             details: None,
//             timestamp: chrono::Local::now(),
//         }
//     })?;

//     // 包装成 Arc 以便在多个地方使用，并确保生命周期
//     let mcp_client = Arc::new(mcp_client);

//     // 2. 获取服务器信息
//     let server_info = mcp_client.peer_info();
//     // tracing::info!("Connected to MCP server: {server_info:#?}");

//     // 列出 MCP 服务器提供的工具
//     let tools: Vec<rmcp::model::Tool> = mcp_client
//         .list_tools(Default::default())
//         .await
//         .map_err(|e| {
//             ErrorResponse {
//                 error: "INTERNAL_SERVER_ERROR".to_string(),
//                 message: format!("Failed to list MCP tools: {}", e),
//                 details: None,
//                 timestamp: chrono::Local::now(),
//             }
//         })?
//         .tools;
//     // let tools = mcp_client.list_tools(Default::default()).await?;
//     tracing::info!("Available MCP tools: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

//     // 3. 创建 Qwen 客户端和 Agent
//     let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
//     let qwen_client = rig::providers::qwen::Client::new_with_api_key(&api_key);
//     let mut agent_builder = qwen_client
//         .agent("qwen-flash")
//         .preamble("你是一个有用的AI助手。当用户的请求需要查询任务状态、提交任务或获取信息时，你必须使用提供的工具来回答。不要猜测答案，要使用工具获取准确信息。")
//         .temperature(0.8)
//         // .rmcp_tools(tools, mcp_client.peer().to_owned())
//         .tool(crate::server::mcp::tools::ThinkTool)
//         .tool(crate::server::mcp::tools::ListTasks);

//     let raw_agent = agent_builder.build();

//     // 4. 包装成 McpAgent，持有 MCP 客户端的生命周期
//     let mcp_agent = McpAgent::new(raw_agent, mcp_client);
//     tracing::info!("McpAgent built successfully, MCP client will stay alive during processing");

//     // 5. 处理请求（流式或非流式）- 使用统一的处理函数，自动适配 McpAgent 和流式模式
//     handle_chat_request(mcp_agent, request).await
// }