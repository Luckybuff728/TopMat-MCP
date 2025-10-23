use rig::prelude::*;

use crate::server::models::*;
use crate::server::request::{handle_normal_request, handle_streaming_request};


pub async fn ollama_qwen3_4b(
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    let agent = rig::providers::ollama::Client::new()
        .agent("qwen3:4b")
        // .preamble("You are a helpful AI assistant.")
        .temperature(0.8)
        .build();

    if request.stream {
        handle_ollama_streaming(agent, request).await
    } else {
        handle_ollama_normal(agent, request).await
    }
}

pub async fn ollama_llama3(
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    let agent = rig::providers::ollama::Client::new()
        .agent("llama3:latest")
        .preamble("You are a helpful AI assistant.")
        .temperature(0.8)
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
    handle_normal_request(agent, request, "Ollama").await
}

pub async fn handle_ollama_streaming(
    agent: rig::agent::Agent<rig::providers::ollama::CompletionModel>,
    request: ChatRequest,
) -> Result<axum::response::Response, ErrorResponse> {
    handle_streaming_request(agent, request).await
}


// pub async fn ollama_qwen3_4b(
//     request: ChatRequest,
// ) -> Result<axum::response::Response, ErrorResponse> {
//     let model = &request.model;
//     let system_prompt = request.system_prompt.as_deref().unwrap_or("a");
//     let temperature = request.temperature.unwrap_or(0.5);
//     let agent = rig::providers::ollama::Client::new()
//         .agent(model)
//         .preamble(system_prompt)
//         .temperature(temperature as f64)
//         .build();

//     if request.stream {
//         handle_ollama_streaming(agent, request).await
//     } else {
//         handle_ollama_normal(agent, request).await
//     }
// }