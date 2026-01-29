use std::collections::HashMap;

use axum::{
    Json,
    response::{IntoResponse, Response, Sse, sse::Event},
};
use futures_util::StreamExt;
use rig::completion::Prompt;
use rig::streaming::{StreamingChat, StreamingPrompt};
use tracing::{info, warn};

use crate::server::mcp::McpAgent;
use crate::server::models::*;

/// 统一的 Agent 包装类型，用于类型擦除 McpAgent 的泛型参数
pub enum AnyAgent<M>
where
    M: rig::completion::CompletionModel + Send + Sync + 'static,
{
    Agent(rig::agent::Agent<M>),
    McpAgent(Box<dyn AnyMcpAgent<M> + Send + Sync>),
}

/// McpAgent 的统一接口 trait
pub(crate) trait AnyMcpAgent<M: rig::completion::CompletionModel + Send + Sync + 'static>:
    Send + Sync
{
    fn inner(&self) -> &rig::agent::Agent<M>;
    fn handle_streaming(
        self: Box<Self>,
        request: ChatRequest,
        history: Option<Vec<rig::message::Message>>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(Response, ChatResponse), ErrorResponse>>
                + Send,
        >,
    >;
}

/// McpAgent 包装器
struct McpAgentWrapper<
    M: rig::completion::CompletionModel + Send + Sync + 'static,
    C: Send + Sync + 'static,
> {
    agent: McpAgent<M, C>,
}

impl<M, C> AnyMcpAgent<M> for McpAgentWrapper<M, C>
where
    M: rig::completion::CompletionModel + Send + Sync + 'static,
    C: Send + Sync + 'static,
    <M as rig::completion::CompletionModel>::StreamingResponse: Send + Sync + 'static,
{
    fn inner(&self) -> &rig::agent::Agent<M> {
        self.agent.inner()
    }

    fn handle_streaming(
        self: Box<Self>,
        request: ChatRequest,
        history: Option<Vec<rig::message::Message>>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(Response, ChatResponse), ErrorResponse>>
                + Send,
        >,
    > {
        Box::pin(async move {
            let agent_ref = self.agent.inner().clone();
            create_sse_response_for_mcp(self.agent, agent_ref, request, history).await
        })
    }
}

impl<M> AnyAgent<M>
where
    M: rig::completion::CompletionModel + Send + Sync + 'static,
{
    fn inner_agent(&self) -> &rig::agent::Agent<M> {
        match self {
            Self::Agent(agent) => agent,
            Self::McpAgent(mcp_wrapper) => mcp_wrapper.inner(),
        }
    }

    // fn is_mcp(&self) -> bool {
    //     matches!(self, Self::McpAgent(_))
    // }
}

impl<M> From<rig::agent::Agent<M>> for AnyAgent<M>
where
    M: rig::completion::CompletionModel + Send + Sync + 'static,
{
    fn from(agent: rig::agent::Agent<M>) -> Self {
        Self::Agent(agent)
    }
}

impl<M, C> From<McpAgent<M, C>> for AnyAgent<M>
where
    M: rig::completion::CompletionModel + Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    fn from(mcp_agent: McpAgent<M, C>) -> Self {
        Self::McpAgent(Box::new(McpAgentWrapper { agent: mcp_agent }))
    }
}

/// 统一的聊天请求处理函数
pub async fn handle_chat_request<M: rig::completion::CompletionModel + Send + Sync + 'static>(
    agent: impl Into<AnyAgent<M>>,
    request: ChatRequest,
    history: Option<Vec<rig::message::Message>>,
) -> Result<(Response, ChatResponse), ErrorResponse>
where
    <M as rig::completion::CompletionModel>::StreamingResponse: Send + Sync + 'static,
{
    if request.stream {
        handle_streaming_request(agent, request, history).await
    } else {
        handle_normal_request(agent, request, history).await
    }
}

/// 处理非流式请求
pub async fn handle_normal_request<M: rig::completion::CompletionModel + Send + Sync + 'static>(
    agent: impl Into<AnyAgent<M>>,
    request: ChatRequest,
    history: Option<Vec<rig::message::Message>>,
) -> Result<(Response, ChatResponse), ErrorResponse> {
    let any_agent = agent.into();
    let agent = any_agent.inner_agent();

    // 构建 PromptRequest
    let mut history_vec = history.unwrap_or_default();
    let initial_len = history_vec.len();

    let prompt_request = agent
        .prompt(&request.message)
        .with_history(&mut history_vec)
        .multi_turn(20)
        .extended_details();

    match prompt_request.await {
        Ok(prompt_response) => {
            let content = prompt_response.output;
            let usage = Some(TokenUsage {
                prompt_tokens: prompt_response.total_usage.input_tokens as u32,
                completion_tokens: prompt_response.total_usage.output_tokens as u32,
                total_tokens: prompt_response.total_usage.total_tokens as u32,
            });

            // Capture intermediate messages
            let mut metadata = HashMap::new();
            if history_vec.len() > initial_len + 1 {
                // The last message is the final assistant response (which might be what we just got)
                // The messages between initial_len + 1 and history_vec.len() - 1 are intermediate
                // Actually, history_vec.push(prompt) happened inside PromptRequest::send
                // Then history_vec.push(assistant_tool_calls)
                // Then history_vec.push(user_tool_results)
                // ...
                // The last assistant message added to history_vec is the one that contains the final text.

                // Let's just take all messages from initial_len onwards, excluding the very last one
                // (which is the final assistant response that we are already returning as ChatResponse)
                let intermediate = &history_vec[initial_len..history_vec.len() - 1];
                if !intermediate.is_empty() {
                    if let Ok(json_msgs) = serde_json::to_value(intermediate) {
                        metadata.insert("_intermediate_messages".to_string(), json_msgs);
                    }
                }
            }

            let chat_response = ChatResponse {
                content: if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                reasoning_content: None,
                tool_calls: None,
                model: request.model,
                usage,
                conversation_id: request
                    .conversation_id
                    .expect("conversation_id should exist"),
                timestamp: chrono::Local::now(),
                metadata,
            };
            Ok((Json(chat_response.clone()).into_response(), chat_response))
        }
        Err(e) => Err(ErrorResponse {
            error: "chat_failed".to_string(),
            message: format!("聊天处理失败: {}", e),
            details: None,
            timestamp: chrono::Local::now(),
        }),
    }
}

/// 处理流式请求
pub async fn handle_streaming_request<M: rig::completion::CompletionModel + Send + Sync + 'static>(
    agent: impl Into<AnyAgent<M>>,
    request: ChatRequest,
    history: Option<Vec<rig::message::Message>>,
) -> Result<(Response, ChatResponse), ErrorResponse>
where
    <M as rig::completion::CompletionModel>::StreamingResponse: Send + Sync + 'static,
{
    let any_agent = agent.into();

    match any_agent {
        AnyAgent::Agent(agent) => create_sse_response_for_agent(agent, request, history).await,
        AnyAgent::McpAgent(mcp_wrapper) => mcp_wrapper.handle_streaming(request, history).await,
    }
}

/// 为普通 Agent 创建 SSE 响应
async fn create_sse_response_for_agent<
    M: rig::completion::CompletionModel + Send + Sync + 'static,
>(
    agent: rig::agent::Agent<M>,
    request: ChatRequest,
    history: Option<Vec<rig::message::Message>>,
) -> Result<(Response, ChatResponse), ErrorResponse>
where
    <M as rig::completion::CompletionModel>::StreamingResponse: Send + Sync + 'static,
{
    let mut stream = if let Some(history) = history {
        agent
            .stream_chat(&request.message, history)
            .multi_turn(20)
            .await
    } else {
        agent.stream_prompt(&request.message).multi_turn(20).await
    };
    let mut final_response: Option<rig::agent::FinalResponse> = None;
    let mut collected_content = String::new();
    let mut stream_items_processed = 0;

    let model = request.model.clone();
    let conversation_id = request.conversation_id.clone();

    let event_stream = async_stream::stream! {
        info!("开始处理流式响应");

        while let Some(item) = stream.next().await {
            stream_items_processed += 1;
            match item {
                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::ToolCall(tool_call))) => {
                    info!("McpAgent: 收到工具调用: {}: {}({})",
                        tool_call.id, tool_call.function.name, tool_call.function.arguments);

                    let chunk = StreamChunk::ToolCall {
                        id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        arguments: serde_json::to_value(&tool_call.function).unwrap_or_default(),
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::ToolResult { id, result })) => {
                    info!("McpAgent: 收到工具响应: {} - {}", id, result.chars().take(20).collect::<String>());

                    // 尝试将结果字符串解析为 JSON，避免双重转义
                    let result_value = serde_json::from_str(&result).unwrap_or_else(|_| serde_json::Value::String(result.clone()));

                    let chunk = StreamChunk::ToolResult {
                        id: id.clone(),
                        result: result_value,
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Text(text))) => {
                    collected_content.push_str(&text.text);

                    let chunk = StreamChunk::Text {
                        text: text.text.clone(),
                        finished: false,
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Reasoning(reasoning))) => {
                    let reasoning_text = reasoning.reasoning.join("\n");

                    let chunk = StreamChunk::Reasoning { reasoning: reasoning_text };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::FinalResponse(res)) => {
                    final_response = Some(res.clone());
                    collected_content = res.response().to_string();

                    let usage = res.usage();
                    let chat_response = ChatResponse {
                        content: if collected_content.is_empty() { None } else { Some(collected_content.clone()) },
                        reasoning_content: None,
                        tool_calls: None,
                        model: model.clone(),
                        usage: Some(TokenUsage {
                            prompt_tokens: usage.input_tokens as u32,
                            completion_tokens: usage.output_tokens as u32,
                            total_tokens: usage.total_tokens as u32,
                        }),
                        conversation_id: conversation_id.clone()
                            .unwrap_or_else(crate::server::models::generate_conversation_id),
                        timestamp: chrono::Local::now(),
                        metadata: HashMap::new(),
                    };

                    let chunk = StreamChunk::Final { response: chat_response.clone() };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));

                    info!("流式处理完成，总共处理了 {} 个流项目", stream_items_processed);
                    break;
                }

                Err(err) => {
                    warn!("流项目处理错误: {}", err);
                    let chunk = StreamChunk::Error { message: err.to_string() };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                    break;
                }

                _ => {}
            }
        }

        if final_response.is_none() {
            warn!("没有收到最终响应");
        }
    };

    let sse_response = Sse::new(event_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(10))
            .text("keepalive"),
    );

    info!("SSE响应已创建");

    let chat_response = ChatResponse {
        content: None, // 将通过SSE流填充
        reasoning_content: None,
        tool_calls: None,
        model: request.model,
        usage: None,
        conversation_id: request
            .conversation_id
            .expect("conversation_id should exist"),
        timestamp: chrono::Local::now(),
        metadata: HashMap::new(),
    };

    Ok((sse_response.into_response(), chat_response))
}

/// 为 McpAgent 创建 SSE 响应
async fn create_sse_response_for_mcp<M, C>(
    mcp_agent: McpAgent<M, C>,
    agent: rig::agent::Agent<M>,
    request: ChatRequest,
    history: Option<Vec<rig::message::Message>>,
) -> Result<(Response, ChatResponse), ErrorResponse>
where
    M: rig::completion::CompletionModel + Send + Sync + 'static,
    C: Send + Sync + 'static,
    <M as rig::completion::CompletionModel>::StreamingResponse: Send + Sync + 'static,
{
    let mut stream = if let Some(history) = history {
        agent
            .stream_chat(&request.message, history)
            .multi_turn(20)
            .await
    } else {
        agent.stream_prompt(&request.message).multi_turn(20).await
    };
    let mut final_response: Option<rig::agent::FinalResponse> = None;
    let mut collected_content = String::new();
    let mut stream_items_processed = 0;

    let model = request.model.clone();
    let conversation_id = request.conversation_id.clone();

    let event_stream = async_stream::stream! {
        info!("McpAgent: 开始处理流式响应");

        // 保持 mcp_agent 存活，防止 MCP 连接被释放
        let _mcp_keeper = mcp_agent;

        while let Some(item) = stream.next().await {
            stream_items_processed += 1;
            match item {
                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::ToolCall(tool_call))) => {
                    info!("McpAgent: 收到工具调用: {}: {}({})",
                        tool_call.id, tool_call.function.name, tool_call.function.arguments);

                    let chunk = StreamChunk::ToolCall {
                        id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        arguments: serde_json::to_value(&tool_call.function).unwrap_or_default(),
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::ToolResult { id, result })) => {
                    info!("McpAgent: 收到工具响应: {} - {}", id, result);

                    // 尝试将结果字符串解析为 JSON，避免双重转义
                    let result_value = serde_json::from_str(&result).unwrap_or_else(|_| serde_json::Value::String(result.clone()));

                    let chunk = StreamChunk::ToolResult {
                        id: id.clone(),
                        result: result_value,
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Text(text))) => {
                    collected_content.push_str(&text.text);

                    let chunk = StreamChunk::Text {
                        text: text.text.clone(),
                        finished: false,
                    };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::StreamItem(rig::streaming::StreamedAssistantContent::Reasoning(reasoning))) => {
                    let reasoning_text = reasoning.reasoning.join("\n");

                    let chunk = StreamChunk::Reasoning { reasoning: reasoning_text };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                }

                Ok(rig::agent::MultiTurnStreamItem::FinalResponse(res)) => {
                    info!("McpAgent: 收到最终响应");
                    final_response = Some(res.clone());
                    collected_content = res.response().to_string();

                    let usage = res.usage();
                    let chat_response = ChatResponse {
                        content: if collected_content.is_empty() { None } else { Some(collected_content.clone()) },
                        reasoning_content: None,
                        tool_calls: None,
                        model: model.clone(),
                        usage: Some(TokenUsage {
                            prompt_tokens: usage.input_tokens as u32,
                            completion_tokens: usage.output_tokens as u32,
                            total_tokens: usage.total_tokens as u32,
                        }),
                        conversation_id: conversation_id.clone()
                            .unwrap_or_else(crate::server::models::generate_conversation_id),
                        timestamp: chrono::Local::now(),
                        metadata: HashMap::new(),
                    };

                    let chunk = StreamChunk::Final { response: chat_response.clone() };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));

                    info!("McpAgent: 流式处理完成，总共处理了 {} 个流项目", stream_items_processed);
                    break;
                }

                Err(err) => {
                    warn!("McpAgent: 流项目处理错误: {}", err);
                    let chunk = StreamChunk::Error { message: err.to_string() };
                    let event_data = serde_json::to_string(&chunk).unwrap_or_default();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event_data));
                    break;
                }

                _ => {}
            }
        }

        if final_response.is_none() {
            warn!("McpAgent: 没有收到最终响应");
        }
    };

    let sse_response = Sse::new(event_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(10))
            .text("keepalive"),
    );

    info!("McpAgent: SSE响应已创建");

    let chat_response = ChatResponse {
        content: None, // 将通过SSE流填充
        reasoning_content: None,
        tool_calls: None,
        model: request.model,
        usage: None,
        conversation_id: request
            .conversation_id
            .expect("conversation_id should exist"),
        timestamp: chrono::Local::now(),
        metadata: HashMap::new(),
    };

    Ok((sse_response.into_response(), chat_response))
}
