use rig::prelude::*;
use rig::client::{ProviderClient, CompletionClient};
use std::sync::Arc;

use crate::server::models::*;
use crate::server::request::handle_chat_request;
use crate::server::mcp::McpAgent;

use rmcp::{model::{ClientInfo, ClientCapabilities, Implementation}, ServiceExt};
use rmcp::transport::{StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig};

/// 处理通义千问请求并返回ChatResponse (qwen-plus)
pub async fn qwen_plus(
    request: ChatRequest,
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
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let agent = rig::providers::qwen::Client::new_with_api_key(&api_key)
        .agent("qwen-turbo")
        .preamble("")
        .temperature(0.8)
        .build();

    handle_chat_request(agent, request).await
}

/// 处理通义千问请求并返回ChatResponse (qwen-max)
pub async fn qwen_max(
    request: ChatRequest,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let agent = rig::providers::qwen::Client::new_with_api_key(&api_key)
        .agent("qwen-max")
        .preamble("")
        .temperature(0.8)
        .build();

    handle_chat_request(agent, request).await
}

/// 处理通义千问请求并返回ChatResponse (qwen-flash)
// pub async fn qwen_flash(
//     request: ChatRequest,
// ) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
//     let agent = rig::providers::qwen::Client::from_env()
//         .agent("qwen-flash")
//         .preamble("")
//         .temperature(0.8)
//         .build();

//     if request.stream {
//         handle_streaming_request(agent, request).await
//     } else {
//         handle_normal_request(agent, request).await
//     }
// }

/// 处理通义千问请求并返回ChatResponse (qwq-plus)
pub async fn qwq_plus(
    request: ChatRequest,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let agent = rig::providers::qwen::Client::new_with_api_key(&api_key)
        .agent("qwq-plus")
        .preamble("")
        .temperature(0.8)
        .build();

    handle_chat_request(agent, request).await
}


pub async fn qwen_flash(
    request: ChatRequest,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    // 1. 连接到 MCP 服务器
    let mcp_server_url = "http://127.0.0.1:10001/mcp".to_string();

    let mcp_api_key = "tk_zaEVQtzrfFIXKh7EnBoja8KnGIfjV0T8".to_string();

    // 使用StreamableHttpClientTransportConfig添加Authorization头
    tracing::info!("Connecting to MCP server at: {}", mcp_server_url);
    tracing::info!("Using MCP Authorization Bearer token");

    let config = StreamableHttpClientTransportConfig::with_uri(mcp_server_url)
        .auth_header(mcp_api_key);

    let transport = StreamableHttpClientTransport::from_config(config);
    
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "TopMat-LLM client".to_string(),
            title: None,
            version: "0.0.1".to_string(),
            website_url: None,
            icons: None,
        },
    };

    // 连接到 MCP 服务器
    let mcp_client = client_info.serve(transport).await.map_err(|e| {
        ErrorResponse {
            error: "INTERNAL_SERVER_ERROR".to_string(),
            message: format!("Failed to connect to MCP server: {}", e),
            details: None,
            timestamp: chrono::Utc::now(),
        }
    })?;

    // 包装成 Arc 以便在多个地方使用，并确保生命周期
    let mcp_client = Arc::new(mcp_client);

    // 2. 获取服务器信息
    let server_info = mcp_client.peer_info();
    // tracing::info!("Connected to MCP server: {server_info:#?}");

    // 列出 MCP 服务器提供的工具
    let tools: Vec<rmcp::model::Tool> = mcp_client
        .list_tools(Default::default())
        .await
        .map_err(|e| {
            ErrorResponse {
                error: "INTERNAL_SERVER_ERROR".to_string(),
                message: format!("Failed to list MCP tools: {}", e),
                details: None,
                timestamp: chrono::Utc::now(),
            }
        })?
        .tools;
    // let tools = mcp_client.list_tools(Default::default()).await?;
    tracing::info!("Available MCP tools: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

    // 3. 创建 Qwen 客户端和 Agent
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let qwen_client = rig::providers::qwen::Client::new_with_api_key(&api_key);
    let mut agent_builder = qwen_client
        .agent("qwen-flash")
        .preamble("你是一个有用的AI助手。当用户的请求需要查询任务状态、提交任务或获取信息时，你必须使用提供的工具来回答。不要猜测答案，要使用工具获取准确信息。")
        .temperature(0.8)
        .rmcp_tools(tools, mcp_client.peer().to_owned());
        // .tool(rig::tools::ThinkTool)
        // .tool(rig::tools::ListTasks);

    let raw_agent = agent_builder.build();
    
    // 4. 包装成 McpAgent，持有 MCP 客户端的生命周期
    let mcp_agent = McpAgent::new(raw_agent, mcp_client);
    tracing::info!("McpAgent built successfully, MCP client will stay alive during processing");

    // 5. 处理请求（流式或非流式）- 使用统一的处理函数，自动适配 McpAgent 和流式模式
    handle_chat_request(mcp_agent, request).await
}