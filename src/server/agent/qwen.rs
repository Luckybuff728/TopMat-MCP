use rig::prelude::*;

use crate::server::models::*;
use crate::server::request::{handle_normal_request, handle_streaming_request};

/// 处理通义千问请求
pub async fn qwen_plus(
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    let model = &request.model;
    let system_prompt = request.system_prompt.as_deref().unwrap_or("");
    let temperature = request.temperature.unwrap_or(0.5);
    let agent = rig::providers::qwen::Client::from_env()
        .agent(model)
        .preamble(system_prompt)
        .temperature(temperature as f64)
        .build();

    if request.stream {
        handle_qwen_streaming(agent, request).await
    } else {
        handle_qwen_normal(agent, request).await
    }
}

pub async fn qwen_max(
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    let agent = rig::providers::qwen::Client::from_env()
        .agent("qwen-max")
        .preamble("")
        .temperature(0.8)
        .build();

    if request.stream {
        handle_qwen_streaming(agent, request).await
    } else {
        handle_qwen_normal(agent, request).await
    }
}

pub async fn qwen_turbo(
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    let agent = rig::providers::qwen::Client::from_env()
        .agent("qwen-turbo")
        .preamble("")
        .temperature(0.8)
        .build();

    if request.stream {
        handle_qwen_streaming(agent, request).await
    } else {
        handle_qwen_normal(agent, request).await
    }
}

pub async fn qwen_flash(
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    let agent = rig::providers::qwen::Client::from_env()
        .agent("qwen-flash")
        .preamble("")
        .temperature(0.8)
        .build();

    if request.stream {
        handle_qwen_streaming(agent, request).await
    } else {
        handle_qwen_normal(agent, request).await
    }
}
pub async fn qwq_plus(
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    let agent = rig::providers::qwen::Client::from_env()
        .agent("qwq-plus")
        .preamble("")
        .temperature(0.8)
        .build();

    if request.stream {
        handle_qwen_streaming(agent, request).await
    } else {
        handle_qwen_normal(agent, request).await
    }
}
/// 处理通义千问非流式请求
pub async fn handle_qwen_normal(
    agent: rig::agent::Agent<rig::providers::qwen::CompletionModel>,
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    handle_normal_request(agent, request, "通义千问").await
}

/// 处理通义千问流式请求
pub async fn handle_qwen_streaming(
    agent: rig::agent::Agent<rig::providers::qwen::CompletionModel>,
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    handle_streaming_request(agent, request).await
}