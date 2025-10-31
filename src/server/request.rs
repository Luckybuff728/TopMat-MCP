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
use crate::server::mcp::McpAgent;

/// 通用的非流式请求处理函数
pub async fn handle_normal_request<M: rig::completion::CompletionModel + Send + Sync + 'static>(
    agent: rig::agent::Agent<M>,
    request: ChatRequest,
) -> Result<(Response, ChatResponse), ErrorResponse> {
    // 使用流式API来获取完整的使用统计，即使是非流式请求
    let mut stream = agent.stream_prompt(&request.message).multi_turn(5).await;
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
                // error!("{} 流式处理失败", model_name, e);
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
                    conversation_id: request.conversation_id.expect("REASON"),
                    timestamp: chrono::Utc::now(),
                    metadata: HashMap::new(),
                };
                Ok((Json(chat_response.clone()).into_response(), chat_response))
            }
            Err(e) => {
                // error!("{} 聊天处理失败: {}", model_name, e);
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
            conversation_id:  request.conversation_id.expect("REASON"),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        Ok((Json(chat_response.clone()).into_response(), chat_response))
    }
}

pub async fn handle_streaming_request<M: rig::completion::CompletionModel + Send + Sync + 'static>(
    agent: rig::agent::Agent<M>,
    request: ChatRequest,
) -> Result<(Response, ChatResponse), ErrorResponse>
where
    <M as rig::completion::CompletionModel>::StreamingResponse: Send + Sync + 'static,
{
    // 创建一个自定义的流，保持agent在同一作用域内
    let mut stream = agent.stream_prompt(&request.message).multi_turn(5).await;

    // 用于收集完整响应
    let mut final_response: Option<rig::agent::FinalResponse> = None;
    let mut stream_items_processed = 0;

    // 克隆需要在async_stream中使用的值
    let model_clone = request.model.clone();
    let conversation_id_clone = request.conversation_id.clone();

    // 创建SSE事件流
    let event_stream = async_stream::stream! {
        tracing::info!("开始处理流式响应");

        while let Some(content) = stream.next().await {
            stream_items_processed += 1;
            match content {
                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::ToolCall(tool_call))) => {
                    tracing::info!("收到工具调用: {}({})", tool_call.function.name, tool_call.function.arguments);
                    let chunk = StreamChunk::ToolCall {
                        name: tool_call.function.name.clone(),
                        arguments: serde_json::to_value(&tool_call.function).unwrap_or_default(),
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::ToolResult { id, result })) => {
                    tracing::info!("收到工具响应: {} - {}", id, result);
                    let chunk = StreamChunk::ToolResult {
                        id: id.clone(),
                        result: result.clone(),
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Text(text))) => {
                    tracing::info!("收到文本内容: {}", text.text);
                    let chunk = StreamChunk::Text {
                        text: text.text.clone(),
                        finished: false,
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Reasoning(reasoning))) => {
                    let reasoning_text = reasoning.reasoning.join("\n");
                    tracing::info!("收到推理内容: {}", reasoning_text);
                    let chunk = StreamChunk::Reasoning { reasoning: reasoning_text };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::FinalResponse(res)) => {
                    tracing::info!("收到最终响应: {}", res.response());
                    final_response = Some(res.clone());

                    let usage = res.usage();
                    let response_content = res.response().to_string();

                    let chat_response = ChatResponse {
                        content: response_content.clone(),
                        model: model_clone.clone(),
                        usage: Some(TokenUsage {
                            prompt_tokens: usage.input_tokens as u32,
                            completion_tokens: usage.output_tokens as u32,
                            total_tokens: usage.total_tokens as u32,
                        }),
                        conversation_id: conversation_id_clone.clone()
                    .unwrap_or_else(|| crate::server::models::generate_conversation_id()),
                        timestamp: chrono::Utc::now(),
                        metadata: HashMap::new(),
                    };

                    let chunk = StreamChunk::Final { response: chat_response.clone() };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));

                    tracing::info!("流式处理完成，总共处理了 {} 个流项目", stream_items_processed);
                    break;
                }

                Err(err) => {
                    tracing::warn!("流项目处理错误: {}", err);
                    let chunk = StreamChunk::Error { message: err.to_string() };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                    break;
                }

                _ => {
                    tracing::debug!("收到未匹配的流项目类型");
                }
            }
        }

        if final_response.is_none() {
            tracing::warn!("没有收到最终响应");
        }
    };

    // 创建SSE响应
    let sse_response = Sse::new(event_stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(10))
                .text("keepalive"),
        );

    tracing::info!("SSE响应已创建");

    // 创建一个基本的ChatResponse用于立即返回
    let chat_response = ChatResponse {
        content: String::new(), // 将通过SSE流填充
        model: request.model,
        usage: None,
        conversation_id: request.conversation_id.expect("REASON"),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    };

    Ok((sse_response.into_response(), chat_response))
}

// ==================== McpAgent 专用处理函数 ====================

/// 处理 McpAgent 的非流式请求
/// 
/// 这个函数专门处理使用 McpAgent 包装的 Agent，确保 MCP 客户端在整个处理过程中保持活跃
pub async fn handle_normal_request_mcp<M, C>(
    mcp_agent: McpAgent<M, C>,
    request: ChatRequest,
) -> Result<(Response, ChatResponse), ErrorResponse> 
where
    M: rig::completion::CompletionModel + Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    tracing::info!("使用 McpAgent 处理非流式请求");
    
    // 使用流式API来获取完整的使用统计，即使是非流式请求
    let mut stream = mcp_agent.inner().stream_prompt(&request.message).multi_turn(5).await;
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
                tracing::warn!("McpAgent 流式处理出错: {}", e);
                break;
            }
            _ => {}
        }
    }

    // 如果没有获取到内容，回退到简单的prompt方法
    if content.is_empty() {
        match mcp_agent.inner().prompt(&request.message).await {
            Ok(response) => {
                let chat_response = ChatResponse {
                    content: response,
                    model: request.model,
                    usage: None,
                    conversation_id: request.conversation_id.expect("conversation_id should exist"),
                    timestamp: chrono::Utc::now(),
                    metadata: HashMap::new(),
                };
                Ok((Json(chat_response.clone()).into_response(), chat_response))
            }
            Err(e) => {
                Err(ErrorResponse {
                    error: "chat_failed".to_string(),
                    message: format!("McpAgent 聊天处理失败: {}", e),
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
            conversation_id: request.conversation_id.expect("conversation_id should exist"),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        Ok((Json(chat_response.clone()).into_response(), chat_response))
    }
}

/// 处理 McpAgent 的流式请求
/// 
/// 这个函数专门处理使用 McpAgent 包装的 Agent，确保 MCP 客户端在整个 SSE 流处理期间保持活跃
pub async fn handle_streaming_request_mcp<M, C>(
    mcp_agent: McpAgent<M, C>,
    request: ChatRequest,
) -> Result<(Response, ChatResponse), ErrorResponse>
where
    M: rig::completion::CompletionModel + Send + Sync + 'static,
    C: Send + Sync + 'static,
    <M as rig::completion::CompletionModel>::StreamingResponse: Send + Sync + 'static,
{
    tracing::info!("使用 McpAgent 处理流式请求");
    
    // 创建流，注意 mcp_agent 持有 MCP 客户端的引用
    let mut stream = mcp_agent.inner().stream_prompt(&request.message).multi_turn(5).await;

    // 用于收集完整响应
    let mut final_response: Option<rig::agent::FinalResponse> = None;
    let mut stream_items_processed = 0;

    // 克隆需要在async_stream中使用的值
    let model_clone = request.model.clone();
    let conversation_id_clone = request.conversation_id.clone();

    // 将 mcp_agent 移入闭包，确保其在流处理期间存活
    // 这是关键：McpAgent 被移动到 event_stream 中，它持有 MCP 客户端的 Arc，
    // 所以在整个流处理期间 MCP 连接都会保持活跃
    let event_stream = async_stream::stream! {
        tracing::info!("McpAgent: 开始处理流式响应");
        
        // 持有 mcp_agent 的所有权，防止其被 drop
        let _mcp_keeper = mcp_agent;

        while let Some(content) = stream.next().await {
            stream_items_processed += 1;
            match content {
                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::ToolCall(tool_call))) => {
                    tracing::info!("McpAgent: 收到工具调用: {}({})", tool_call.function.name, tool_call.function.arguments);
                    let chunk = StreamChunk::ToolCall {
                        name: tool_call.function.name.clone(),
                        arguments: serde_json::to_value(&tool_call.function).unwrap_or_default(),
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::ToolResult { id, result })) => {
                    tracing::info!("McpAgent: 收到工具响应: {} - {}", id, result);
                    let chunk = StreamChunk::ToolResult {
                        id: id.clone(),
                        result: result.clone(),
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Text(text))) => {
                    tracing::info!("McpAgent: 收到文本内容: {}", text.text);
                    let chunk = StreamChunk::Text {
                        text: text.text.clone(),
                        finished: false,
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Reasoning(reasoning))) => {
                    let reasoning_text = reasoning.reasoning.join("\n");
                    tracing::info!("McpAgent: 收到推理内容: {}", reasoning_text);
                    let chunk = StreamChunk::Reasoning { reasoning: reasoning_text };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::FinalResponse(res)) => {
                    tracing::info!("McpAgent: 收到最终响应: {}", res.response());
                    final_response = Some(res.clone());

                    let usage = res.usage();
                    let response_content = res.response().to_string();

                    let chat_response = ChatResponse {
                        content: response_content.clone(),
                        model: model_clone.clone(),
                        usage: Some(TokenUsage {
                            prompt_tokens: usage.input_tokens as u32,
                            completion_tokens: usage.output_tokens as u32,
                            total_tokens: usage.total_tokens as u32,
                        }),
                        conversation_id: conversation_id_clone.clone()
                            .unwrap_or_else(|| crate::server::models::generate_conversation_id()),
                        timestamp: chrono::Utc::now(),
                        metadata: HashMap::new(),
                    };

                    let chunk = StreamChunk::Final { response: chat_response.clone() };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));

                    tracing::info!("McpAgent: 流式处理完成，总共处理了 {} 个流项目", stream_items_processed);
                    break;
                }

                Err(err) => {
                    tracing::warn!("McpAgent: 流项目处理错误: {}", err);
                    let chunk = StreamChunk::Error { message: err.to_string() };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<axum::response::sse::Event, std::convert::Infallible>(Event::default().data(event_data));
                    break;
                }

                _ => {
                    tracing::debug!("McpAgent: 收到未匹配的流项目类型");
                }
            }
        }

        if final_response.is_none() {
            tracing::warn!("McpAgent: 没有收到最终响应");
        }
        
        tracing::info!("McpAgent: event_stream 结束，MCP 客户端即将释放");
    };

    // 创建SSE响应
    let sse_response = Sse::new(event_stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(10))
                .text("keepalive"),
        );

    tracing::info!("McpAgent: SSE响应已创建");

    // 创建一个基本的ChatResponse用于立即返回
    let chat_response = ChatResponse {
        content: String::new(), // 将通过SSE流填充
        model: request.model,
        usage: None,
        conversation_id: request.conversation_id.expect("conversation_id should exist"),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    };

    Ok((sse_response.into_response(), chat_response))
}

// /// 创建简单的非流式响应（用于测试）
// fn create_simple_response(content: String, request: &ChatRequest) -> Result<(Response, ChatResponse), ErrorResponse> {
//     tracing::info!("创建简单响应，内容长度: {}", content.len());

//     let chat_response = ChatResponse {
//         content: content.clone(),
//         model: request.model.clone(),
//         usage: None,
//         conversation_id: request.conversation_id.clone()
//             .unwrap_or_else(|| crate::server::models::generate_conversation_id()),
//         timestamp: chrono::Utc::now(),
//         metadata: HashMap::new(),
//     };

//     // 创建单个JSON响应（非流式）
//     let json_response = json!({
//         "type": "final",
//         "response": chat_response
//     });

//     Ok((axum::response::Json(json_response).into_response(), chat_response))
// }