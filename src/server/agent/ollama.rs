use rig::prelude::*;

use crate::server::models::*;
use crate::server::request::{handle_normal_request, handle_streaming_request};



/// 处理Ollama请求并返回ChatResponse (ollama-qwen3-4b)
pub async fn ollama_qwen3_4b(
    request: ChatRequest,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let system_prompt = request.system_prompt.as_deref().unwrap_or("");
    let temperature = request.temperature.unwrap_or(0.8);
    let agent = rig::providers::ollama::Client::new()
        .agent("qwen3:4b")
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    if request.stream {
        handle_streaming_request(agent, request).await
    } else {
        handle_normal_request(agent, request).await
    }
}

// /// 处理Ollama请求并返回ChatResponse (ollama-llama3)
// pub async fn ollama_llama3(
//     request: ChatRequest,
// ) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
//     let system_prompt = request.system_prompt.as_deref().unwrap_or("You are a helpful AI assistant.");
//     let temperature = request.temperature.unwrap_or(0.8);
//     let agent = rig::providers::ollama::Client::new()
//         .agent("llama3:latest")
//         .preamble(system_prompt)
//         .temperature(temperature as f64)
//         .build();

//     if request.stream {
//         handle_streaming_request(agent, request).await
//     } else {
//         handle_normal_request(agent, request).await
//     }
// }

use anyhow::Result;
use rig::agent::stream_to_stdout;
use rig::{completion::ToolDefinition, providers, streaming::StreamingPrompt};

use rmcp::{
    model::{ClientCapabilities, ClientInfo, Implementation, Tool as McpTool},
    transport::{StreamableHttpClientTransport},
    transport::streamable_http_client::StreamableHttpClientTransportConfig,
    ServiceExt,
};
/// 独立测试MCP工具集成的函数
pub async fn ollama_llama3_test() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();

    // 1. 连接到 MCP 服务器
    let mcp_server_url = "http://127.0.0.1:3001/mcp".to_string();
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
            name: "Ollama-MCP client".to_string(),
            title: None,
            version: "0.0.1".to_string(),
            website_url: None,
            icons: None,
        },
    };

    // 连接到 MCP 服务器
    let mcp_client = client_info.serve(transport).await.map_err(|e| {
        anyhow::anyhow!("Failed to connect to MCP server: {}", e)
    })?;

    // 2. 获取服务器信息
    let server_info = mcp_client.peer_info();
    tracing::info!("Connected to MCP server: {server_info:#?}");

    // 列出 MCP 服务器提供的工具
    let tools_result = mcp_client
        .list_tools(Default::default())
        .await
        .map_err(|e| {
            anyhow::anyhow!("Failed to list MCP tools: {}", e)
        })?;
    let tools: Vec<McpTool> = tools_result.tools;

    // tracing::info!("Available MCP tools: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

    // 3. 创建 Ollama 客户端和 Agent
    let mut agent_builder = providers::ollama::Client::new()
        .agent("llama3.2")
        .preamble(
            "你是一个材料方向的助理，擅长数学计算和使用工具进行计算。
            ",
        )
        .max_tokens(1024)
        .tool(rig::tools::ThinkTool)
        .rmcp_tools(tools, mcp_client.peer().to_owned());

    let agent = agent_builder.build();


    let mut stream = agent.stream_prompt("列出我的任务").await;
    let res = stream_to_stdout(&mut stream).await?;

    println!("Token usage response: {usage:?}", usage = res.usage());
    println!("Final text response: {message:?}", message = res.response());

    Ok(())
}

/// 处理Ollama请求并返回ChatResponse (ollama-llama3) - 支持MCP工具
pub async fn ollama_llama3(
    request: ChatRequest,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    // 1. 连接到 MCP 服务器
    let mcp_server_url = "http://127.0.0.1:3001/mcp".to_string();
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
            name: "TopMat-LLM ollama client".to_string(),
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

    // 获取服务器信息
    let _server_info = mcp_client.peer_info();

    // 列出 MCP 服务器提供的工具
    let tools_result: rmcp::model::ListToolsResult = mcp_client
        .list_tools(Default::default())
        .await
        .map_err(|e| {
            ErrorResponse {
                error: "INTERNAL_SERVER_ERROR".to_string(),
                message: format!("Failed to list MCP tools: {}", e),
                details: None,
                timestamp: chrono::Utc::now(),
            }
        })?;

    let tools: Vec<McpTool> = tools_result.tools;

    tracing::info!("Available MCP tools: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

    // 2. 创建 Ollama 客户端和 Agent
    let system_prompt = request.system_prompt.as_deref().unwrap_or(
        "你是一个材料方向的助理，擅长数学计算和使用工具进行计算。当需要查询任务或获取信息时，必须使用提供的工具。"
    );
    let temperature = request.temperature.unwrap_or(0.8);

    let mut agent_builder = providers::ollama::Client::new()
        .agent("qwen3:8b")
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .tool(rig::tools::ThinkTool)
        .tool(rig::tools::ListTasks);
        // .rmcp_tools(tools, mcp_client.peer().to_owned());

    let agent = agent_builder.build();
    tracing::info!("Agent built successfully, starting request processing");

    // 3. 处理请求（流式或非流式）
    if request.stream {
        handle_streaming_request(agent, request).await
    } else {
        handle_normal_request(agent, request).await
    }
}