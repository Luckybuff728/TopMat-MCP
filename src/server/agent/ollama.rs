use rig::prelude::*;
use axum::{extract::Request, Extension};

use crate::server::models::*;
use crate::server::request::handle_chat_request;
use crate::server::mcp::McpAgent;
use crate::server::middleware::auth::AuthUser;


/// 处理Ollama请求并返回ChatResponse (ollama-qwen3-4b)
pub async fn ollama_qwen3_4b(
    request: ChatRequest,
    _auth_user: crate::server::middleware::auth::AuthUser,  // 目前暂不使用，但为了统一接口
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let system_prompt = request.system_prompt.as_deref().unwrap_or("");
    let temperature = request.temperature.unwrap_or(0.8);
    let agent = rig::providers::ollama::Client::new()
        .agent("qwen3:4b")
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    handle_chat_request(agent, request).await
}

/// 处理Ollama请求并返回ChatResponse (ollama-llama3)
pub async fn ollama_llama3(
    request: ChatRequest,
    _auth_user: crate::server::middleware::auth::AuthUser,  // 目前暂不使用，但为了统一接口
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let system_prompt = request.system_prompt.as_deref().unwrap_or("");
    let temperature = request.temperature.unwrap_or(0.8);
    let agent = rig::providers::ollama::Client::new()
        .agent("llama3:latest")
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    handle_chat_request(agent, request).await
}

// use anyhow::Result;
// use rig::agent::stream_to_stdout;
// use rig::{completion::ToolDefinition, providers, streaming::StreamingPrompt};

// use rmcp::{
//     model::{ClientCapabilities, ClientInfo, Implementation, Tool as McpTool},
//     transport::{StreamableHttpClientTransport},
//     transport::streamable_http_client::StreamableHttpClientTransportConfig,
//     ServiceExt,
// };
// /// 独立测试MCP工具集成的函数
// pub async fn ollama_llama3_test() -> Result<(), anyhow::Error> {
//     tracing_subscriber::fmt().init();

//     // 1. 连接到 MCP 服务器
//     let mcp_server_url = "http://127.0.0.1:3001/mcp".to_string();
//     let mcp_api_key = "tk_zaEVQtzrfFIXKh7EnBoja8KnGIfjV0T8".to_string();

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
//             name: "Ollama-MCP client".to_string(),
//             title: None,
//             version: "0.0.1".to_string(),
//             website_url: None,
//             icons: None,
//         },
//     };

//     // 连接到 MCP 服务器
//     let mcp_client = client_info.serve(transport).await.map_err(|e| {
//         anyhow::anyhow!("Failed to connect to MCP server: {}", e)
//     })?;

//     // 2. 获取服务器信息
//     let server_info = mcp_client.peer_info();
//     tracing::info!("Connected to MCP server: {server_info:#?}");

//     // 列出 MCP 服务器提供的工具
//     let tools_result = mcp_client
//         .list_tools(Default::default())
//         .await
//         .map_err(|e| {
//             anyhow::anyhow!("Failed to list MCP tools: {}", e)
//         })?;
//     let tools: Vec<McpTool> = tools_result.tools;

//     // tracing::info!("Available MCP tools: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

//     // 3. 创建 Ollama 客户端和 Agent
//     let mut agent_builder = providers::ollama::Client::new()
//         .agent("llama3.2")
//         .preamble(
//             "你是一个材料方向的助理，擅长数学计算和使用工具进行计算。
//             ",
//         )
//         .max_tokens(1024)
//         .tool(rig::tools::ThinkTool)
//         .rmcp_tools(tools, mcp_client.peer().to_owned());

//     let agent = agent_builder.build();


//     let mut stream = agent.stream_prompt("列出我的任务").await;
//     let res = stream_to_stdout(&mut stream).await?;

//     println!("Token usage response: {usage:?}", usage = res.usage());
//     println!("Final text response: {message:?}", message = res.response());

//     Ok(())
// }

// /// 处理Ollama请求并返回ChatResponse (ollama-llama3) - 支持MCP工具
// pub async fn ollama_llama3(
//     request: ChatRequest,
//     auth_user: AuthUser,  // 使用用户的API key
// ) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
//     // 使用统一的API key获取逻辑 - 从Extension中获取
//     let user_api_key = auth_user.api_key.clone();

//     // 1. 连接到 MCP 服务器
//     let mcp_server_url = "http://127.0.0.1:10001/mcp".to_string();
//     let mcp_api_key = user_api_key.clone();

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
//             name: "TopMat-LLM ollama client".to_string(),
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

//     // 获取服务器信息
//     let _server_info = mcp_client.peer_info();

//     // 列出 MCP 服务器提供的工具
//     let tools_result: rmcp::model::ListToolsResult = mcp_client
//         .list_tools(Default::default())
//         .await
//         .map_err(|e| {
//             ErrorResponse {
//                 error: "INTERNAL_SERVER_ERROR".to_string(),
//                 message: format!("Failed to list MCP tools: {}", e),
//                 details: None,
//                 timestamp: chrono::Local::now(),
//             }
//         })?;

//     let tools: Vec<McpTool> = tools_result.tools;

//     tracing::info!("Available MCP tools: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

//     // 2. 创建 Ollama 客户端和 Agent
//     let default_prompt = format!(
//         "你是一个材料方向的助理，擅长数学计算和使用工具进行计算。\
//         \n\n重要：你的用户 CalphaMesh API key 是: {}\
//         \n当调用任何 calphamesh 工具时，必须在参数中包含 'api_key' 字段，值为这个 API key。",
//         user_api_key
//     );

//     let system_prompt = request.system_prompt.as_deref().unwrap_or(&default_prompt);
//     let temperature = request.temperature.unwrap_or(0.8);

//     let mut agent_builder = providers::ollama::Client::new()
//         .agent("qwen3:8b")
//         .preamble(system_prompt)
//         .temperature(temperature as f64)
//         .tool(crate::server::mcp::tools::ThinkTool)
//         .tool(crate::server::mcp::tools::ListTasks);
//         // .rmcp_tools(tools, mcp_client.peer().to_owned());

//     let agent = agent_builder.build();
//     let mcp_agent = McpAgent::new(agent, mcp_client);
//     tracing::info!("Agent built successfully, starting request processing");

//     // 3. 处理请求（流式或非流式）
//     handle_chat_request(mcp_agent, request).await
// }