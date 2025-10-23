// 导入一或多个容器和完成模型相关类型
use crate::{
    OneOrMany,
    completion::GetTokenUsage,
    message::{AssistantContent, Reasoning, ToolResultContent, UserContent},
    streaming::{StreamedAssistantContent, StreamingCompletion},
};
// 导入流处理相关类型
use futures::{Stream, StreamExt};
// 导入序列化和反序列化 trait
use serde::{Deserialize, Serialize};
// 导入 Pin 和原子引用计数相关类型
use std::{pin::Pin, sync::Arc};
// 导入 tokio 读写锁
use tokio::sync::RwLock;
// 导入追踪信息跨度
use tracing::info_span;
// 导入追踪 Future 扩展
use tracing_futures::Instrument;

// 导入代理和完成模型相关类型
use crate::{
    agent::Agent,
    completion::{CompletionError, CompletionModel, PromptError},
    message::{Message, Text},
    tool::ToolSetError,
};

// 非 WASM32 架构的流式结果类型别名
#[cfg(not(target_arch = "wasm32"))]
// 定义流式结果类型，包含发送能力
pub type StreamingResult<R> =
    // 盒装的流，包含多轮流项目结果或流式错误，支持发送
    Pin<Box<dyn Stream<Item = Result<MultiTurnStreamItem<R>, StreamingError>> + Send>>;

// WASM32 架构的流式结果类型别名
#[cfg(target_arch = "wasm32")]
// 定义流式结果类型，不包含发送能力（WASM 限制）
pub type StreamingResult<R> =
    // 盒装的流，包含多轮流项目结果或流式错误
    Pin<Box<dyn Stream<Item = Result<MultiTurnStreamItem<R>, StreamingError>>>>;

// 派生反序列化、序列化、调试和克隆 trait
#[derive(Deserialize, Serialize, Debug, Clone)]
// 设置序列化标签和字段重命名规则
#[serde(tag = "type", rename_all = "camelCase")]
// 标记为非穷尽枚举，表示未来可能添加更多变体
#[non_exhaustive]
// 定义多轮流项目枚举，支持泛型响应类型
pub enum MultiTurnStreamItem<R> {
    // 流项目变体，包含流式助手内容
    StreamItem(StreamedAssistantContent<R>),
    // 最终响应变体，包含最终响应
    FinalResponse(FinalResponse),
}

// 派生反序列化、序列化、调试和克隆 trait
#[derive(Deserialize, Serialize, Debug, Clone)]
// 设置字段重命名规则为驼峰命名
#[serde(rename_all = "camelCase")]
// 定义最终响应结构体
pub struct FinalResponse {
    // 响应字符串
    response: String,
    // 聚合的使用情况
    aggregated_usage: crate::completion::Usage,
}

// 为最终响应实现方法
impl FinalResponse {
    // 创建空的最终响应的公共方法
    pub fn empty() -> Self {
        // 返回新的最终响应实例
        Self {
            // 初始化响应为空字符串
            response: String::new(),
            // 初始化聚合使用情况为新实例
            aggregated_usage: crate::completion::Usage::new(),
        }
    }

    // 获取响应字符串的公共方法
    pub fn response(&self) -> &str {
        // 返回响应字符串的引用
        &self.response
    }

    // 获取使用情况的公共方法
    pub fn usage(&self) -> crate::completion::Usage {
        // 返回聚合使用情况的副本
        self.aggregated_usage
    }
}

// 为多轮流项目实现方法，支持泛型响应类型
impl<R> MultiTurnStreamItem<R> {
    // 创建流项目的内部方法
    pub(crate) fn stream_item(item: StreamedAssistantContent<R>) -> Self {
        // 返回流项目变体
        Self::StreamItem(item)
    }

    // 创建最终响应的公共方法
    pub fn final_response(response: &str, aggregated_usage: crate::completion::Usage) -> Self {
        // 返回最终响应变体
        Self::FinalResponse(FinalResponse {
            // 将响应字符串转换为 String
            response: response.to_string(),
            // 设置聚合使用情况
            aggregated_usage,
        })
    }
}

// 派生调试和错误 trait
#[derive(Debug, thiserror::Error)]
// 定义流式错误枚举
pub enum StreamingError {
    // 完成错误变体，自动从 CompletionError 转换
    #[error("CompletionError: {0}")]
    Completion(#[from] CompletionError),
    // 提示错误变体，自动从盒装 PromptError 转换
    #[error("PromptError: {0}")]
    Prompt(#[from] Box<PromptError>),
    // 工具集错误变体，自动从 ToolSetError 转换
    #[error("ToolSetError: {0}")]
    Tool(#[from] ToolSetError),
}

/// 用于创建具有可自定义选项的提示请求的构建器。
/// 使用泛型来跟踪构建过程中设置了哪些选项。
///
/// 如果您期望连续调用工具，您需要确保使用 `.multi_turn()`
/// 参数来添加更多轮次，因为默认情况下它是 0（意味着只有 1 个工具往返）。否则，
/// 尝试 await（这将发送提示请求）可能会返回
/// [`crate::completion::request::PromptError::MaxDepthError`] 如果代理决定连续调用工具。
// 定义流式提示请求结构体，支持泛型模型类型和钩子类型
pub struct StreamingPromptRequest<M, P>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // P 必须实现 StreamingPromptHook trait 并且具有静态生命周期
    P: StreamingPromptHook<M> + 'static,
{
    /// The prompt message to send to the model
    // 要发送给模型的提示消息
    prompt: Message,
    /// Optional chat history to include with the prompt
    /// Note: chat history needs to outlive the agent as it might be used with other agents
    // 包含在提示中的可选聊天历史
    // 注意：聊天历史需要比代理存活更长时间，因为它可能与其他代理一起使用
    chat_history: Option<Vec<Message>>,
    /// Maximum depth for multi-turn conversations (0 means no multi-turn)
    // 多轮对话的最大深度（0 表示无多轮）
    max_depth: usize,
    /// The agent to use for execution
    // 用于执行的代理
    agent: Arc<Agent<M>>,
    /// Optional per-request hook for events
    // 事件的可选每请求钩子
    hook: Option<P>,
}

// 为流式提示请求实现方法，支持泛型模型类型和钩子类型
impl<M, P> StreamingPromptRequest<M, P>
where
    // M 必须实现 CompletionModel trait 并且具有静态生命周期
    M: CompletionModel + 'static,
    // 模型的流式响应必须支持发送和获取 token 使用情况
    <M as CompletionModel>::StreamingResponse: Send + GetTokenUsage,
    // P 必须实现 StreamingPromptHook trait
    P: StreamingPromptHook<M>,
{
    /// Create a new PromptRequest with the given prompt and model
    // 使用给定的提示和模型创建新的提示请求
    pub fn new(agent: Arc<Agent<M>>, prompt: impl Into<Message>) -> Self {
        // 返回新的流式提示请求实例
        Self {
            // 将提示转换为消息
            prompt: prompt.into(),
            // 初始化聊天历史为 None
            chat_history: None,
            // 初始化最大深度为 0
            max_depth: 0,
            // 设置代理
            agent,
            // 初始化钩子为 None
            hook: None,
        }
    }

    /// Set the maximum depth for multi-turn conversations (ie, the maximum number of turns an LLM can have calling tools before writing a text response).
    /// If the maximum turn number is exceeded, it will return a [`crate::completion::request::PromptError::MaxDepthError`].
    // 设置多轮对话的最大深度（即 LLM 在写入文本响应之前可以调用工具的最大轮数）
    // 如果超过最大轮数，将返回 [`crate::completion::request::PromptError::MaxDepthError`]
    pub fn multi_turn(mut self, depth: usize) -> Self {
        // 设置最大深度
        self.max_depth = depth;
        // 返回修改后的实例
        self
    }

    /// Add chat history to the prompt request
    // 为提示请求添加聊天历史
    pub fn with_history(mut self, history: Vec<Message>) -> Self {
        // 设置聊天历史
        self.chat_history = Some(history);
        // 返回修改后的实例
        self
    }

    /// Attach a per-request hook for tool call events
    // 为工具调用事件附加每请求钩子
    pub fn with_hook<P2>(self, hook: P2) -> StreamingPromptRequest<M, P2>
    where
        // P2 必须实现 StreamingPromptHook trait
        P2: StreamingPromptHook<M>,
    {
        // 返回带新钩子的流式提示请求实例
        StreamingPromptRequest {
            // 复制提示消息
            prompt: self.prompt,
            // 复制聊天历史
            chat_history: self.chat_history,
            // 复制最大深度
            max_depth: self.max_depth,
            // 复制代理引用
            agent: self.agent,
            // 设置新的钩子
            hook: Some(hook),
        }
    }

    // 为 worker 功能添加属性
    #[cfg_attr(feature = "worker", worker::send)]
    // 异步发送方法，返回流式结果
    async fn send(self) -> StreamingResult<M::StreamingResponse> {
        // 创建代理跨度，如果当前跨度被禁用则创建新的，否则使用当前跨度
        let agent_span = if tracing::Span::current().is_disabled() {
            // 创建新的信息跨度
            info_span!(
                // 跨度的名称
                "invoke_agent",
                // 操作名称
                gen_ai.operation.name = "invoke_agent",
                // 代理名称
                gen_ai.agent.name = self.agent.name(),
                // 系统指令
                gen_ai.system_instructions = self.agent.preamble,
                // 提示（空字段）
                gen_ai.prompt = tracing::field::Empty,
                // 完成（空字段）
                gen_ai.completion = tracing::field::Empty,
                // 输入 token 使用情况（空字段）
                gen_ai.usage.input_tokens = tracing::field::Empty,
                // 输出 token 使用情况（空字段）
                gen_ai.usage.output_tokens = tracing::field::Empty,
            )
        } else {
            // 使用当前跨度
            tracing::Span::current()
        };

        // 获取提示消息
        let prompt = self.prompt;
        // 如果提示有 RAG 文本，则记录到跨度中
        if let Some(text) = prompt.rag_text() {
            // 记录提示文本到跨度
            agent_span.record("gen_ai.prompt", text);
        }

        // 获取代理引用
        let agent = self.agent;

        // 设置聊天历史，如果有历史则使用，否则创建新的向量
        let chat_history = if let Some(history) = self.chat_history {
            // 使用读写锁包装历史
            Arc::new(RwLock::new(history))
        } else {
            // 创建新的空向量并用读写锁包装
            Arc::new(RwLock::new(vec![]))
        };

        // 初始化当前最大深度为 0
        let mut current_max_depth = 0;
        // 初始化最后一个提示错误为空字符串
        let mut last_prompt_error = String::new();

        // 初始化最后一个文本响应为空字符串
        let mut last_text_response = String::new();
        // 初始化是否为文本响应为 false
        let mut is_text_response = false;
        // 初始化是否达到最大深度为 false
        let mut max_depth_reached = false;

        // 初始化聚合使用情况
        let mut aggregated_usage = crate::completion::Usage::new();

        // 创建盒装的异步流生成器
        Box::pin(async_stream::stream! {
            // 进入代理跨度
            let _guard = agent_span.enter();
            // 克隆当前提示
            let mut current_prompt = prompt.clone();
            // 初始化是否调用工具为 false
            let mut did_call_tool = false;

            // 开始外层循环
            'outer: loop {
                // 如果当前最大深度超过最大深度加 1，则跳出循环
                if current_max_depth > self.max_depth + 1 {
                    // 设置最后一个提示错误为当前提示的 RAG 文本
                    last_prompt_error = current_prompt.rag_text().unwrap_or_default();
                    // 标记已达到最大深度
                    max_depth_reached = true;
                    // 跳出外层循环
                    break;
                }

                // 增加当前最大深度
                current_max_depth += 1;

                // 如果最大深度大于 1，则记录当前对话深度
                if self.max_depth > 1 {
                    // 记录当前对话深度信息
                    tracing::info!(
                        "Current conversation depth: {}/{}",
                        current_max_depth,
                        self.max_depth
                    );
                }

                // 如果有钩子，则在完成调用前执行钩子
                if let Some(ref hook) = self.hook {
                    // 获取聊天历史的读取锁
                    let reader = chat_history.read().await;
                    // 获取聊天历史中的最后一个提示
                    let prompt = reader.last().cloned().expect("there should always be at least one message in the chat history");
                    // 获取除最后一个消息外的聊天历史
                    let chat_history_except_last = reader[..reader.len() - 1].to_vec();

                    // 调用完成调用前的钩子
                    hook.on_completion_call(&prompt, &chat_history_except_last)
                    .await;
                }

                // 创建聊天流信息跨度
                let chat_stream_span = info_span!(
                    // 设置目标
                    target: "rig::agent_chat",
                    // 设置父跨度
                    parent: tracing::Span::current(),
                    // 跨度名称
                    "chat_streaming",
                    // 操作名称
                    gen_ai.operation.name = "chat",
                    // 系统指令
                    gen_ai.system_instructions = &agent.preamble,
                    // 提供商名称（空字段）
                    gen_ai.provider.name = tracing::field::Empty,
                    // 请求模型（空字段）
                    gen_ai.request.model = tracing::field::Empty,
                    // 响应 ID（空字段）
                    gen_ai.response.id = tracing::field::Empty,
                    // 响应模型（空字段）
                    gen_ai.response.model = tracing::field::Empty,
                    // 输出 token 使用情况（空字段）
                    gen_ai.usage.output_tokens = tracing::field::Empty,
                    // 输入 token 使用情况（空字段）
                    gen_ai.usage.input_tokens = tracing::field::Empty,
                    // 输入消息（空字段）
                    gen_ai.input.messages = tracing::field::Empty,
                    // 输出消息（空字段）
                    gen_ai.output.messages = tracing::field::Empty,
                );

                // 创建带检测的流
                let mut stream = tracing::Instrument::instrument(
                    // 调用代理的流式完成方法
                    agent
                    .stream_completion(current_prompt.clone(), (*chat_history.read().await).clone())
                    .await?
                    .stream(), chat_stream_span
                )

                .await?;

                // 将当前提示添加到聊天历史中
                chat_history.write().await.push(current_prompt.clone());

                // 初始化工具调用向量
                let mut tool_calls = vec![];
                // 初始化工具结果向量
                let mut tool_results = vec![];

                // 处理流中的每个内容项
                while let Some(content) = stream.next().await {
                    // 匹配内容类型
                    match content {
                        // 如果是文本内容
                        Ok(StreamedAssistantContent::Text(text)) => {
                            // 如果不是文本响应，则重置文本响应
                            if !is_text_response {
                                // 重置最后一个文本响应为空字符串
                                last_text_response = String::new();
                                // 标记为文本响应
                                is_text_response = true;
                            }
                            // 将文本追加到最后一个文本响应
                            last_text_response.push_str(&text.text);
                            // 如果有钩子，则调用文本增量钩子
                            if let Some(ref hook) = self.hook {
                                // 调用文本增量钩子
                                hook.on_text_delta(&text.text, &last_text_response).await;
                            }
                            // 产生流项目
                            yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::Text(text)));
                            // 标记未调用工具
                            did_call_tool = false;
                        },
                        // 如果是工具调用内容
                        Ok(StreamedAssistantContent::ToolCall(tool_call)) => {
                            // 创建工具执行信息跨度
                            let tool_span = info_span!(
                                // 设置父跨度
                                parent: tracing::Span::current(),
                                // 跨度名称
                                "execute_tool",
                                // 操作名称
                                gen_ai.operation.name = "execute_tool",
                                // 工具类型
                                gen_ai.tool.type = "function",
                                // 工具名称（空字段）
                                gen_ai.tool.name = tracing::field::Empty,
                                // 工具调用 ID（空字段）
                                gen_ai.tool.call.id = tracing::field::Empty,
                                // 工具调用参数（空字段）
                                gen_ai.tool.call.arguments = tracing::field::Empty,
                                // 工具调用结果（空字段）
                                gen_ai.tool.call.result = tracing::field::Empty
                            );

                            // 异步执行工具
                            async {
                                // 获取当前跨度
                                let tool_span = tracing::Span::current();
                                // 如果有钩子，则调用工具调用前的钩子
                                if let Some(ref hook) = self.hook {
                                    // 调用工具调用前的钩子
                                    hook.on_tool_call(&tool_call.function.name, &tool_call.function.arguments.to_string()).await;
                                }

                                // 记录工具名称到跨度
                                tool_span.record("gen_ai.tool.name", &tool_call.function.name);
                                // 记录工具调用参数到跨度
                                tool_span.record("gen_ai.tool.call.arguments", tool_call.function.arguments.to_string());

                                // 调用工具并获取结果
                                let tool_result = match
                                agent.tools.call(&tool_call.function.name, tool_call.function.arguments.to_string()).await {
                                    // 如果成功，返回结果
                                    Ok(thing) => thing,
                                    // 如果失败，返回错误字符串
                                    Err(e) => e.to_string()
                                };

                                // 记录工具调用结果到跨度
                                tool_span.record("gen_ai.tool.call.result", &tool_result);

                                // 如果有钩子，则调用工具调用后的钩子
                                if let Some(ref hook) = self.hook {
                                    // 调用工具调用后的钩子
                                    hook.on_tool_result(&tool_call.function.name, &tool_call.function.arguments.to_string(), &tool_result.to_string())
                                    .await;
                                }

                                // 创建工具调用消息
                                let tool_call_msg = AssistantContent::ToolCall(tool_call.clone());

                                // 将工具调用添加到工具调用列表
                                tool_calls.push(tool_call_msg);
                                // 将工具结果添加到工具结果列表
                                tool_results.push((tool_call.id, tool_call.call_id, tool_result));

                                // 标记已调用工具
                                did_call_tool = true;
                                // break;
                            }.instrument(tool_span).await
                        },
                        // 如果是推理内容
                        Ok(StreamedAssistantContent::Reasoning(rig::message::Reasoning { reasoning, id })) => {
                            // 将推理内容添加到聊天历史中
                            chat_history.write().await.push(rig::message::Message::Assistant {
                                // 设置 ID 为 None
                                id: None,
                                // 设置内容为推理内容
                                content: OneOrMany::one(AssistantContent::Reasoning(Reasoning {
                                    reasoning: reasoning.clone(), id: id.clone()
                                }))
                            });
                            // 产生推理流项目
                            yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::Reasoning(rig::message::Reasoning { reasoning, id })));
                            // 标记未调用工具
                            did_call_tool = false;
                        },
                        // 如果是最终响应内容
                        Ok(StreamedAssistantContent::Final(final_resp)) => {
                            // 如果有 token 使用情况，则累加到聚合使用情况
                            if let Some(usage) = final_resp.token_usage() { aggregated_usage += usage; };
                            // 如果是文本响应
                            if is_text_response {
                                // 如果有钩子，则调用流完成响应结束钩子
                                if let Some(ref hook) = self.hook {
                                    // 调用流完成响应结束钩子
                                    hook.on_stream_completion_response_finish(&prompt, &final_resp).await;
                                }
                                // 记录完成信息到当前跨度
                                tracing::Span::current().record("gen_ai.completion", &last_text_response);
                                // 产生最终响应流项目
                                yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::Final(final_resp)));
                                // 标记不是文本响应
                                is_text_response = false;
                            }
                        }
                        // 如果是错误
                        Err(e) => {
                            // 产生错误并跳出外层循环
                            yield Err(e.into());
                            break 'outer;
                        }
                    }
                }

                // Add (parallel) tool calls to chat history
                // 将（并行）工具调用添加到聊天历史中
                if !tool_calls.is_empty() {
                    // 将工具调用添加到聊天历史中
                    chat_history.write().await.push(Message::Assistant {
                        // 设置 ID 为 None
                        id: None,
                        // 设置内容为多个工具调用，期望不会出现空列表错误
                        content: OneOrMany::many(tool_calls.clone()).expect("Impossible EmptyListError"),
                    });
                }

                // Add tool results to chat history
                // 将工具结果添加到聊天历史中
                for (id, call_id, tool_result) in tool_results {
                    // 如果有调用 ID
                    if let Some(call_id) = call_id {
                        // 添加带调用 ID 的工具结果到聊天历史
                        chat_history.write().await.push(Message::User {
                            content: OneOrMany::one(UserContent::tool_result_with_call_id(
                                &id,
                                call_id.clone(),
                                OneOrMany::one(ToolResultContent::text(&tool_result)),
                            )),
                        });
                    } else {
                        // 添加不带调用 ID 的工具结果到聊天历史
                        chat_history.write().await.push(Message::User {
                            content: OneOrMany::one(UserContent::tool_result(
                                &id,
                                OneOrMany::one(ToolResultContent::text(&tool_result)),
                            )),
                        });
                    }
                }

                // Set the current prompt to the last message in the chat history
                // 将当前提示设置为聊天历史中的最后一条消息
                current_prompt = match chat_history.write().await.pop() {
                    // 如果有提示，则使用它
                    Some(prompt) => prompt,
                    // 这种情况不应该发生，聊天历史在此点不应该为空
                    None => unreachable!("Chat history should never be empty at this point"),
                };

                // 如果没有调用工具
                if !did_call_tool {
                    // 获取当前跨度
                    let current_span = tracing::Span::current();
                    // 记录输入 token 使用情况
                    current_span.record("gen_ai.usage.input_tokens", aggregated_usage.input_tokens);
                    // 记录输出 token 使用情况
                    current_span.record("gen_ai.usage.output_tokens", aggregated_usage.output_tokens);
                    // 记录代理多轮流完成信息
                    tracing::info!("Agent multi-turn stream finished");
                    // 产生最终响应
                    yield Ok(MultiTurnStreamItem::final_response(&last_text_response, aggregated_usage));
                    // 跳出循环
                    break;
                }
            }

            // 如果达到最大深度
            if max_depth_reached {
                // 产生最大深度错误
                yield Err(Box::new(PromptError::MaxDepthError {
                    // 最大深度
                    max_depth: self.max_depth,
                    // 聊天历史（装箱）
                    chat_history: Box::new((*chat_history.read().await).clone()),
                    // 最后一个提示错误
                    prompt: last_prompt_error.clone().into(),
                }).into());
            }

        })
    }
}

// 为流式提示请求实现 IntoFuture trait
impl<M, P> IntoFuture for StreamingPromptRequest<M, P>
where
    // M 必须实现 CompletionModel trait 并且具有静态生命周期
    M: CompletionModel + 'static,
    // 模型的流式响应必须支持发送
    <M as CompletionModel>::StreamingResponse: Send,
    // P 必须实现 StreamingPromptHook trait 并且具有静态生命周期
    P: StreamingPromptHook<M> + 'static,
{
    // 定义输出类型为流式结果（`.await` 返回的内容）
    type Output = StreamingResult<M::StreamingResponse>; // what `.await` returns
    // 定义 IntoFuture 类型为盒装 Future
    type IntoFuture = Pin<Box<dyn futures::Future<Output = Self::Output> + Send>>;

    // 将自身转换为 Future
    fn into_future(self) -> Self::IntoFuture {
        // Wrap send() in a future, because send() returns a stream immediately
        // 将 send() 包装在 Future 中，因为 send() 立即返回一个流
        Box::pin(async move { self.send().await })
    }
}

/// helper function to stream a completion selfuest to stdout
// 将完成请求流式传输到标准输出的辅助函数
pub async fn stream_to_stdout<R>(
    // 流式结果的引用
    stream: &mut StreamingResult<R>,
) -> Result<FinalResponse, std::io::Error> {
    // 初始化最终响应为空
    let mut final_res = FinalResponse::empty();
    // 打印响应前缀
    print!("Response: ");
    // 处理流中的每个内容项
    while let Some(content) = stream.next().await {
        // 匹配内容类型
        match content {
            // 如果是文本流项目
            Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::Text(Text { text }))) => {
                // 打印文本内容
                print!("{text}");
                // 刷新标准输出
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
            // 如果是推理流项目
            Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::Reasoning(
                Reasoning { reasoning, .. },
            ))) => {
                // 将推理内容用换行符连接
                let reasoning = reasoning.join("\n");
                // 打印推理内容
                print!("{reasoning}");
                // 刷新标准输出
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
            // 如果是最终响应
            Ok(MultiTurnStreamItem::FinalResponse(res)) => {
                // 设置最终响应
                final_res = res;
            }
            // 如果是错误
            Err(err) => {
                // 打印错误到标准错误
                eprintln!("Error: {err}");
            }
            // 其他情况忽略
            _ => {}
        }
    }

    // 返回最终响应
    Ok(final_res)
}

// dead code allowed because of functions being left empty to allow for users to not have to implement every single function
/// Trait for per-request hooks to observe tool call events.
// 允许死代码，因为函数留空以允许用户不必实现每个函数
// 用于观察工具调用事件的每请求钩子的 trait
pub trait StreamingPromptHook<M>: Clone + Send + Sync
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    // 允许未使用的变量
    #[allow(unused_variables)]
    /// Called before the prompt is sent to the model
    // 在提示发送给模型之前调用
    fn on_completion_call(
        // 接收自身引用
        &self,
        // 提示消息的引用
        prompt: &Message,
        // 聊天历史的引用
        history: &[Message],
    ) -> impl Future<Output = ()> + Send {
        // 返回空的异步块
        async {}
    }

    // 允许未使用的变量
    #[allow(unused_variables)]
    /// Called when receiving a text delta
    // 在接收文本增量时调用
    fn on_text_delta(
        // 接收自身引用
        &self,
        // 文本增量
        text_delta: &str,
        // 聚合文本
        aggregated_text: &str,
    ) -> impl Future<Output = ()> + Send {
        // 返回空的异步块
        async {}
    }

    // 允许未使用的变量
    #[allow(unused_variables)]
    /// Called after the model provider has finished streaming a text response from their completion API to the client.
    // 在模型提供商完成从其完成 API 向客户端流式传输文本响应后调用
    fn on_stream_completion_response_finish(
        // 接收自身引用
        &self,
        // 提示消息的引用
        prompt: &Message,
        // 流式响应的引用
        response: &<M as CompletionModel>::StreamingResponse,
    ) -> impl Future<Output = ()> + Send {
        // 返回空的异步块
        async {}
    }

    // 允许未使用的变量
    #[allow(unused_variables)]
    /// Called before a tool is invoked.
    // 在调用工具之前调用
    fn on_tool_call(&self, tool_name: &str, args: &str) -> impl Future<Output = ()> + Send {
        // 返回空的异步块
        async {}
    }

    // 允许未使用的变量
    #[allow(unused_variables)]
    /// Called after a tool is invoked (and a result has been returned).
    // 在调用工具后（并返回结果）调用
    fn on_tool_result(
        // 接收自身引用
        &self,
        // 工具名称
        tool_name: &str,
        // 工具参数
        args: &str,
        // 工具结果
        result: &str,
    ) -> impl Future<Output = ()> + Send {
        // 返回空的异步块
        async {}
    }
}

// 为单元类型实现 StreamingPromptHook trait
impl<M> StreamingPromptHook<M> for () where M: CompletionModel {}
