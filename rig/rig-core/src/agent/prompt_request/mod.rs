// 定义流式处理模块（仅内部可见）
pub(crate) mod streaming;

// 导入标准库相关类型
use std::{
    future::IntoFuture,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};
// 导入追踪相关类型
use tracing::{Instrument, span::Id};

// 导入异步处理相关类型
use futures::{FutureExt, StreamExt, future::BoxFuture, stream};
// 导入追踪信息跨度
use tracing::info_span;

// 导入完成模型和消息相关类型
use crate::{
    OneOrMany,
    completion::{Completion, CompletionError, CompletionModel, Message, PromptError, Usage},
    message::{AssistantContent, UserContent},
    tool::ToolSetError,
};

// 导入父模块中的代理类型
use super::Agent;

// 定义提示类型 trait
pub trait PromptType {}
// 定义标准提示类型结构体
pub struct Standard;
// 定义扩展提示类型结构体
pub struct Extended;

// 为标准类型实现提示类型 trait
impl PromptType for Standard {}
// 为扩展类型实现提示类型 trait
impl PromptType for Extended {}

/// 用于创建具有可自定义选项的提示请求的构建器。
/// 使用泛型来跟踪构建过程中设置了哪些选项。
///
/// 如果您期望连续调用工具，您需要确保使用 `.multi_turn()`
/// 参数来添加更多轮次，因为默认情况下它是 0（意味着只有 1 个工具往返）。否则，
/// 尝试 await（这将发送提示请求）可能会返回
/// [`crate::completion::request::PromptError::MaxDepthError`] 如果代理决定连续调用工具。
// 定义提示请求结构体，支持生命周期、状态类型、模型类型和钩子类型
pub struct PromptRequest<'a, S, M, P>
where
    // S 必须实现 PromptType trait
    S: PromptType,
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // P 必须实现 PromptHook trait
    P: PromptHook<M>,
{
    /// 要发送给模型的提示消息
    // 要发送给模型的提示消息
    prompt: Message,
    /// 包含在提示中的可选聊天历史
    /// 注意：聊天历史需要比代理存活更长时间，因为它可能与其他代理一起使用
    // 包含在提示中的可选聊天历史，注意：聊天历史需要比代理存活更长时间，因为它可能与其他代理一起使用
    chat_history: Option<&'a mut Vec<Message>>,
    /// 多轮对话的最大深度（0 表示无多轮）
    // 多轮对话的最大深度（0 表示无多轮）
    max_depth: usize,
    /// 用于执行的代理
    // 用于执行的代理
    agent: &'a Agent<M>,
    /// 用于跟踪请求类型的幽灵数据
    // 用于跟踪请求类型的幽灵数据
    state: PhantomData<S>,
    /// 事件的可选每请求钩子
    // 事件的可选每请求钩子
    hook: Option<P>,
}

// 为标准提示请求实现方法，支持泛型完成模型
impl<'a, M> PromptRequest<'a, Standard, M, ()>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    /// Create a new PromptRequest with the given prompt and model
    // 创建新的提示请求的公共方法
    pub fn new(agent: &'a Agent<M>, prompt: impl Into<Message>) -> Self {
        // 返回新的提示请求实例
        Self {
            // 将提示转换为消息
            prompt: prompt.into(),
            // 初始化聊天历史为 None
            chat_history: None,
            // 初始化最大深度为 0
            max_depth: 0,
            // 设置代理引用
            agent,
            // 设置幽灵数据
            state: PhantomData,
            // 初始化钩子为 None
            hook: None,
        }
    }
}

// 为通用提示请求实现方法，支持泛型状态类型、模型类型和钩子类型
impl<'a, S, M, P> PromptRequest<'a, S, M, P>
where
    // S 必须实现 PromptType trait
    S: PromptType,
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // P 必须实现 PromptHook trait
    P: PromptHook<M>,
{
    /// Enable returning extended details for responses (includes aggregated token usage)
    ///
    /// Note: This changes the type of the response from `.send` to return a `PromptResponse` struct
    /// instead of a simple `String`. This is useful for tracking token usage across multiple turns
    /// of conversation.
    // 启用返回扩展详细信息（包括聚合的 token 使用情况）
    // 注意：这会将响应类型从 `.send` 返回简单的 `String` 改为返回 `PromptResponse` 结构体
    // 这对于跟踪多轮对话中的 token 使用情况很有用
    pub fn extended_details(self) -> PromptRequest<'a, Extended, M, P> {
        // 返回扩展提示请求实例
        PromptRequest {
            // 复制提示消息
            prompt: self.prompt,
            // 复制聊天历史
            chat_history: self.chat_history,
            // 复制最大深度
            max_depth: self.max_depth,
            // 复制代理引用
            agent: self.agent,
            // 设置新的幽灵数据
            state: PhantomData,
            // 复制钩子
            hook: self.hook,
        }
    }
    /// Set the maximum depth for multi-turn conversations (ie, the maximum number of turns an LLM can have calling tools before writing a text response).
    /// If the maximum turn number is exceeded, it will return a [`crate::completion::request::PromptError::MaxDepthError`].
    // 设置多轮对话的最大深度（即 LLM 在写入文本响应之前可以调用工具的最大轮数）
    // 如果超过最大轮数，将返回 [`crate::completion::request::PromptError::MaxDepthError`]
    pub fn multi_turn(self, depth: usize) -> PromptRequest<'a, S, M, P> {
        // 返回设置多轮深度的提示请求实例
        PromptRequest {
            // 复制提示消息
            prompt: self.prompt,
            // 复制聊天历史
            chat_history: self.chat_history,
            // 设置新的最大深度
            max_depth: depth,
            // 复制代理引用
            agent: self.agent,
            // 复制幽灵数据
            state: PhantomData,
            // 复制钩子
            hook: self.hook,
        }
    }

    /// Add chat history to the prompt request
    // 为提示请求添加聊天历史
    pub fn with_history(self, history: &'a mut Vec<Message>) -> PromptRequest<'a, S, M, P> {
        // 返回带聊天历史的提示请求实例
        PromptRequest {
            // 复制提示消息
            prompt: self.prompt,
            // 设置聊天历史
            chat_history: Some(history),
            // 复制最大深度
            max_depth: self.max_depth,
            // 复制代理引用
            agent: self.agent,
            // 复制幽灵数据
            state: PhantomData,
            // 复制钩子
            hook: self.hook,
        }
    }

    /// Attach a per-request hook for tool call events
    // 为工具调用事件附加每请求钩子
    pub fn with_hook<P2>(self, hook: P2) -> PromptRequest<'a, S, M, P2>
    where
        // P2 必须实现 PromptHook trait
        P2: PromptHook<M>,
    {
        // 返回带钩子的提示请求实例
        PromptRequest {
            // 复制提示消息
            prompt: self.prompt,
            // 复制聊天历史
            chat_history: self.chat_history,
            // 复制最大深度
            max_depth: self.max_depth,
            // 复制代理引用
            agent: self.agent,
            // 复制幽灵数据
            state: PhantomData,
            // 设置新的钩子
            hook: Some(hook),
        }
    }
}

// dead code allowed because of functions being left empty to allow for users to not have to implement every single function
/// Trait for per-request hooks to observe tool call events.
// 允许死代码，因为函数留空以允许用户不必实现每个函数
// 用于观察工具调用事件的每请求钩子的 trait
pub trait PromptHook<M>: Clone + Send + Sync
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
    /// Called after the prompt is sent to the model and a response is received.
    // 在提示发送给模型并收到响应后调用
    fn on_completion_response(
        // 接收自身引用
        &self,
        // 提示消息的引用
        prompt: &Message,
        // 完成响应的引用
        response: &crate::completion::CompletionResponse<M::Response>,
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

// 为单元类型实现 PromptHook trait
impl<M> PromptHook<M> for () where M: CompletionModel {}

/// Due to: [RFC 2515](https://github.com/rust-lang/rust/issues/63063), we have to use a `BoxFuture`
///  for the `IntoFuture` implementation. In the future, we should be able to use `impl Future<...>`
///  directly via the associated type.
// 由于 [RFC 2515](https://github.com/rust-lang/rust/issues/63063)，我们必须使用 `BoxFuture` 来实现 `IntoFuture`
// 将来，我们应该能够通过关联类型直接使用 `impl Future<...>`
// 为标准提示请求实现 IntoFuture trait
impl<'a, M, P> IntoFuture for PromptRequest<'a, Standard, M, P>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // P 必须实现 PromptHook trait 并且具有静态生命周期
    P: PromptHook<M> + 'static,
{
    // 定义输出类型为字符串结果或提示错误
    type Output = Result<String, PromptError>;
    // 定义 IntoFuture 类型为盒装 Future，这个 Future 不应该比代理存活更久
    type IntoFuture = BoxFuture<'a, Self::Output>; // This future should not outlive the agent

    // 将自身转换为 Future
    fn into_future(self) -> Self::IntoFuture {
        // 调用 send 方法并装箱
        self.send().boxed()
    }
}

// 为扩展提示请求实现 IntoFuture trait
impl<'a, M, P> IntoFuture for PromptRequest<'a, Extended, M, P>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // P 必须实现 PromptHook trait 并且具有静态生命周期
    P: PromptHook<M> + 'static,
{
    // 定义输出类型为提示响应结果或提示错误
    type Output = Result<PromptResponse, PromptError>;
    // 定义 IntoFuture 类型为盒装 Future，这个 Future 不应该比代理存活更久
    type IntoFuture = BoxFuture<'a, Self::Output>; // This future should not outlive the agent

    // 将自身转换为 Future
    fn into_future(self) -> Self::IntoFuture {
        // 调用 send 方法并装箱
        self.send().boxed()
    }
}

// 为标准提示请求实现发送方法
impl<M, P> PromptRequest<'_, Standard, M, P>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // P 必须实现 PromptHook trait
    P: PromptHook<M>,
{
    // 异步发送方法，返回字符串结果或提示错误
    async fn send(self) -> Result<String, PromptError> {
        // 转换为扩展详情并发送，然后映射响应输出
        self.extended_details().send().await.map(|resp| resp.output)
    }
}

// 派生调试和克隆 trait
#[derive(Debug, Clone)]
// 定义提示响应结构体
pub struct PromptResponse {
    // 输出字符串
    pub output: String,
    // 总使用情况
    pub total_usage: Usage,
}

// 为提示响应实现方法
impl PromptResponse {
    // 创建新的提示响应的公共方法
    pub fn new(output: impl Into<String>, total_usage: Usage) -> Self {
        // 返回新的提示响应实例
        Self {
            // 将输出转换为字符串
            output: output.into(),
            // 设置总使用情况
            total_usage,
        }
    }
}

// 为扩展提示请求实现发送方法
impl<M, P> PromptRequest<'_, Extended, M, P>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // P 必须实现 PromptHook trait
    P: PromptHook<M>,
{
    // 异步发送方法，返回提示响应结果或提示错误
    async fn send(self) -> Result<PromptResponse, PromptError> {
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

        // 获取代理引用
        let agent = self.agent;
        // 设置聊天历史，如果有历史则添加到其中，否则创建新的向量
        let chat_history = if let Some(history) = self.chat_history {
            // 将提示添加到历史中
            history.push(self.prompt.to_owned());
            // 返回历史引用
            history
        } else {
            // 创建包含当前提示的新向量
            &mut vec![self.prompt.to_owned()]
        };

        // 如果提示有 RAG 文本，则记录到跨度中
        if let Some(text) = self.prompt.rag_text() {
            // 记录提示文本到跨度
            agent_span.record("gen_ai.prompt", text);
        }

        // 初始化当前最大深度为 0
        let mut current_max_depth = 0;
        // 初始化使用情况
        let mut usage = Usage::new();
        // 初始化当前跨度 ID 的原子计数器
        let current_span_id: AtomicU64 = AtomicU64::new(0);

        // We need to do at least 2 loops for 1 roundtrip (user expects normal message)
        // 我们需要至少进行 2 次循环来完成 1 次往返（用户期望正常消息）
        // 开始主循环，返回最后一个提示
        let last_prompt = loop {
            // 获取聊天历史中的最后一个提示消息
            let prompt = chat_history
                // 获取最后一个元素
                .last()
                // 克隆消息
                .cloned()
                // 期望总是至少有一条消息在聊天历史中
                .expect("there should always be at least one message in the chat history");

            // 如果当前最大深度超过最大深度加 1，则跳出循环
            if current_max_depth > self.max_depth + 1 {
                // 返回当前提示
                break prompt;
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
                // 调用完成调用前的钩子
                hook.on_completion_call(&prompt, &chat_history[..chat_history.len() - 1])
                    .await;
            }
            // 获取当前跨度
            let span = tracing::Span::current();
            // 创建聊天信息跨度
            let chat_span = info_span!(
                // 设置目标
                target: "rig::agent_chat",
                // 设置父跨度
                parent: &span,
                // 跨度名称
                "chat",
                // 操作名称
                gen_ai.operation.name = "chat",
                // 系统指令
                gen_ai.system_instructions = self.agent.preamble,
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

            // 设置聊天跨度的跟随关系
            let chat_span = if current_span_id.load(Ordering::SeqCst) != 0 {
                // 从当前跨度 ID 创建 ID
                let id = Id::from_u64(current_span_id.load(Ordering::SeqCst));
                // 设置跟随关系并拥有所有权
                chat_span.follows_from(id).to_owned()
            } else {
                // 直接使用聊天跨度
                chat_span
            };

            // 如果聊天跨度有 ID，则存储到当前跨度 ID 中
            if let Some(id) = chat_span.id() {
                // 存储跨度 ID
                current_span_id.store(id.into_u64(), Ordering::SeqCst);
            };

            // 调用代理的完成方法获取响应
            let resp = agent
                // 调用完成方法，传入克隆的提示和聊天历史
                .completion(
                    prompt.clone(),
                    chat_history[..chat_history.len() - 1].to_vec(),
                )
                // 等待异步操作完成
                .await?
                // 发送请求
                .send()
                // 使用聊天跨度进行检测
                .instrument(chat_span.clone())
                // 等待异步操作完成
                .await?;

            // 累加使用情况
            usage += resp.usage;

            // 如果有钩子，则在完成响应后执行钩子
            if let Some(ref hook) = self.hook {
                // 调用完成响应后的钩子
                hook.on_completion_response(&prompt, &resp).await;
            }

            // 将响应选择分为工具调用和文本
            let (tool_calls, texts): (Vec<_>, Vec<_>) = resp
                // 获取选择列表的迭代器
                .choice
                .iter()
                // 根据是否为工具调用进行分区
                .partition(|choice| matches!(choice, AssistantContent::ToolCall(_)));

            // 将助手响应添加到聊天历史中
            chat_history.push(Message::Assistant {
                // 设置 ID 为 None
                id: None,
                // 复制响应选择内容
                content: resp.choice.clone(),
            });

            // 如果没有工具调用，则处理文本响应
            if tool_calls.is_empty() {
                // 合并文本内容
                let merged_texts = texts
                    // 转换为迭代器
                    .into_iter()
                    // 过滤并映射文本内容
                    .filter_map(|content| {
                        // 如果是文本内容，则提取文本
                        if let AssistantContent::Text(text) = content {
                            Some(text.text.clone())
                        } else {
                            None
                        }
                    })
                    // 收集为向量
                    .collect::<Vec<_>>()
                    // 用换行符连接
                    .join("\n");

                // 如果最大深度大于 1，则记录达到的深度
                if self.max_depth > 1 {
                    // 记录深度信息
                    tracing::info!("Depth reached: {}/{}", current_max_depth, self.max_depth);
                }

                // 记录完成信息到代理跨度
                agent_span.record("gen_ai.completion", &merged_texts);
                // 记录输入 token 使用情况
                agent_span.record("gen_ai.usage.input_tokens", usage.input_tokens);
                // 记录输出 token 使用情况
                agent_span.record("gen_ai.usage.output_tokens", usage.output_tokens);

                // If there are no tool calls, depth is not relevant, we can just return the merged text response.
                // 如果没有工具调用，深度就不相关，我们可以直接返回合并的文本响应
                return Ok(PromptResponse::new(merged_texts, usage));
            }

            // 克隆钩子引用
            let hook = self.hook.clone();
            // 处理工具调用流
            let tool_content = stream::iter(tool_calls)
                // 对每个工具调用进行处理
                .then(|choice| {
                    // 克隆钩子引用用于工具调用前
                    let hook1 = hook.clone();
                    // 克隆钩子引用用于工具调用后
                    let hook2 = hook.clone();

                    // 创建工具执行信息跨度
                    let tool_span = info_span!(
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

                    // 设置工具跨度的跟随关系
                    let tool_span = if current_span_id.load(Ordering::SeqCst) != 0 {
                        // 从当前跨度 ID 创建 ID
                        let id = Id::from_u64(current_span_id.load(Ordering::SeqCst));
                        // 设置跟随关系并拥有所有权
                        tool_span.follows_from(id).to_owned()
                    } else {
                        // 直接使用工具跨度
                        tool_span
                    };

                    // 如果工具跨度有 ID，则存储到当前跨度 ID 中
                    if let Some(id) = tool_span.id() {
                        // 存储跨度 ID
                        current_span_id.store(id.into_u64(), Ordering::SeqCst);
                    };

                    // 异步移动闭包
                    async move {
                        // 如果是工具调用内容
                        if let AssistantContent::ToolCall(tool_call) = choice {
                            // 获取工具名称
                            let tool_name = &tool_call.function.name;
                            // 将工具参数转换为字符串
                            let args = tool_call.function.arguments.to_string();
                            // 获取当前跨度
                            let tool_span = tracing::Span::current();
                            // 记录工具名称
                            tool_span.record("gen_ai.tool.name", tool_name);
                            // 记录工具调用 ID
                            tool_span.record("gen_ai.tool.call.id", &tool_call.id);
                            // 记录工具调用参数
                            tool_span.record("gen_ai.tool.call.arguments", &args);
                            // 如果有钩子，则调用工具调用前的钩子
                            if let Some(hook) = hook1 {
                                // 调用工具调用前的钩子
                                hook.on_tool_call(tool_name, &args).await;
                            }
                            // 调用工具并获取输出
                            let output = agent.tools.call(tool_name, args.clone()).await?;
                            // 如果有钩子，则调用工具调用后的钩子
                            if let Some(hook) = hook2 {
                                // 调用工具调用后的钩子
                                hook.on_tool_result(tool_name, &args, &output.to_string())
                                    .await;
                            }
                            // 记录工具调用结果
                            tool_span.record("gen_ai.tool.call.result", &output);
                            // 记录工具执行信息
                            tracing::info!(
                                "executed tool {tool_name} with args {args}. result: {output}"
                            );
                            // 根据是否有调用 ID 创建用户内容
                            if let Some(call_id) = tool_call.call_id.clone() {
                                // 返回带调用 ID 的工具结果用户内容
                                Ok(UserContent::tool_result_with_call_id(
                                    tool_call.id.clone(),
                                    call_id,
                                    OneOrMany::one(output.into()),
                                ))
                            } else {
                                // 返回不带调用 ID 的工具结果用户内容
                                Ok(UserContent::tool_result(
                                    tool_call.id.clone(),
                                    OneOrMany::one(output.into()),
                                ))
                            }
                        } else {
                            // 这种情况不应该发生，因为我们已经过滤了 `ToolCall`
                            unreachable!(
                                "This should never happen as we already filtered for `ToolCall`"
                            )
                        }
                    }
                    // 使用工具跨度进行检测
                    .instrument(tool_span)
                })
                // 收集所有结果
                .collect::<Vec<Result<UserContent, ToolSetError>>>()
                // 等待异步操作完成
                .await
                // 转换为迭代器
                .into_iter()
                // 收集结果或错误
                .collect::<Result<Vec<_>, _>>()
                // 将工具集错误映射为完成错误
                .map_err(|e| CompletionError::RequestError(Box::new(e)))?;

            // 将用户工具结果添加到聊天历史中
            chat_history.push(Message::User {
                // 将工具内容转换为多内容，期望至少有一个工具调用
                content: OneOrMany::many(tool_content).expect("There is atleast one tool call"),
            });
        };

        // If we reach here, we never resolved the final tool call. We need to do ... something.
        // 如果我们到达这里，我们从未解决最终的工具调用。我们需要做一些事情。
        // 返回最大深度错误
        Err(PromptError::MaxDepthError {
            // 最大深度
            max_depth: self.max_depth,
            // 聊天历史（装箱）
            chat_history: Box::new(chat_history.clone()),
            // 最后一个提示
            prompt: last_prompt,
        })
    }
}
