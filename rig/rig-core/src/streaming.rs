//! 此模块提供与流式完成模型协作的功能。
//! 它提供了用于生成流式完成请求和
//! 处理流式完成响应的 trait 和类型。
//!
//! 此模块中定义的主要 trait 有：
//! - [StreamingPrompt]：定义高级流式 LLM 一次性提示接口
//! - [StreamingChat]：定义带历史记录的高级流式 LLM 聊天接口
//! - [StreamingCompletion]：定义低级流式 LLM 完成接口
//!
// 此模块提供与流式完成模型协作的功能
// 它提供了用于生成流式完成请求和处理流式完成响应的 trait 和类型
// 此模块中定义的主要 trait 有：
// - StreamingPrompt：定义高级流式 LLM 一次性提示接口
// - StreamingChat：定义带历史记录的高级流式 LLM 聊天接口
// - StreamingCompletion：定义低级流式 LLM 完成接口

// 导入一或多个容器类型
use crate::OneOrMany;
// 导入代理相关类型
use crate::agent::Agent;
// 导入流式提示请求类型
use crate::agent::prompt_request::streaming::StreamingPromptRequest;
// 导入完成相关类型和 trait
use crate::completion::{
    CompletionError, CompletionModel, CompletionRequestBuilder, CompletionResponse, GetTokenUsage,
    Message, Usage,
};
// 导入消息相关类型
use crate::message::{AssistantContent, Reasoning, Text, ToolCall, ToolFunction};
// 导入流处理相关类型
use futures::stream::{AbortHandle, Abortable};
// 导入流扩展 trait
use futures::{Stream, StreamExt};
// 导入序列化和反序列化 trait
use serde::{Deserialize, Serialize};
// 导入装箱类型
use std::boxed::Box;
// 导入 Future trait
use std::future::Future;
// 导入 Pin 类型
use std::pin::Pin;
// 导入原子布尔类型
use std::sync::atomic::AtomicBool;
// 导入任务相关类型
use std::task::{Context, Poll};
// 导入 tokio 观察者通道
use tokio::sync::watch;

/// 用于暂停和恢复流式响应的控制
// 定义暂停控制结构体
pub struct PauseControl {
    // 暂停状态发送器
    pub(crate) paused_tx: watch::Sender<bool>,
    // 暂停状态接收器
    pub(crate) paused_rx: watch::Receiver<bool>,
}

// 暂停控制实现
impl PauseControl {
    // 创建新的暂停控制实例
    pub fn new() -> Self {
        // 创建观察者通道
        let (paused_tx, paused_rx) = watch::channel(false);
        // 返回暂停控制实例
        Self {
            paused_tx,
            paused_rx,
        }
    }

    // 暂停流式响应
    pub fn pause(&self) {
        // 发送暂停信号
        self.paused_tx.send(true).unwrap();
    }

    // 恢复流式响应
    pub fn resume(&self) {
        // 发送恢复信号
        self.paused_tx.send(false).unwrap();
    }

    // 检查是否处于暂停状态
    pub fn is_paused(&self) -> bool {
        // 获取当前暂停状态
        *self.paused_rx.borrow()
    }
}

// 为暂停控制实现默认 trait
impl Default for PauseControl {
    // 创建默认的暂停控制实例
    fn default() -> Self {
        Self::new()
    }
}

/// 表示来自模型的流式数据块的枚举
// 表示来自模型的流式数据块的枚举
#[derive(Debug, Clone)]
pub enum RawStreamingChoice<R>
where
    // R 必须实现 Clone trait
    R: Clone,
{
    /// 来自消息响应的文本块
    // 来自消息响应的文本块
    Message(String),

    /// 工具调用响应块
    // 工具调用响应块
    ToolCall {
        // 工具调用 ID
        id: String,
        // 调用 ID（可选）
        call_id: Option<String>,
        // 工具名称
        name: String,
        // 工具参数
        arguments: serde_json::Value,
    },
    /// 推理块
    // 推理块
    Reasoning {
        // 推理 ID（可选）
        id: Option<String>,
        // 推理内容
        reasoning: String,
    },

    /// 最终响应对象，如果您希望
    /// `StreamingCompletionResponse` 上的 `response` 字段被填充，则必须产生此对象
    // 最终响应对象，如果您希望 StreamingCompletionResponse 上的 response 字段被填充，则必须产生此对象
    FinalResponse(R),
}

// 条件编译：非 WASM 平台
#[cfg(not(target_arch = "wasm32"))]
// 流式结果类型别名（支持 Send）
pub type StreamingResult<R> =
    Pin<Box<dyn Stream<Item = Result<RawStreamingChoice<R>, CompletionError>> + Send>>;

// 条件编译：WASM 平台
#[cfg(target_arch = "wasm32")]
// 流式结果类型别名（不支持 Send）
pub type StreamingResult<R> =
    Pin<Box<dyn Stream<Item = Result<RawStreamingChoice<R>, CompletionError>>>>;

/// 来自流式完成请求的响应；
/// 消息和响应在 `inner` 流的末尾被填充。
// 来自流式完成请求的响应
// 消息和响应在 inner 流的末尾被填充
pub struct StreamingCompletionResponse<R>
where
    // R 必须实现 Clone、Unpin 和 GetTokenUsage trait
    R: Clone + Unpin + GetTokenUsage,
{
    // 内部可中断流
    pub(crate) inner: Abortable<StreamingResult<R>>,
    // 中断句柄
    pub(crate) abort_handle: AbortHandle,
    // 暂停控制
    pub(crate) pause_control: PauseControl,
    // 累积的文本内容
    text: String,
    // 累积的推理内容
    reasoning: String,
    // 累积的工具调用
    tool_calls: Vec<ToolCall>,
    /// 来自流的最终聚合消息
    /// 包含生成的所有文本和工具调用
    // 来自流的最终聚合消息，包含生成的所有文本和工具调用
    pub choice: OneOrMany<AssistantContent>,
    /// 来自流的最终响应，如果
    /// 提供商在流期间没有产生它，则可能为 `None`
    // 来自流的最终响应，如果提供商在流期间没有产生它，则可能为 None
    pub response: Option<R>,
    // 是否已产生最终响应的标志
    pub final_response_yielded: AtomicBool,
}

// 流式完成响应实现
impl<R> StreamingCompletionResponse<R>
where
    // R 必须实现 Clone、Unpin 和 GetTokenUsage trait
    R: Clone + Unpin + GetTokenUsage,
{
    // 创建新的流式完成响应
    pub fn stream(inner: StreamingResult<R>) -> StreamingCompletionResponse<R> {
        // 创建中断句柄对
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        // 创建可中断流
        let abortable_stream = Abortable::new(inner, abort_registration);
        // 创建暂停控制
        let pause_control = PauseControl::new();
        // 返回流式完成响应实例
        Self {
            inner: abortable_stream,
            abort_handle,
            pause_control,
            reasoning: String::new(),
            text: "".to_string(),
            tool_calls: vec![],
            choice: OneOrMany::one(AssistantContent::text("")),
            response: None,
            final_response_yielded: AtomicBool::new(false),
        }
    }

    // 取消流式响应
    pub fn cancel(&self) {
        self.abort_handle.abort();
    }

    // 暂停流式响应
    pub fn pause(&self) {
        self.pause_control.pause();
    }

    // 恢复流式响应
    pub fn resume(&self) {
        self.pause_control.resume();
    }

    // 检查是否处于暂停状态
    pub fn is_paused(&self) -> bool {
        self.pause_control.is_paused()
    }
}

// 为流式完成响应实现 From trait，转换为完成响应
impl<R> From<StreamingCompletionResponse<R>> for CompletionResponse<Option<R>>
where
    // R 必须实现 Clone、Unpin 和 GetTokenUsage trait
    R: Clone + Unpin + GetTokenUsage,
{
    // 从流式完成响应转换为完成响应
    fn from(value: StreamingCompletionResponse<R>) -> CompletionResponse<Option<R>> {
        // 创建完成响应
        CompletionResponse {
            choice: value.choice,
            usage: Usage::new(), // 使用情况在流式响应中不跟踪
            raw_response: value.response,
        }
    }
}

// 为流式完成响应实现 Stream trait
impl<R> Stream for StreamingCompletionResponse<R>
where
    // R 必须实现 Clone、Unpin 和 GetTokenUsage trait
    R: Clone + Unpin + GetTokenUsage,
{
    // 流项目类型
    type Item = Result<StreamedAssistantContent<R>, CompletionError>;

    // 轮询下一个项目
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // 获取可变引用
        let stream = self.get_mut();

        // 检查是否处于暂停状态
        if stream.is_paused() {
            // 唤醒任务以便稍后重试
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        // 轮询内部流
        match Pin::new(&mut stream.inner).poll_next(cx) {
            // 流未准备好
            Poll::Pending => Poll::Pending,
            // 流结束
            Poll::Ready(None) => {
                // 这在内部流的末尾运行，将所有 token 收集到
                // 单个统一的 Message 中
                let mut choice = vec![];

                // 添加所有工具调用到选择中
                stream.tool_calls.iter().for_each(|tc| {
                    choice.push(AssistantContent::ToolCall(tc.clone()));
                });

                // 这确保内容中始终至少有一个项目
                if choice.is_empty() || !stream.text.is_empty() {
                    choice.insert(0, AssistantContent::text(stream.text.clone()));
                }

                // 设置最终选择
                stream.choice = OneOrMany::many(choice)
                    .expect("There should be at least one assistant message");

                // 返回流结束
                Poll::Ready(None)
            }
            // 流错误
            Poll::Ready(Some(Err(err))) => {
                // 检查是否是中断错误
                if matches!(err, CompletionError::ProviderError(ref e) if e.to_string().contains("aborted"))
                {
                    return Poll::Ready(None); // 将取消视为流终止
                }
                // 返回错误
                Poll::Ready(Some(Err(err)))
            }
            // 流数据
            Poll::Ready(Some(Ok(choice))) => match choice {
                // 处理消息文本
                RawStreamingChoice::Message(text) => {
                    // 将流式 token 转发到外部流
                    // 并将文本连接在一起
                    stream.text = format!("{}{}", stream.text, text.clone());
                    Poll::Ready(Some(Ok(StreamedAssistantContent::text(&text))))
                }
                // 处理推理内容
                RawStreamingChoice::Reasoning { id, reasoning } => {
                    // 将流式 token 转发到外部流
                    // 并将文本连接在一起
                    stream.reasoning = format!("{}{}", stream.reasoning, reasoning.clone());
                    Poll::Ready(Some(Ok(StreamedAssistantContent::Reasoning(Reasoning {
                        id,
                        reasoning: vec![stream.reasoning.clone()],
                    }))))
                }
                // 处理工具调用
                RawStreamingChoice::ToolCall {
                    id,
                    name,
                    arguments,
                    call_id,
                } => {
                    // 跟踪每个工具调用以便稍后聚合最终消息
                    // 并将其传递给外部流
                    stream.tool_calls.push(ToolCall {
                        id: id.clone(),
                        call_id: call_id.clone(),
                        function: ToolFunction {
                            name: name.clone(),
                            arguments: arguments.clone(),
                        },
                    });
                    // 根据是否有 call_id 选择不同的工具调用内容
                    if let Some(call_id) = call_id {
                        Poll::Ready(Some(Ok(StreamedAssistantContent::tool_call_with_call_id(
                            id, call_id, name, arguments,
                        ))))
                    } else {
                        Poll::Ready(Some(Ok(StreamedAssistantContent::tool_call(
                            id, name, arguments,
                        ))))
                    }
                }
                // 处理最终响应
                RawStreamingChoice::FinalResponse(response) => {
                    // 检查是否已经产生过最终响应
                    if stream
                        .final_response_yielded
                        .load(std::sync::atomic::Ordering::SeqCst)
                    {
                        // 如果已经产生过，继续轮询下一个项目
                        stream.poll_next_unpin(cx)
                    } else {
                        // Set the final response field and return the next item in the stream
                        // 设置最终响应字段并返回流中的下一个项目
                        stream.response = Some(response.clone());
                        stream
                            .final_response_yielded
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                        let final_response = StreamedAssistantContent::final_response(response);
                        Poll::Ready(Some(Ok(final_response)))
                    }
                }
            },
        }
    }
}

/// Trait for high-level streaming prompt interface
// 高级流式提示接口的 trait
pub trait StreamingPrompt<M, R>
where
    // M 必须实现 CompletionModel trait 并且具有静态生命周期
    M: CompletionModel + 'static,
    // M 的流式响应必须支持 Send
    <M as CompletionModel>::StreamingResponse: Send,
    // R 必须实现 Clone、Unpin 和 GetTokenUsage trait
    R: Clone + Unpin + GetTokenUsage,
{
    /// Stream a simple prompt to the model
    // 向模型流式发送简单提示
    fn stream_prompt(&self, prompt: impl Into<Message> + Send) -> StreamingPromptRequest<M, ()>;
}

/// Trait for high-level streaming chat interface
// 高级流式聊天接口的 trait
pub trait StreamingChat<M, R>: Send + Sync
where
    // M 必须实现 CompletionModel trait 并且具有静态生命周期
    M: CompletionModel + 'static,
    // M 的流式响应必须支持 Send
    <M as CompletionModel>::StreamingResponse: Send,
    // R 必须实现 Clone、Unpin 和 GetTokenUsage trait
    R: Clone + Unpin + GetTokenUsage,
{
    /// Stream a chat with history to the model
    // 向模型流式发送带历史的聊天
    fn stream_chat(
        &self,
        prompt: impl Into<Message> + Send,
        chat_history: Vec<Message>,
    ) -> StreamingPromptRequest<M, ()>;
}

/// Trait for low-level streaming completion interface
// 低级流式完成接口的 trait
pub trait StreamingCompletion<M: CompletionModel> {
    /// Generate a streaming completion from a request
    // 从请求生成流式完成
    fn stream_completion(
        &self,
        prompt: impl Into<Message> + Send,
        chat_history: Vec<Message>,
    ) -> impl Future<Output = Result<CompletionRequestBuilder<M>, CompletionError>>;
}

// 动态流式结果结构体
pub(crate) struct StreamingResultDyn<R: Clone + Unpin> {
    // 内部流式结果
    pub(crate) inner: StreamingResult<R>,
}

// 为动态流式结果实现 Stream trait
impl<R: Clone + Unpin> Stream for StreamingResultDyn<R> {
    // 流项目类型
    type Item = Result<RawStreamingChoice<()>, CompletionError>;

    // 轮询下一个项目
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // 获取可变引用
        let stream = self.get_mut();

        // 轮询内部流
        match stream.inner.as_mut().poll_next(cx) {
            // 流未准备好
            Poll::Pending => Poll::Pending,
            // 流结束
            Poll::Ready(None) => Poll::Ready(None),
            // 流错误
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            // 流数据
            Poll::Ready(Some(Ok(chunk))) => match chunk {
                // 处理最终响应
                RawStreamingChoice::FinalResponse(_) => {
                    Poll::Ready(Some(Ok(RawStreamingChoice::FinalResponse(()))))
                }
                // 处理消息
                RawStreamingChoice::Message(m) => {
                    Poll::Ready(Some(Ok(RawStreamingChoice::Message(m))))
                }
                // 处理推理
                RawStreamingChoice::Reasoning { id, reasoning } => {
                    Poll::Ready(Some(Ok(RawStreamingChoice::Reasoning { id, reasoning })))
                }
                // 处理工具调用
                RawStreamingChoice::ToolCall {
                    id,
                    name,
                    arguments,
                    call_id,
                } => Poll::Ready(Some(Ok(RawStreamingChoice::ToolCall {
                    id,
                    name,
                    arguments,
                    call_id,
                }))),
            },
        }
    }
}

/// 将完成请求流式传输到标准输出的辅助函数
// 将完成请求流式传输到标准输出的辅助函数
pub async fn stream_to_stdout<M>(
    agent: &Agent<M>,
    stream: &mut StreamingCompletionResponse<M::StreamingResponse>,
) -> Result<(), std::io::Error>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    // 推理状态标志
    let mut is_reasoning = false;
    // 打印响应前缀
    print!("Response: ");
    // 循环处理流式数据块
    while let Some(chunk) = stream.next().await {
        match chunk {
            // 处理文本内容
            Ok(StreamedAssistantContent::Text(text)) => {
                // 如果之前是推理状态，打印分隔符
                if is_reasoning {
                    is_reasoning = false;
                    println!("\n---\n");
                }
                // 打印文本内容
                print!("{}", text.text);
                // 刷新输出缓冲区
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            // 处理工具调用
            Ok(StreamedAssistantContent::ToolCall(tool_call)) => {
                // 调用工具
                let res = agent
                    .tools
                    .call(
                        &tool_call.function.name,
                        tool_call.function.arguments.to_string(),
                    )
                    .await
                    // 将错误转换为 IO 错误
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                // 打印工具调用结果
                println!("\nResult: {res}");
            }
            // 处理最终响应
            Ok(StreamedAssistantContent::Final(res)) => {
                // 将响应序列化为 JSON
                let json_res = serde_json::to_string_pretty(&res).unwrap();
                println!();
                // 记录最终结果
                tracing::info!("Final result: {json_res}");
            }
            // 处理推理内容
            Ok(StreamedAssistantContent::Reasoning(Reasoning { reasoning, .. })) => {
                // 如果之前不是推理状态，打印推理前缀
                if !is_reasoning {
                    is_reasoning = true;
                    println!();
                    println!("Thinking: ");
                }
                // 连接推理内容
                let reasoning = reasoning.into_iter().collect::<Vec<String>>().join("");

                // 打印推理内容
                print!("{reasoning}");
                // 刷新输出缓冲区
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            // 处理错误
            Err(e) => {
                // 检查是否是中断错误
                if e.to_string().contains("aborted") {
                    println!("\nStream cancelled.");
                    break;
                }
                // 打印错误信息
                eprintln!("Error: {e}");
                break;
            }
        }
    }

    // 流式传输完成后的新行
    println!(); // 流式传输完成后的新行

    Ok(())
}

// 测试模块
#[cfg(test)]
mod tests {
    // 导入时间相关类型
    use std::time::Duration;

    // 导入父模块的所有内容
    use super::*;
    // 导入异步流宏
    use async_stream::stream;
    // 导入睡眠函数
    use tokio::time::sleep;

    // 派生调试和克隆 trait
    #[derive(Debug, Clone)]
    // 模拟响应结构体
    pub struct MockResponse {
        // 允许未使用的字段
        #[allow(dead_code)]
        // token 计数
        token_count: u32,
    }

    // 为模拟响应实现 GetTokenUsage trait
    impl GetTokenUsage for MockResponse {
        // 获取 token 使用情况
        fn token_usage(&self) -> Option<crate::completion::Usage> {
            // 创建使用情况
            let mut usage = Usage::new();
            // 设置总 token 数
            usage.total_tokens = 15;
            // 返回使用情况
            Some(usage)
        }
    }

    // 创建模拟流
    fn create_mock_stream() -> StreamingCompletionResponse<MockResponse> {
        // 创建异步流
        let stream = stream! {
            // 产生第一个消息
            yield Ok(RawStreamingChoice::Message("hello 1".to_string()));
            // 等待 100 毫秒
            sleep(Duration::from_millis(100)).await;
            // 产生第二个消息
            yield Ok(RawStreamingChoice::Message("hello 2".to_string()));
            // 等待 100 毫秒
            sleep(Duration::from_millis(100)).await;
            // 产生第三个消息
            yield Ok(RawStreamingChoice::Message("hello 3".to_string()));
            // 等待 100 毫秒
            sleep(Duration::from_millis(100)).await;
            // 产生最终响应
            yield Ok(RawStreamingChoice::FinalResponse(MockResponse { token_count: 15 }));
        };

        // 条件编译：非 WASM 平台
        #[cfg(not(target_arch = "wasm32"))]
        let pinned_stream: StreamingResult<MockResponse> = Box::pin(stream);
        // 条件编译：WASM 平台
        #[cfg(target_arch = "wasm32")]
        let pinned_stream: StreamingResult<MockResponse> = Box::pin(stream);

        // 创建流式完成响应
        StreamingCompletionResponse::stream(pinned_stream)
    }

    // 异步测试：流取消
    #[tokio::test]
    async fn test_stream_cancellation() {
        // 创建模拟流
        let mut stream = create_mock_stream();

        // 打印响应前缀
        println!("Response: ");
        // 块计数器
        let mut chunk_count = 0;
        // 循环处理流式数据块
        while let Some(chunk) = stream.next().await {
            match chunk {
                // 处理文本内容
                Ok(StreamedAssistantContent::Text(text)) => {
                    // 打印文本内容
                    print!("{}", text.text);
                    // 刷新输出缓冲区
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                    // 增加块计数
                    chunk_count += 1;
                }
                // 处理工具调用
                Ok(StreamedAssistantContent::ToolCall(tc)) => {
                    // 打印工具调用
                    println!("\nTool Call: {tc:?}");
                    // 增加块计数
                    chunk_count += 1;
                }
                // 处理最终响应
                Ok(StreamedAssistantContent::Final(res)) => {
                    // 打印最终响应
                    println!("\nFinal response: {res:?}");
                }
                // 处理推理内容
                Ok(StreamedAssistantContent::Reasoning(Reasoning { reasoning, .. })) => {
                    // 连接推理内容
                    let reasoning = reasoning.into_iter().collect::<Vec<String>>().join("");
                    // 打印推理内容
                    print!("{reasoning}");
                    // 刷新输出缓冲区
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                }
                // 处理错误
                Err(e) => {
                    // 打印错误信息
                    eprintln!("Error: {e:?}");
                    break;
                }
            }

            // 如果处理了足够的块，取消流
            if chunk_count >= 2 {
                println!("\nCancelling stream...");
                stream.cancel();
                println!("Stream cancelled.");
                break;
            }
        }

        // 验证取消后没有更多块
        let next_chunk = stream.next().await;
        assert!(
            next_chunk.is_none(),
            "Expected no further chunks after cancellation, got {next_chunk:?}"
        );
    }

    // 异步测试：流暂停和恢复
    #[tokio::test]
    async fn test_stream_pause_resume() {
        // 创建模拟流
        let stream = create_mock_stream();

        // Test pause
        // 测试暂停功能
        stream.pause();
        // 验证流处于暂停状态
        assert!(stream.is_paused());

        // Test resume
        // 测试恢复功能
        stream.resume();
        // 验证流不再处于暂停状态
        assert!(!stream.is_paused());
    }
}

/// Describes responses from a streamed provider response which is either text, a tool call or a final usage response.
// 描述来自流式提供商响应的响应，可以是文本、工具调用或最终使用响应
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 未标记的序列化
#[serde(untagged)]
pub enum StreamedAssistantContent<R> {
    // 文本内容
    Text(Text),
    // 工具调用
    ToolCall(ToolCall),
    // 推理内容
    Reasoning(Reasoning),
    // 最终响应
    Final(R),
}

// 流式助手内容实现
impl<R> StreamedAssistantContent<R>
where
    // R 必须实现 Clone 和 Unpin trait
    R: Clone + Unpin,
{
    // 创建文本内容
    pub fn text(text: &str) -> Self {
        Self::Text(Text {
            text: text.to_string(),
        })
    }

    /// Helper constructor to make creating assistant tool call content easier.
    // 辅助构造函数，使创建助手工具调用内容更容易
    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self::ToolCall(ToolCall {
            id: id.into(),
            call_id: None,
            function: ToolFunction {
                name: name.into(),
                arguments,
            },
        })
    }

    // 创建带调用 ID 的工具调用内容
    pub fn tool_call_with_call_id(
        id: impl Into<String>,
        call_id: String,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self::ToolCall(ToolCall {
            id: id.into(),
            call_id: Some(call_id),
            function: ToolFunction {
                name: name.into(),
                arguments,
            },
        })
    }

    // 创建最终响应内容
    pub fn final_response(res: R) -> Self {
        Self::Final(res)
    }
}
