//! 此模块提供与完成模型协作的功能。
//! 它提供了用于生成完成请求、
//! 处理完成响应和定义完成模型的 trait、结构体和枚举。
//!
//! 此模块中定义的主要 trait 有：
//! - [Prompt]：定义高级 LLM 一次性提示接口。
//! - [Chat]：定义带聊天历史的高级 LLM 聊天接口。
//! - [Completion]：定义用于生成完成请求的低级 LLM 完成接口。
//! - [CompletionModel]：定义可用于从请求生成完成
//!   响应的完成模型。
//!
//! [Prompt] 和 [Chat] trait 是用户应该用来
//! 与 LLM 模型交互的高级 trait。此外，对于使用多个 LLM 模型生成响应的
//! 复合代理，实现这些 trait 之一是一个好的做法。
//!
//! [Completion] trait 定义了在用户希望
//! 将请求发送到完成模型提供商之前进一步自定义请求时有用的低级接口。
//!
//! [CompletionModel] trait 旨在充当提供商和
//! 库之间的接口。它定义了用户需要实现的方法来定义
//! 自定义基础完成模型（即：私有或第三方 LLM 提供商）。
//!
//! 该模块还提供各种结构体和枚举来表示通用完成请求、
//! 响应和错误。
//!
//! 使用示例：
//! ```rust
//! use rig::providers::openai::{Client, self};
//! use rig::completion::*;
//!
//! // 初始化 OpenAI 客户端和完成模型
//! let openai = Client::new("your-openai-api-key");
//!
//! let gpt_4 = openai.completion_model(openai::GPT_4);
//!
//! // 创建完成请求
//! let request = gpt_4.completion_request("Who are you?")
//!     .preamble("\
//!         You are Marvin, an extremely smart but depressed robot who is \
//!         nonetheless helpful towards humanity.\
//!     ")
//!     .temperature(0.5)
//!     .build();
//!
//! // 发送完成请求并获取完成响应
//! let response = gpt_4.completion(request)
//!     .await
//!     .expect("Failed to get completion response");
//!
//! // 处理完成响应
//! match completion_response.choice {
//!     ModelChoice::Message(message) => {
//!         // Handle the completion response as a message
//!         println!("Received message: {}", message);
//!     }
//!     ModelChoice::ToolCall(tool_name, tool_params) => {
//!         // Handle the completion response as a tool call
//!         println!("Received tool call: {} {:?}", tool_name, tool_params);
//!     }
//! }
//! ```
//!
//! For more information on how to use the completion functionality, refer to the documentation of
//! the individual traits, structs, and enums defined in this module.

// 导入父模块的消息类型
use super::message::{AssistantContent, DocumentMediaType};
// 导入客户端完成模型句柄
use crate::client::completion::CompletionModelHandle;
// 导入工具选择类型
use crate::message::ToolChoice;
// 导入流式完成响应类型
use crate::streaming::StreamingCompletionResponse;
// 导入 OneOrMany 类型和流式处理模块
use crate::{OneOrMany, streaming};
// 导入工具模块
use crate::{
    // JSON 工具函数
    json_utils,
    // 消息和用户内容类型
    message::{Message, UserContent},
    // 工具集错误类型
    tool::ToolSetError,
};
// 导入 futures 的 BoxFuture 类型
use futures::future::BoxFuture;
// 导入 serde 的反序列化 trait
use serde::de::DeserializeOwned;
// 导入 serde 的序列化和反序列化 trait
use serde::{Deserialize, Serialize};
// 导入标准库的 HashMap
use std::collections::HashMap;
// 导入标准库的运算符 trait
use std::ops::{Add, AddAssign};
// 导入标准库的 Arc 智能指针
use std::sync::Arc;
// 导入错误处理宏
use thiserror::Error;

// 错误类型定义
#[derive(Debug, Error)]
pub enum CompletionError {
    /// Http error (e.g.: connection error, timeout, etc.)
    // HTTP 错误（例如：连接错误、超时等）
    #[error("HttpError: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Json error (e.g.: serialization, deserialization)
    // JSON 错误（例如：序列化、反序列化）
    #[error("JsonError: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Url error (e.g.: invalid URL)
    // URL 错误（例如：无效的 URL）
    #[error("UrlError: {0}")]
    UrlError(#[from] url::ParseError),

    /// Error building the completion request
    // 构建完成请求时的错误
    #[error("RequestError: {0}")]
    RequestError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// Error parsing the completion response
    // 解析完成响应时的错误
    #[error("ResponseError: {0}")]
    ResponseError(String),

    /// Error returned by the completion model provider
    // 完成模型提供商返回的错误
    #[error("ProviderError: {0}")]
    ProviderError(String),
}

/// Prompt errors
// 提示错误类型
#[derive(Debug, Error)]
pub enum PromptError {
    /// Something went wrong with the completion
    // 完成过程中出现问题
    #[error("CompletionError: {0}")]
    CompletionError(#[from] CompletionError),

    /// There was an error while using a tool
    // 使用工具时出现错误
    #[error("ToolCallError: {0}")]
    ToolError(#[from] ToolSetError),

    /// The LLM tried to call too many tools during a multi-turn conversation.
    /// To fix this, you may either need to lower the amount of tools your model has access to (and then create other agents to share the tool load)
    /// or increase the amount of turns given in `.multi_turn()`.
    // LLM 在多轮对话中尝试调用过多工具
    // 要修复此问题，您可能需要减少模型可访问的工具数量（然后创建其他代理来分担工具负载）
    // 或者在 `.multi_turn()` 中增加轮次数量
    #[error("MaxDepthError: (reached limit: {max_depth})")]
    MaxDepthError {
        // 最大深度
        max_depth: usize,
        // 聊天历史记录
        chat_history: Box<Vec<Message>>,
        // 提示消息
        prompt: Message,
    },
}

// 文档结构体，用于表示文档内容
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Document {
    // 文档 ID
    pub id: String,
    // 文档文本内容
    pub text: String,
    // 展平序列化，将额外属性合并到主结构体中
    #[serde(flatten)]
    // 额外的属性映射
    pub additional_props: HashMap<String, String>,
}

// 为 Document 实现 Display trait
impl std::fmt::Display for Document {
    // 格式化文档内容用于显示
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 写入格式化内容
        write!(
            f,
            // 使用 concat! 宏连接字符串字面量
            concat!("<file id: {}>\n", "{}\n", "</file>\n"),
            // 文档 ID
            self.id,
            // 根据是否有额外属性决定显示内容
            if self.additional_props.is_empty() {
                // 如果没有额外属性，直接显示文本
                self.text.clone()
            } else {
                // 如果有额外属性，先收集并排序
                let mut sorted_props = self.additional_props.iter().collect::<Vec<_>>();
                // 按键名排序
                sorted_props.sort_by(|a, b| a.0.cmp(b.0));
                // 格式化元数据
                let metadata = sorted_props
                    .iter()
                    // 将每个键值对格式化为 "key: value" 形式
                    .map(|(k, v)| format!("{k}: {v:?}"))
                    .collect::<Vec<_>>()
                    // 用空格连接所有元数据
                    .join(" ");
                // 格式化包含元数据的完整内容
                format!("<metadata {} />\n{}", metadata, self.text)
            }
        )
    }
}

// 工具定义结构体，用于描述工具的信息
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolDefinition {
    // 工具名称
    pub name: String,
    // 工具描述
    pub description: String,
    // 工具参数（JSON 格式）
    pub parameters: serde_json::Value,
}

// ================================================================
// 实现部分
// ================================================================
/// Trait defining a high-level LLM simple prompt interface (i.e.: prompt in, response out).
// 定义高级 LLM 简单提示接口的 trait（即：输入提示，输出响应）
pub trait Prompt: Send + Sync {
    /// Send a simple prompt to the underlying completion model.
    ///
    /// If the completion model's response is a message, then it is returned as a string.
    ///
    /// If the completion model's response is a tool call, then the tool is called and
    /// the result is returned as a string.
    ///
    /// If the tool does not exist, or the tool call fails, then an error is returned.
    // 向底层完成模型发送简单提示
    //
    // 如果完成模型的响应是消息，则作为字符串返回
    //
    // 如果完成模型的响应是工具调用，则调用工具并将结果作为字符串返回
    //
    // 如果工具不存在或工具调用失败，则返回错误
    fn prompt(
        // 自身引用
        &self,
        // 提示消息，可以转换为 Message 类型
        prompt: impl Into<Message> + Send,
    ) -> impl std::future::IntoFuture<Output = Result<String, PromptError>, IntoFuture: Send>;
}

/// Trait defining a high-level LLM chat interface (i.e.: prompt and chat history in, response out).
// 定义高级 LLM 聊天接口的 trait（即：输入提示和聊天历史，输出响应）
pub trait Chat: Send + Sync {
    /// Send a prompt with optional chat history to the underlying completion model.
    ///
    /// If the completion model's response is a message, then it is returned as a string.
    ///
    /// If the completion model's response is a tool call, then the tool is called and the result
    /// is returned as a string.
    ///
    /// If the tool does not exist, or the tool call fails, then an error is returned.
    // 向底层完成模型发送带有可选聊天历史的提示
    //
    // 如果完成模型的响应是消息，则作为字符串返回
    //
    // 如果完成模型的响应是工具调用，则调用工具并将结果作为字符串返回
    //
    // 如果工具不存在或工具调用失败，则返回错误
    fn chat(
        // 自身引用
        &self,
        // 提示消息，可以转换为 Message 类型
        prompt: impl Into<Message> + Send,
        // 聊天历史记录
        chat_history: Vec<Message>,
    ) -> impl std::future::IntoFuture<Output = Result<String, PromptError>, IntoFuture: Send>;
}

/// Trait defining a low-level LLM completion interface
// 定义低级 LLM 完成接口的 trait
pub trait Completion<M: CompletionModel> {
    /// Generates a completion request builder for the given `prompt` and `chat_history`.
    /// This function is meant to be called by the user to further customize the
    /// request at prompt time before sending it.
    ///
    /// ❗IMPORTANT: The type that implements this trait might have already
    /// populated fields in the builder (the exact fields depend on the type).
    /// For fields that have already been set by the model, calling the corresponding
    /// method on the builder will overwrite the value set by the model.
    ///
    /// For example, the request builder returned by [`Agent::completion`](crate::agent::Agent::completion) will already
    /// contain the `preamble` provided when creating the agent.
    // 为给定的提示和聊天历史生成完成请求构建器
    // 此函数旨在由用户在发送请求之前进一步自定义请求时调用
    //
    // ❗重要：实现此 trait 的类型可能已经在构建器中填充了字段（具体字段取决于类型）
    // 对于已由模型设置的字段，在构建器上调用相应方法将覆盖模型设置的值
    //
    // 例如，由 [`Agent::completion`](crate::agent::Agent::completion) 返回的请求构建器已经
    // 包含创建代理时提供的 `preamble`
    fn completion(
        // 自身引用
        &self,
        // 提示消息，可以转换为 Message 类型
        prompt: impl Into<Message> + Send,
        // 聊天历史记录
        chat_history: Vec<Message>,
    ) -> impl std::future::Future<Output = Result<CompletionRequestBuilder<M>, CompletionError>> + Send;
}

/// General completion response struct that contains the high-level completion choice
/// and the raw response. The completion choice contains one or more assistant content.
// 通用完成响应结构体，包含高级完成选择和原始响应
// 完成选择包含一个或多个助手内容
#[derive(Debug)]
pub struct CompletionResponse<T> {
    /// The completion choice (represented by one or more assistant message content)
    /// returned by the completion model provider
    // 完成选择（由一个或多个助手消息内容表示）
    // 由完成模型提供商返回
    pub choice: OneOrMany<AssistantContent>,
    /// Tokens used during prompting and responding
    // 提示和响应期间使用的令牌
    pub usage: Usage,
    /// The raw response returned by the completion model provider
    // 完成模型提供商返回的原始响应
    pub raw_response: T,
}

/// A trait for grabbing the token usage of a completion response.
///
/// Primarily designed for streamed completion responses in streamed multi-turn, as otherwise it would be impossible to do.
// 用于获取完成响应令牌使用情况的 trait
//
// 主要设计用于流式多轮对话中的流式完成响应，否则无法实现
pub trait GetTokenUsage {
    // 获取令牌使用情况
    fn token_usage(&self) -> Option<crate::completion::Usage>;
}

// 为单元类型实现 GetTokenUsage
impl GetTokenUsage for () {
    // 单元类型没有令牌使用情况
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        // 返回 None
        None
    }
}

// 为 Option<T> 实现 GetTokenUsage
impl<T> GetTokenUsage for Option<T>
where
    // T 必须实现 GetTokenUsage
    T: GetTokenUsage,
{
    // 获取 Option 中的令牌使用情况
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        // 如果包含值，则调用其 token_usage 方法
        if let Some(usage) = self {
            usage.token_usage()
        } else {
            // 如果没有值，返回 None
            None
        }
    }
}

/// Struct representing the token usage for a completion request.
/// If tokens used are `0`, then the provider failed to supply token usage metrics.
// 表示完成请求令牌使用情况的结构体
// 如果使用的令牌为 `0`，则提供商未能提供令牌使用指标
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct Usage {
    /// The number of input ("prompt") tokens used in a given request.
    // 给定请求中使用的输入（"提示"）令牌数量
    pub input_tokens: u64,
    /// The number of output ("completion") tokens used in a given request.
    // 给定请求中使用的输出（"完成"）令牌数量
    pub output_tokens: u64,
    /// We store this separately as some providers may only report one number
    // 我们单独存储此值，因为某些提供商可能只报告一个数字
    pub total_tokens: u64,
}

// 为 Usage 实现方法
impl Usage {
    /// Creates a new instance of `Usage`.
    // 创建新的 Usage 实例
    pub fn new() -> Self {
        Self {
            // 初始化输入令牌为 0
            input_tokens: 0,
            // 初始化输出令牌为 0
            output_tokens: 0,
            // 初始化总令牌为 0
            total_tokens: 0,
        }
    }
}

// 为 Usage 实现 Default trait
impl Default for Usage {
    // 返回默认值
    fn default() -> Self {
        // 调用 new 方法
        Self::new()
    }
}

// 为 Usage 实现 Add trait
impl Add for Usage {
    // 输出类型为自身
    type Output = Self;

    // 加法运算
    fn add(self, other: Self) -> Self::Output {
        Self {
            // 输入令牌相加
            input_tokens: self.input_tokens + other.input_tokens,
            // 输出令牌相加
            output_tokens: self.output_tokens + other.output_tokens,
            // 总令牌相加
            total_tokens: self.total_tokens + other.total_tokens,
        }
    }
}

// 为 Usage 实现 AddAssign trait
impl AddAssign for Usage {
    // 加法赋值运算
    fn add_assign(&mut self, other: Self) {
        // 输入令牌相加赋值
        self.input_tokens += other.input_tokens;
        // 输出令牌相加赋值
        self.output_tokens += other.output_tokens;
        // 总令牌相加赋值
        self.total_tokens += other.total_tokens;
    }
}

/// Trait defining a completion model that can be used to generate completion responses.
/// This trait is meant to be implemented by the user to define a custom completion model,
/// either from a third party provider (e.g.: OpenAI) or a local model.
// 定义可用于生成完成响应的完成模型的 trait
// 此 trait 旨在由用户实现以定义自定义完成模型，
// 可以是第三方提供商（例如：OpenAI）或本地模型
pub trait CompletionModel: Clone + Send + Sync {
    /// The raw response type returned by the underlying completion model.
    // 底层完成模型返回的原始响应类型
    type Response: Send + Sync + Serialize + DeserializeOwned;
    /// The raw response type returned by the underlying completion model when streaming.
    // 底层完成模型在流式传输时返回的原始响应类型
    type StreamingResponse: Clone
        + Unpin
        + Send
        + Sync
        + Serialize
        + DeserializeOwned
        + GetTokenUsage;

    /// Generates a completion response for the given completion request.
    // 为给定的完成请求生成完成响应
    fn completion(
        // 自身引用
        &self,
        // 完成请求
        request: CompletionRequest,
    ) -> impl std::future::Future<
        Output = Result<CompletionResponse<Self::Response>, CompletionError>,
    > + Send;

    // 流式生成完成响应
    fn stream(
        // 自身引用
        &self,
        // 完成请求
        request: CompletionRequest,
    ) -> impl std::future::Future<
        Output = Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>,
    > + Send;

    /// Generates a completion request builder for the given `prompt`.
    // 为给定的提示生成完成请求构建器
    fn completion_request(&self, prompt: impl Into<Message>) -> CompletionRequestBuilder<Self> {
        // 创建新的构建器
        CompletionRequestBuilder::new(self.clone(), prompt)
    }
}
pub trait CompletionModelDyn: Send + Sync {
    fn completion(
        &self,
        request: CompletionRequest,
    ) -> BoxFuture<'_, Result<CompletionResponse<()>, CompletionError>>;

    fn stream(
        &self,
        request: CompletionRequest,
    ) -> BoxFuture<'_, Result<StreamingCompletionResponse<()>, CompletionError>>;

    fn completion_request(
        &self,
        prompt: Message,
    ) -> CompletionRequestBuilder<CompletionModelHandle<'_>>;
}

impl<T, R> CompletionModelDyn for T
where
    T: CompletionModel<StreamingResponse = R>,
    R: Clone + Unpin + GetTokenUsage + 'static,
{
    fn completion(
        &self,
        request: CompletionRequest,
    ) -> BoxFuture<'_, Result<CompletionResponse<()>, CompletionError>> {
        Box::pin(async move {
            self.completion(request)
                .await
                .map(|resp| CompletionResponse {
                    choice: resp.choice,
                    usage: resp.usage,
                    raw_response: (),
                })
        })
    }

    fn stream(
        &self,
        request: CompletionRequest,
    ) -> BoxFuture<'_, Result<StreamingCompletionResponse<()>, CompletionError>> {
        Box::pin(async move {
            let resp = self.stream(request).await?;
            let inner = resp.inner;

            let stream = Box::pin(streaming::StreamingResultDyn {
                inner: Box::pin(inner),
            });

            Ok(StreamingCompletionResponse::stream(stream))
        })
    }

    /// Generates a completion request builder for the given `prompt`.
    fn completion_request(
        &self,
        prompt: Message,
    ) -> CompletionRequestBuilder<CompletionModelHandle<'_>> {
        CompletionRequestBuilder::new(
            CompletionModelHandle {
                inner: Arc::new(self.clone()),
            },
            prompt,
        )
    }
}

/// Struct representing a general completion request that can be sent to a completion model provider.
// 表示可发送给完成模型提供商的通用完成请求的结构体
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// The preamble to be sent to the completion model provider
    // 要发送给完成模型提供商的序言
    pub preamble: Option<String>,
    /// The chat history to be sent to the completion model provider.
    /// The very last message will always be the prompt (hence why there is *always* one)
    // 要发送给完成模型提供商的聊天历史
    // 最后一条消息始终是提示（因此总是有一条）
    pub chat_history: OneOrMany<Message>,
    /// The documents to be sent to the completion model provider
    // 要发送给完成模型提供商的文档
    pub documents: Vec<Document>,
    /// The tools to be sent to the completion model provider
    // 要发送给完成模型提供商的工具
    pub tools: Vec<ToolDefinition>,
    /// The temperature to be sent to the completion model provider
    // 要发送给完成模型提供商的温度参数
    pub temperature: Option<f64>,
    /// The max tokens to be sent to the completion model provider
    // 要发送给完成模型提供商的最大令牌数
    pub max_tokens: Option<u64>,
    /// Whether tools are required to be used by the model provider or not before providing a response.
    // 模型提供商是否必须在提供响应之前使用工具
    pub tool_choice: Option<ToolChoice>,
    /// Additional provider-specific parameters to be sent to the completion model provider
    // 要发送给完成模型提供商的附加提供商特定参数
    pub additional_params: Option<serde_json::Value>,
}

// 为 CompletionRequest 实现方法
impl CompletionRequest {
    /// Returns documents normalized into a message (if any).
    /// Most providers do not accept documents directly as input, so it needs to convert into a
    ///  `Message` so that it can be incorporated into `chat_history` as a
    // 返回规范化为消息的文档（如果有的话）
    // 大多数提供商不接受文档作为直接输入，因此需要转换为消息
    // 以便可以将其作为用户内容合并到聊天历史中
    pub fn normalized_documents(&self) -> Option<Message> {
        // 如果没有文档，返回 None
        if self.documents.is_empty() {
            return None;
        }

        // Most providers will convert documents into a text unless it can handle document messages.
        // We use `UserContent::document` for those who handle it directly!
        // 大多数提供商会将文档转换为文本，除非它能处理文档消息
        // 对于直接处理文档的提供商，我们使用 `UserContent::document`！
        let messages = self
            // 遍历文档
            .documents
            .iter()
            .map(|doc| {
                // 创建用户内容文档
                UserContent::document(
                    // 将文档转换为字符串
                    doc.to_string(),
                    // In the future, we can customize `Document` to pass these extra types through.
                    // Most providers ditch these but they might want to use them.
                    // 将来，我们可以自定义 `Document` 来传递这些额外类型
                    // 大多数提供商会丢弃这些，但它们可能想要使用它们
                    Some(DocumentMediaType::TXT),
                )
            })
            .collect::<Vec<_>>();

        // 创建用户消息
        Some(Message::User {
            // 创建多个内容的消息
            content: OneOrMany::many(messages).expect("There will be atleast one document"),
        })
    }
}

/// Builder struct for constructing a completion request.
///
/// Example usage:
/// ```rust
/// use rig::{
///     providers::openai::{Client, self},
///     completion::CompletionRequestBuilder,
/// };
///
/// let openai = Client::new("your-openai-api-key");
/// let model = openai.completion_model(openai::GPT_4O).build();
///
/// // Create the completion request and execute it separately
/// let request = CompletionRequestBuilder::new(model, "Who are you?".to_string())
///     .preamble("You are Marvin from the Hitchhiker's Guide to the Galaxy.".to_string())
///     .temperature(0.5)
///     .build();
///
/// let response = model.completion(request)
///     .await
///     .expect("Failed to get completion response");
/// ```
///
/// Alternatively, you can execute the completion request directly from the builder:
/// ```rust
/// use rig::{
///     providers::openai::{Client, self},
///     completion::CompletionRequestBuilder,
/// };
///
/// let openai = Client::new("your-openai-api-key");
/// let model = openai.completion_model(openai::GPT_4O).build();
///
/// // Create the completion request and execute it directly
/// let response = CompletionRequestBuilder::new(model, "Who are you?".to_string())
///     .preamble("You are Marvin from the Hitchhiker's Guide to the Galaxy.".to_string())
///     .temperature(0.5)
///     .send()
///     .await
///     .expect("Failed to get completion response");
/// ```
///
/// Note: It is usually unnecessary to create a completion request builder directly.
/// Instead, use the [CompletionModel::completion_request] method.
// 用于构建完成请求的构建器结构体
//
// 使用示例：
// ```rust
// use rig::{
//     providers::openai::{Client, self},
//     completion::CompletionRequestBuilder,
// };
//
// let openai = Client::new("your-openai-api-key");
// let model = openai.completion_model(openai::GPT_4O).build();
//
// // 创建完成请求并分别执行
// let request = CompletionRequestBuilder::new(model, "Who are you?".to_string())
//     .preamble("You are Marvin from the Hitchhiker's Guide to the Galaxy.".to_string())
//     .temperature(0.5)
//     .build();
//
// let response = model.completion(request)
//     .await
//     .expect("Failed to get completion response");
// ```
//
// 或者，您可以直接从构建器执行完成请求：
// ```rust
// use rig::{
//     providers::openai::{Client, self},
//     completion::CompletionRequestBuilder,
// };
//
// let openai = Client::new("your-openai-api-key");
// let model = openai.completion_model(openai::GPT_4O).build();
//
// // 创建完成请求并直接执行
// let response = CompletionRequestBuilder::new(model, "Who are you?".to_string())
//     .preamble("You are Marvin from the Hitchhiker's Guide to the Galaxy.".to_string())
//     .temperature(0.5)
//     .send()
//     .await
//     .expect("Failed to get completion response");
// ```
//
// 注意：通常不需要直接创建完成请求构建器
// 相反，使用 [CompletionModel::completion_request] 方法
pub struct CompletionRequestBuilder<M: CompletionModel> {
    // 完成模型
    model: M,
    // 提示消息
    prompt: Message,
    // 序言
    preamble: Option<String>,
    // 聊天历史记录
    chat_history: Vec<Message>,
    // 文档列表
    documents: Vec<Document>,
    // 工具列表
    tools: Vec<ToolDefinition>,
    // 温度参数
    temperature: Option<f64>,
    // 最大令牌数
    max_tokens: Option<u64>,
    // 工具选择
    tool_choice: Option<ToolChoice>,
    // 附加参数
    additional_params: Option<serde_json::Value>,
}

// 为 CompletionRequestBuilder 实现方法
impl<M: CompletionModel> CompletionRequestBuilder<M> {
    // 创建新的完成请求构建器
    pub fn new(model: M, prompt: impl Into<Message>) -> Self {
        Self {
            // 设置完成模型
            model,
            // 转换并设置提示消息
            prompt: prompt.into(),
            // 初始化序言为 None
            preamble: None,
            // 初始化聊天历史记录为空向量
            chat_history: Vec::new(),
            // 初始化文档列表为空向量
            documents: Vec::new(),
            // 初始化工具列表为空向量
            tools: Vec::new(),
            // 初始化温度参数为 None
            temperature: None,
            // 初始化最大令牌数为 None
            max_tokens: None,
            // 初始化工具选择为 None
            tool_choice: None,
            // 初始化附加参数为 None
            additional_params: None,
        }
    }

    /// Sets the preamble for the completion request.
    // 为完成请求设置序言
    pub fn preamble(mut self, preamble: String) -> Self {
        // 设置序言
        self.preamble = Some(preamble);
        // 返回自身
        self
    }

    // 移除序言
    pub fn without_preamble(mut self) -> Self {
        // 将序言设置为 None
        self.preamble = None;
        // 返回自身
        self
    }

    /// Adds a message to the chat history for the completion request.
    pub fn message(mut self, message: Message) -> Self {
        self.chat_history.push(message);
        self
    }

    /// Adds a list of messages to the chat history for the completion request.
    pub fn messages(self, messages: Vec<Message>) -> Self {
        messages
            .into_iter()
            .fold(self, |builder, msg| builder.message(msg))
    }

    /// Adds a document to the completion request.
    pub fn document(mut self, document: Document) -> Self {
        self.documents.push(document);
        self
    }

    /// Adds a list of documents to the completion request.
    pub fn documents(self, documents: Vec<Document>) -> Self {
        documents
            .into_iter()
            .fold(self, |builder, doc| builder.document(doc))
    }

    /// Adds a tool to the completion request.
    pub fn tool(mut self, tool: ToolDefinition) -> Self {
        self.tools.push(tool);
        self
    }

    /// Adds a list of tools to the completion request.
    pub fn tools(self, tools: Vec<ToolDefinition>) -> Self {
        tools
            .into_iter()
            .fold(self, |builder, tool| builder.tool(tool))
    }

    /// Adds additional parameters to the completion request.
    /// This can be used to set additional provider-specific parameters. For example,
    /// Cohere's completion models accept a `connectors` parameter that can be used to
    /// specify the data connectors used by Cohere when executing the completion
    /// (see `examples/cohere_connectors.rs`).
    pub fn additional_params(mut self, additional_params: serde_json::Value) -> Self {
        match self.additional_params {
            Some(params) => {
                self.additional_params = Some(json_utils::merge(params, additional_params));
            }
            None => {
                self.additional_params = Some(additional_params);
            }
        }
        self
    }

    /// Sets the additional parameters for the completion request.
    /// This can be used to set additional provider-specific parameters. For example,
    /// Cohere's completion models accept a `connectors` parameter that can be used to
    /// specify the data connectors used by Cohere when executing the completion
    /// (see `examples/cohere_connectors.rs`).
    pub fn additional_params_opt(mut self, additional_params: Option<serde_json::Value>) -> Self {
        self.additional_params = additional_params;
        self
    }

    /// Sets the temperature for the completion request.
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the temperature for the completion request.
    pub fn temperature_opt(mut self, temperature: Option<f64>) -> Self {
        self.temperature = temperature;
        self
    }

    /// Sets the max tokens for the completion request.
    /// Note: This is required if using Anthropic
    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Sets the max tokens for the completion request.
    /// Note: This is required if using Anthropic
    pub fn max_tokens_opt(mut self, max_tokens: Option<u64>) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Sets the thing.
    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Builds the completion request.
    pub fn build(self) -> CompletionRequest {
        let chat_history = OneOrMany::many([self.chat_history, vec![self.prompt]].concat())
            .expect("There will always be atleast the prompt");

        CompletionRequest {
            preamble: self.preamble,
            chat_history,
            documents: self.documents,
            tools: self.tools,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            tool_choice: self.tool_choice,
            additional_params: self.additional_params,
        }
    }

    /// Sends the completion request to the completion model provider and returns the completion response.
    pub async fn send(self) -> Result<CompletionResponse<M::Response>, CompletionError> {
        let model = self.model.clone();
        model.completion(self.build()).await
    }

    /// Stream the completion request
    pub async fn stream<'a>(
        self,
    ) -> Result<StreamingCompletionResponse<M::StreamingResponse>, CompletionError>
    where
        <M as CompletionModel>::StreamingResponse: 'a,
        Self: 'a,
    {
        let model = self.model.clone();
        model.stream(self.build()).await
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_document_display_without_metadata() {
        let doc = Document {
            id: "123".to_string(),
            text: "This is a test document.".to_string(),
            additional_props: HashMap::new(),
        };

        let expected = "<file id: 123>\nThis is a test document.\n</file>\n";
        assert_eq!(format!("{doc}"), expected);
    }

    #[test]
    fn test_document_display_with_metadata() {
        let mut additional_props = HashMap::new();
        additional_props.insert("author".to_string(), "John Doe".to_string());
        additional_props.insert("length".to_string(), "42".to_string());

        let doc = Document {
            id: "123".to_string(),
            text: "This is a test document.".to_string(),
            additional_props,
        };

        let expected = concat!(
            "<file id: 123>\n",
            "<metadata author: \"John Doe\" length: \"42\" />\n",
            "This is a test document.\n",
            "</file>\n"
        );
        assert_eq!(format!("{doc}"), expected);
    }

    #[test]
    fn test_normalize_documents_with_documents() {
        let doc1 = Document {
            id: "doc1".to_string(),
            text: "Document 1 text.".to_string(),
            additional_props: HashMap::new(),
        };

        let doc2 = Document {
            id: "doc2".to_string(),
            text: "Document 2 text.".to_string(),
            additional_props: HashMap::new(),
        };

        let request = CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one("What is the capital of France?".into()),
            documents: vec![doc1, doc2],
            tools: Vec::new(),
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
        };

        let expected = Message::User {
            content: OneOrMany::many(vec![
                UserContent::document(
                    "<file id: doc1>\nDocument 1 text.\n</file>\n".to_string(),
                    Some(DocumentMediaType::TXT),
                ),
                UserContent::document(
                    "<file id: doc2>\nDocument 2 text.\n</file>\n".to_string(),
                    Some(DocumentMediaType::TXT),
                ),
            ])
            .expect("There will be at least one document"),
        };

        assert_eq!(request.normalized_documents(), Some(expected));
    }

    #[test]
    fn test_normalize_documents_without_documents() {
        let request = CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one("What is the capital of France?".into()),
            documents: Vec::new(),
            tools: Vec::new(),
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
        };

        assert_eq!(request.normalized_documents(), None);
    }
}
