use std::collections::HashMap;

use axum::{
    response::{sse::Event, IntoResponse, Response, Sse},
    Json,
};
use futures_util::StreamExt;
use tracing::error;
use rig::completion::Prompt;
use rig::streaming::StreamingPrompt;

use crate::server::models::*;

/// 通用的非流式请求处理函数
pub async fn handle_normal_request<M: rig::completion::CompletionModel + Send + Sync + 'static>(
    agent: rig::agent::Agent<M>,
    request: ChatRequest,
    model_name: &str,
) -> Result<Response, ErrorResponse> {
    // 使用流式API来获取完整的使用统计，即使是非流式请求
    let mut stream = agent.stream_prompt(&request.message).await;
    let mut content = String::new();
    let mut final_response: Option<rig::agent::FinalResponse> = None;

    while let Some(item) = stream.next().await {
        match item {
            Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Text(text))) => {
                content.push_str(&text.text);
            }
            Ok(rig::agent::MultiTurnStreamItem::FinalResponse(response)) => {
                final_response = Some(response);
                break;
            }
            Err(e) => {
                error!("{} 流式处理失败: {}", model_name, e);
                // 如果流式失败，回退到简单的prompt方法
                break;
            }
            _ => {}
        }
    }

    // 如果没有获取到内容，回退到简单的prompt方法
    if content.is_empty() {
        match agent.prompt(&request.message).await {
            Ok(response) => {
                let chat_response = ChatResponse {
                    content: response,
                    model: request.model,
                    usage: None,
                    session_id: request.session_id,
                    timestamp: chrono::Utc::now(),
                    metadata: HashMap::new(),
                };
                Ok(Json(chat_response).into_response())
            }
            Err(e) => {
                error!("{} 聊天处理失败: {}", model_name, e);
                Err(ErrorResponse {
                    error: "chat_failed".to_string(),
                    message: format!("聊天处理失败: {}", e),
                    details: None,
                    timestamp: chrono::Utc::now(),
                })
            }
        }
    } else {
        // 使用流式响应的数据构建完整响应
        let usage = final_response.map(|r| {
            let u = r.usage();
            TokenUsage {
                prompt_tokens: u.input_tokens as u32,
                completion_tokens: u.output_tokens as u32,
                total_tokens: u.total_tokens as u32,
            }
        });

        let chat_response = ChatResponse {
            content,
            model: request.model,
            usage,
            session_id: request.session_id,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        Ok(Json(chat_response).into_response())
    }
}

/// 通用的流式请求处理函数
pub async fn handle_streaming_request<M: rig::completion::CompletionModel + Send + Sync + 'static>(
    agent: rig::agent::Agent<M>,
    request: ChatRequest,
) -> Result<Response, ErrorResponse>
where
    <M as rig::completion::CompletionModel>::StreamingResponse: Send + Sync + 'static,
{
    let mut stream = agent.stream_prompt(&request.message).await;

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(100);

    // 启动后台任务处理流
    let request_clone = request.clone();
    tokio::spawn(async move {
        while let Some(item) = stream.next().await {
            match item {
                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Text(text))) => {
                    let chunk = StreamChunk::Text {
                        text: text.text,
                        finished: false,
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    let _ = tx.send(Ok(Event::default().data(event_data))).await;
                }
                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Reasoning(reasoning))) => {
                    let r = reasoning.reasoning.into_iter().collect::<Vec<_>>().join("");
                    let chunk = StreamChunk::Reasoning { reasoning: r };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    let _ = tx.send(Ok(Event::default().data(event_data))).await;
                }
                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::ToolCall(tool_call))) => {
                    let chunk = StreamChunk::ToolCall {
                        name: tool_call.function.name.clone(),
                        arguments: serde_json::to_value(&tool_call.function).unwrap_or_default(),
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    let _ = tx.send(Ok(Event::default().data(event_data))).await;
                }
                Ok(rig::agent::MultiTurnStreamItem::FinalResponse(response)) => {
                    // 使用 FinalResponse 的公共方法
                    let usage = response.usage();
                    let chat_response = ChatResponse {
                        content: response.response().to_string(),
                        model: request_clone.model.clone(),
                        usage: Some(TokenUsage {
                            prompt_tokens: usage.input_tokens as u32,
                            completion_tokens: usage.output_tokens as u32,
                            total_tokens: usage.total_tokens as u32,
                        }),
                        session_id: request_clone.session_id.clone(),
                        timestamp: chrono::Utc::now(),
                        metadata: HashMap::new(),
                    };
                    let chunk = StreamChunk::Final { response: chat_response };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    let _ = tx.send(Ok(Event::default().data(event_data))).await;
                    break;
                }
                Err(e) => {
                    let chunk = StreamChunk::Error { message: e.to_string() };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    let _ = tx.send(Ok(Event::default().data(event_data))).await;
                    break;
                }
                _ => {}
            }
        }
    });

    let sse_response = Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx))
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keepalive-text"),
        );

    Ok(sse_response.into_response())
}