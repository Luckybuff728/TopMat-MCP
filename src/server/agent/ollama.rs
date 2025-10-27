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
        handle_normal_request(agent, request, "Ollama").await
    }
}

/// 处理Ollama请求并返回ChatResponse (ollama-llama3)
pub async fn ollama_llama3(
    request: ChatRequest,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let system_prompt = request.system_prompt.as_deref().unwrap_or("You are a helpful AI assistant.");
    let temperature = request.temperature.unwrap_or(0.8);
    let agent = rig::providers::ollama::Client::new()
        .agent("llama3:latest")
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    if request.stream {
        handle_streaming_request(agent, request).await
    } else {
        handle_normal_request(agent, request, "Ollama").await
    }
}


pub async fn handle_ollama_normal(
    agent: rig::agent::Agent<rig::providers::ollama::CompletionModel>,
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    let (response, _) = handle_normal_request(agent, request, "Ollama").await?;
    Ok(response)
}

pub async fn handle_ollama_streaming(
    agent: rig::agent::Agent<rig::providers::ollama::CompletionModel>,
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    let (response, _) = handle_streaming_request(agent, request).await?;
    Ok(response)
}
