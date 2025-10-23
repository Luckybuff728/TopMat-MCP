//! DeepSeek API 客户端和 Rig 集成
//!
//! # 示例
//! ```
//! use rig::providers::deepseek;
//!
//! let client = deepseek::Client::new("DEEPSEEK_API_KEY");
//!
//! let deepseek_chat = client.completion_model(deepseek::DEEPSEEK_CHAT);
//! ```

// DeepSeek API 客户端和 Rig 集成模块
// 提供与 DeepSeek API 的完整集成，包括聊天完成、流式响应和工具调用

// 导入异步流宏
use async_stream::stream;
// 导入 Future 流的扩展方法
use futures::StreamExt;
// 导入事件源相关类型
use reqwest_eventsource::{Event, RequestBuilderExt};
// 导入标准库的 HashMap
use std::collections::HashMap;
// 导入跟踪模块
use tracing::{Instrument, info_span};

// 导入客户端相关的 trait 和类型
use crate::client::{
    ClientBuilderError, CompletionClient, ProviderClient, VerifyClient, VerifyError,
};
// 导入获取令牌使用量 trait
use crate::completion::GetTokenUsage;
// 导入 JSON 合并工具
use crate::json_utils::merge;
// 导入消息文档相关类型
use crate::message::{Document, DocumentSourceKind};
// 导入核心类型和模块
use crate::{
    OneOrMany,
    completion::{self, CompletionError, CompletionRequest},
    impl_conversion_traits, json_utils, message,
};
// 导入 HTTP 客户端
use reqwest::Client as HttpClient;
// 导入序列化和反序列化宏
use serde::{Deserialize, Serialize};
// 导入 JSON 宏
use serde_json::json;

// 导入 OpenAI 的流式工具调用类型
use super::openai::StreamingToolCall;

// ================================================================
// Main DeepSeek Client
// 主 DeepSeek 客户端
// ================================================================
// DeepSeek API 基础 URL 常量
const DEEPSEEK_API_BASE_URL: &str = "https://api.deepseek.com";

// 客户端构建器结构体
pub struct ClientBuilder<'a> {
    // API 密钥
    api_key: &'a str,
    // 基础 URL
    base_url: &'a str,
    // HTTP 客户端（可选）
    http_client: Option<reqwest::Client>,
}

// ClientBuilder 的实现
impl<'a> ClientBuilder<'a> {
    // 创建新的客户端构建器
    pub fn new(api_key: &'a str) -> Self {
        Self {
            // 设置 API 密钥
            api_key,
            // 设置默认基础 URL
            base_url: DEEPSEEK_API_BASE_URL,
            // 初始化 HTTP 客户端为 None
            http_client: None,
        }
    }

    // 设置基础 URL
    pub fn base_url(mut self, base_url: &'a str) -> Self {
        // 更新基础 URL
        self.base_url = base_url;
        // 返回自身以支持链式调用
        self
    }

    // 设置自定义 HTTP 客户端
    pub fn custom_client(mut self, client: reqwest::Client) -> Self {
        // 设置 HTTP 客户端
        self.http_client = Some(client);
        // 返回自身以支持链式调用
        self
    }

    // 构建客户端
    pub fn build(self) -> Result<Client, ClientBuilderError> {
        // 确定使用的 HTTP 客户端
        let http_client = if let Some(http_client) = self.http_client {
            // 使用提供的客户端
            http_client
        } else {
            // 创建默认客户端
            reqwest::Client::builder().build()?
        };

        // 返回构建的客户端
        Ok(Client {
            // 转换基础 URL 为字符串
            base_url: self.base_url.to_string(),
            // 转换 API 密钥为字符串
            api_key: self.api_key.to_string(),
            // 设置 HTTP 客户端
            http_client,
        })
    }
}

// 派生 Clone trait
#[derive(Clone)]
// 客户端结构体
pub struct Client {
    // 基础 URL（公开）
    pub base_url: String,
    // API 密钥
    api_key: String,
    // HTTP 客户端
    http_client: HttpClient,
}

// 为 Client 实现 Debug trait
impl std::fmt::Debug for Client {
    // 格式化调试输出
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            // 输出基础 URL
            .field("base_url", &self.base_url)
            // 输出 HTTP 客户端
            .field("http_client", &self.http_client)
            // 隐藏 API 密钥（安全考虑）
            .field("api_key", &"<REDACTED>")
            .finish()
    }
}

// Client 的实现
impl Client {
    /// Create a new DeepSeek client builder.
    ///
    /// # Example
    /// ```
    /// use rig::providers::deepseek::{ClientBuilder, self};
    ///
    /// // Initialize the DeepSeek client
    /// let deepseek = Client::builder("your-deepseek-api-key")
    ///    .build()
    /// ```
    // 创建新的 DeepSeek 客户端构建器
    ///
    /// # 示例
    /// ```
    /// use rig::providers::deepseek::{ClientBuilder, self};
    ///
    /// // 初始化 DeepSeek 客户端
    /// let deepseek = Client::builder("your-deepseek-api-key")
    ///    .build()
    /// ```
    pub fn builder(api_key: &str) -> ClientBuilder<'_> {
        // 创建新的客户端构建器
        ClientBuilder::new(api_key)
    }

    /// Create a new DeepSeek client. For more control, use the `builder` method.
    ///
    /// # Panics
    /// - If the reqwest client cannot be built (if the TLS backend cannot be initialized).
    // 创建新的 DeepSeek 客户端。如需更多控制，请使用 `builder` 方法
    ///
    /// # 恐慌
    /// - 如果无法构建 reqwest 客户端（如果无法初始化 TLS 后端）
    pub fn new(api_key: &str) -> Self {
        Self::builder(api_key)
            .build()
            .expect("DeepSeek client should build")
    }

    // POST 请求方法（包可见）
    pub(crate) fn post(&self, path: &str) -> reqwest::RequestBuilder {
        // 构建完整的 URL
        let url = format!("{}/{}", self.base_url, path).replace("//", "/");
        // 创建 POST 请求并设置 Bearer 认证
        self.http_client.post(url).bearer_auth(&self.api_key)
    }

    // GET 请求方法（包可见）
    pub(crate) fn get(&self, path: &str) -> reqwest::RequestBuilder {
        // 构建完整的 URL
        let url = format!("{}/{}", self.base_url, path).replace("//", "/");
        // 创建 GET 请求并设置 Bearer 认证
        self.http_client.get(url).bearer_auth(&self.api_key)
    }
}

// 为 Client 实现 ProviderClient trait
impl ProviderClient for Client {
    // If you prefer the environment variable approach:
    // 如果您更喜欢环境变量方式：
    // 从环境变量创建客户端
    fn from_env() -> Self {
        // 获取 DEEPSEEK_API_KEY 环境变量
        let api_key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY not set");
        // 创建新的客户端
        Self::new(&api_key)
    }

    // 从 ProviderValue 创建客户端
    fn from_val(input: crate::client::ProviderValue) -> Self {
        // 解构 ProviderValue
        let crate::client::ProviderValue::Simple(api_key) = input else {
            // 如果不是 Simple 类型，则恐慌
            panic!("Incorrect provider value type")
        };
        // 创建新的客户端
        Self::new(&api_key)
    }
}

// 为 Client 实现 CompletionClient trait
impl CompletionClient for Client {
    // 完成模型类型
    type CompletionModel = CompletionModel;

    /// Creates a DeepSeek completion model with the given `model_name`.
    // 使用给定的 `model_name` 创建 DeepSeek 完成模型
    fn completion_model(&self, model_name: &str) -> CompletionModel {
        CompletionModel {
            // 克隆客户端
            client: self.clone(),
            // 转换模型名称为字符串
            model: model_name.to_string(),
        }
    }
}

// 为 Client 实现 VerifyClient trait
impl VerifyClient for Client {
    // 支持 worker 特性
    #[cfg_attr(feature = "worker", worker::send)]
    // 验证客户端连接
    async fn verify(&self) -> Result<(), VerifyError> {
        // 发送 GET 请求到余额端点
        let response = self.get("/user/balance").send().await?;
        // 匹配响应状态码
        match response.status() {
            // 200 OK - 验证成功
            reqwest::StatusCode::OK => Ok(()),
            // 401 未授权 - 无效认证
            reqwest::StatusCode::UNAUTHORIZED => Err(VerifyError::InvalidAuthentication),
            // 500 内部服务器错误或 503 服务不可用
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
            | reqwest::StatusCode::SERVICE_UNAVAILABLE => {
                // 返回提供商错误
                Err(VerifyError::ProviderError(response.text().await?))
            }
            // 其他状态码
            _ => {
                // 检查响应状态
                response.error_for_status()?;
                // 返回成功
                Ok(())
            }
        }
    }
}

// 为 Client 实现转换 traits
// 支持嵌入、转录、图像生成和音频生成
impl_conversion_traits!(
    AsEmbeddings,
    AsTranscription,
    AsImageGeneration,
    AsAudioGeneration for Client
);

// API 错误响应结构体
#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    // 错误消息
    message: String,
}

// API 响应枚举（未标记）
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiResponse<T> {
    // 成功响应
    Ok(T),
    // 错误响应
    Err(ApiErrorResponse),
}

// 为 ApiErrorResponse 实现转换到 CompletionError
impl From<ApiErrorResponse> for CompletionError {
    // 转换方法
    fn from(err: ApiErrorResponse) -> Self {
        // 将错误消息包装为 ProviderError
        CompletionError::ProviderError(err.message)
    }
}

/// The response shape from the DeepSeek API
// DeepSeek API 的响应结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    // We'll match the JSON:
    // 与 JSON 匹配：
    // 选择列表
    pub choices: Vec<Choice>,
    // 使用情况统计
    pub usage: Usage,
    // you may want other fields
    // 您可能需要其他字段
}

// 使用情况统计结构体
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Usage {
    // 完成令牌数
    pub completion_tokens: u32,
    // 提示令牌数
    pub prompt_tokens: u32,
    // 提示缓存命中令牌数
    pub prompt_cache_hit_tokens: u32,
    // 提示缓存未命中令牌数
    pub prompt_cache_miss_tokens: u32,
    // 总令牌数
    pub total_tokens: u32,
    // 完成令牌详情（如果为 None 则跳过序列化）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    // 提示令牌详情（如果为 None 则跳过序列化）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

// Usage 的实现
impl Usage {
    // 创建新的使用情况统计（所有字段初始化为 0 或 None）
    fn new() -> Self {
        Self {
            // 完成令牌数初始化为 0
            completion_tokens: 0,
            // 提示令牌数初始化为 0
            prompt_tokens: 0,
            // 提示缓存命中令牌数初始化为 0
            prompt_cache_hit_tokens: 0,
            // 提示缓存未命中令牌数初始化为 0
            prompt_cache_miss_tokens: 0,
            // 总令牌数初始化为 0
            total_tokens: 0,
            // 完成令牌详情初始化为 None
            completion_tokens_details: None,
            // 提示令牌详情初始化为 None
            prompt_tokens_details: None,
        }
    }
}

// 完成令牌详情结构体
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CompletionTokensDetails {
    // 推理令牌数（如果为 None 则跳过序列化）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

// 提示令牌详情结构体
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PromptTokensDetails {
    // 缓存令牌数（如果为 None 则跳过序列化）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

// 选择结构体
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    // 选择索引
    pub index: usize,
    // 消息内容
    pub message: Message,
    // 对数概率（可选）
    pub logprobs: Option<serde_json::Value>,
    // 结束原因
    pub finish_reason: String,
}

// 消息枚举（按角色标记，重命名为小写）
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    // 系统消息
    System {
        // 消息内容
        content: String,
        // 名称（如果为 None 则跳过序列化）
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    // 用户消息
    User {
        // 消息内容
        content: String,
        // 名称（如果为 None 则跳过序列化）
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    // 助手消息
    Assistant {
        // 消息内容
        content: String,
        // 名称（如果为 None 则跳过序列化）
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        // 工具调用列表（默认为空，使用自定义反序列化，如果为空则跳过序列化）
        #[serde(
            default,
            deserialize_with = "json_utils::null_or_vec",
            skip_serializing_if = "Vec::is_empty"
        )]
        tool_calls: Vec<ToolCall>,
    },
    // 工具结果（重命名为 "tool"）
    #[serde(rename = "tool")]
    ToolResult {
        // 工具调用 ID
        tool_call_id: String,
        // 消息内容
        content: String,
    },
}

// Message 的实现
impl Message {
    // 创建系统消息
    pub fn system(content: &str) -> Self {
        Message::System {
            // 转换内容为拥有的字符串
            content: content.to_owned(),
            // 名称为 None
            name: None,
        }
    }
}

impl From<message::ToolResult> for Message {
    fn from(tool_result: message::ToolResult) -> Self {
        let content = match tool_result.content.first() {
            message::ToolResultContent::Text(text) => text.text,
            message::ToolResultContent::Image(_) => String::from("[Image]"),
        };

        Message::ToolResult {
            tool_call_id: tool_result.id,
            content,
        }
    }
}

impl From<message::ToolCall> for ToolCall {
    fn from(tool_call: message::ToolCall) -> Self {
        Self {
            id: tool_call.id,
            // TODO: update index when we have it
            index: 0,
            r#type: ToolType::Function,
            function: Function {
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            },
        }
    }
}

impl TryFrom<message::Message> for Vec<Message> {
    type Error = message::MessageError;

    fn try_from(message: message::Message) -> Result<Self, Self::Error> {
        match message {
            message::Message::User { content } => {
                // extract tool results
                let mut messages = vec![];

                let tool_results = content
                    .clone()
                    .into_iter()
                    .filter_map(|content| match content {
                        message::UserContent::ToolResult(tool_result) => {
                            Some(Message::from(tool_result))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                messages.extend(tool_results);

                // extract text results
                let text_messages = content
                    .into_iter()
                    .filter_map(|content| match content {
                        message::UserContent::Text(text) => Some(Message::User {
                            content: text.text,
                            name: None,
                        }),
                        message::UserContent::Document(Document {
                            data:
                                DocumentSourceKind::Base64(content)
                                | DocumentSourceKind::String(content),
                            ..
                        }) => Some(Message::User {
                            content,
                            name: None,
                        }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                messages.extend(text_messages);

                Ok(messages)
            }
            message::Message::Assistant { content, .. } => {
                let mut messages: Vec<Message> = vec![];

                // extract text
                let text_content = content
                    .clone()
                    .into_iter()
                    .filter_map(|content| match content {
                        message::AssistantContent::Text(text) => Some(Message::Assistant {
                            content: text.text,
                            name: None,
                            tool_calls: vec![],
                        }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                messages.extend(text_content);

                // extract tool calls
                let tool_calls = content
                    .clone()
                    .into_iter()
                    .filter_map(|content| match content {
                        message::AssistantContent::ToolCall(tool_call) => {
                            Some(ToolCall::from(tool_call))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                // if we have tool calls, we add a new Assistant message with them
                if !tool_calls.is_empty() {
                    messages.push(Message::Assistant {
                        content: "".to_string(),
                        name: None,
                        tool_calls,
                    });
                }

                Ok(messages)
            }
        }
    }
}

// 工具调用结构体
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ToolCall {
    // 工具调用 ID
    pub id: String,
    // 工具调用索引
    pub index: usize,
    // 工具类型（默认）
    #[serde(default)]
    pub r#type: ToolType,
    // 函数信息
    pub function: Function,
}

// 函数结构体
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Function {
    // 函数名称
    pub name: String,
    // 函数参数（使用字符串化 JSON 格式）
    #[serde(with = "json_utils::stringified_json")]
    pub arguments: serde_json::Value,
}

// 工具类型枚举（重命名为小写）
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    // 默认类型：函数
    #[default]
    Function,
}

// 工具定义结构体
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolDefinition {
    // 工具类型
    pub r#type: String,
    // 函数定义
    pub function: completion::ToolDefinition,
}

// 为 ToolDefinition 实现从 crate::completion::ToolDefinition 的转换
impl From<crate::completion::ToolDefinition> for ToolDefinition {
    // 转换方法
    fn from(tool: crate::completion::ToolDefinition) -> Self {
        Self {
            // 设置类型为 "function"
            r#type: "function".into(),
            // 复制函数定义
            function: tool,
        }
    }
}

impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    type Error = CompletionError;

    fn try_from(response: CompletionResponse) -> Result<Self, Self::Error> {
        let choice = response.choices.first().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no choices".to_owned())
        })?;
        let content = match &choice.message {
            Message::Assistant {
                content,
                tool_calls,
                ..
            } => {
                let mut content = if content.trim().is_empty() {
                    vec![]
                } else {
                    vec![completion::AssistantContent::text(content)]
                };

                content.extend(
                    tool_calls
                        .iter()
                        .map(|call| {
                            completion::AssistantContent::tool_call(
                                &call.id,
                                &call.function.name,
                                call.function.arguments.clone(),
                            )
                        })
                        .collect::<Vec<_>>(),
                );
                Ok(content)
            }
            _ => Err(CompletionError::ResponseError(
                "Response did not contain a valid message or tool call".into(),
            )),
        }?;

        let choice = OneOrMany::many(content).map_err(|_| {
            CompletionError::ResponseError(
                "Response contained no message or tool call (empty)".to_owned(),
            )
        })?;

        let usage = completion::Usage {
            input_tokens: response.usage.prompt_tokens as u64,
            output_tokens: response.usage.completion_tokens as u64,
            total_tokens: response.usage.total_tokens as u64,
        };

        Ok(completion::CompletionResponse {
            choice,
            usage,
            raw_response: response,
        })
    }
}

/// The struct implementing the `CompletionModel` trait
// 实现 `CompletionModel` trait 的结构体
#[derive(Clone)]
pub struct CompletionModel {
    // 客户端
    pub client: Client,
    // 模型名称
    pub model: String,
}

// CompletionModel 的实现
impl CompletionModel {
    // 创建完成请求
    fn create_completion_request(
        &self,
        // 完成请求参数
        completion_request: CompletionRequest,
    ) -> Result<serde_json::Value, CompletionError> {
        // Build up the order of messages (context, chat_history, prompt)
        // 构建消息顺序（上下文、聊天历史、提示）
        let mut partial_history = vec![];

        // 如果有标准化文档，添加到历史中
        if let Some(docs) = completion_request.normalized_documents() {
            partial_history.push(docs);
        }

        // 扩展聊天历史
        partial_history.extend(completion_request.chat_history);

        // Initialize full history with preamble (or empty if non-existent)
        // 使用前言初始化完整历史（如果不存在则为空）
        let mut full_history: Vec<Message> = completion_request
            .preamble
            .map_or_else(Vec::new, |preamble| vec![Message::system(&preamble)]);

        // Convert and extend the rest of the history
        // 转换并扩展剩余的历史记录
        full_history.extend(
            partial_history
                .into_iter()
                .map(message::Message::try_into)
                .collect::<Result<Vec<Vec<Message>>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        );

        // 转换工具选择
        let tool_choice = completion_request
            .tool_choice
            .map(crate::providers::openrouter::ToolChoice::try_from)
            .transpose()?;

        // 根据是否有工具构建请求
        let request = if completion_request.tools.is_empty() {
            // 没有工具的请求
            json!({
                "model": self.model,
                "messages": full_history,
                "temperature": completion_request.temperature,
            })
        } else {
            // 有工具的请求
            json!({
                "model": self.model,
                "messages": full_history,
                "temperature": completion_request.temperature,
                "tools": completion_request.tools.into_iter().map(ToolDefinition::from).collect::<Vec<_>>(),
                "tool_choice": tool_choice,
            })
        };

        // 合并额外参数（如果有）
        let request = if let Some(params) = completion_request.additional_params {
            json_utils::merge(request, params)
        } else {
            request
        };

        // 返回构建的请求
        Ok(request)
    }
}

// 为 CompletionModel 实现 completion::CompletionModel trait
impl completion::CompletionModel for CompletionModel {
    // 响应类型
    type Response = CompletionResponse;
    // 流式响应类型
    type StreamingResponse = StreamingCompletionResponse;

    // 支持 worker 特性
    #[cfg_attr(feature = "worker", worker::send)]
    // 完成方法
    async fn completion(
        &self,
        // 完成请求参数
        completion_request: CompletionRequest,
    ) -> Result<
        // 返回完成响应
        completion::CompletionResponse<CompletionResponse>,
        // 完成错误
        crate::completion::CompletionError,
    > {
        // 克隆前言
        let preamble = completion_request.preamble.clone();
        // 创建完成请求
        let request = self.create_completion_request(completion_request)?;

        // 创建或获取追踪 span
        let span = if tracing::Span::current().is_disabled() {
            // 创建新的信息 span
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = "deepseek",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("messages").unwrap()).unwrap(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            // 使用当前 span
            tracing::Span::current()
        };

        // 记录调试信息
        tracing::debug!("DeepSeek completion request: {request:?}");

        // 异步移动块
        async move {
            // 发送 POST 请求到聊天完成端点
            let response = self
                .client
                .post("/chat/completions")
                .json(&request)
                .send()
                .await?;

            // 检查响应状态
            if response.status().is_success() {
                // 获取响应文本
                let t = response.text().await?;
                // 记录调试信息
                tracing::debug!(target: "rig", "DeepSeek completion: {t}");

                // 解析响应
                match serde_json::from_str::<ApiResponse<CompletionResponse>>(&t)? {
                    // 成功响应
                    ApiResponse::Ok(response) => {
                        // 获取当前 span
                        let span = tracing::Span::current();
                        // 记录输出消息
                        span.record(
                            "gen_ai.output.messages",
                            serde_json::to_string(&response.choices).unwrap(),
                        );
                        // 记录输入令牌数
                        span.record("gen_ai.usage.input_tokens", response.usage.prompt_tokens);
                        // 记录输出令牌数
                        span.record(
                            "gen_ai.usage.output_tokens",
                            response.usage.completion_tokens,
                        );
                        // 转换响应
                        response.try_into()
                    }
                    // 错误响应
                    ApiResponse::Err(err) => Err(CompletionError::ProviderError(err.message)),
                }
            } else {
                // 返回提供商错误
                Err(CompletionError::ProviderError(response.text().await?))
            }
        }
        // 应用追踪工具
        .instrument(span)
        .await
    }

    // 支持 worker 特性
    #[cfg_attr(feature = "worker", worker::send)]
    // 流式处理方法
    async fn stream(
        &self,
        // 完成请求参数
        completion_request: CompletionRequest,
    ) -> Result<
        // 返回流式完成响应
        crate::streaming::StreamingCompletionResponse<Self::StreamingResponse>,
        // 完成错误
        CompletionError,
    > {
        // 克隆前言
        let preamble = completion_request.preamble.clone();
        // 创建完成请求
        let mut request = self.create_completion_request(completion_request)?;

        // 合并流式选项（启用流式传输并包含使用情况）
        request = merge(
            request,
            json!({"stream": true, "stream_options": {"include_usage": true}}),
        );

        // 构建 HTTP 请求
        let builder = self.client.post("/chat/completions").json(&request);

        // 创建或获取追踪 span
        let span = if tracing::Span::current().is_disabled() {
            // 创建新的信息 span
            info_span!(
                target: "rig::completions",
                "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = "deepseek",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("messages").unwrap()).unwrap(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            // 使用当前 span
            tracing::Span::current()
        };

        // 使用追踪工具发送兼容的流式请求
        tracing::Instrument::instrument(send_compatible_streaming_request(builder), span).await
    }
}

// 流式增量结构体
#[derive(Deserialize, Debug)]
pub struct StreamingDelta {
    // 内容（默认，可选）
    #[serde(default)]
    content: Option<String>,
    // 工具调用列表（默认为空，使用自定义反序列化）
    #[serde(default, deserialize_with = "json_utils::null_or_vec")]
    tool_calls: Vec<StreamingToolCall>,
    // 推理内容（可选）
    reasoning_content: Option<String>,
}

// 流式选择结构体
#[derive(Deserialize, Debug)]
struct StreamingChoice {
    // 增量内容
    delta: StreamingDelta,
}

// 流式完成块结构体
#[derive(Deserialize, Debug)]
struct StreamingCompletionChunk {
    // 选择列表
    choices: Vec<StreamingChoice>,
    // 使用情况统计（可选）
    usage: Option<Usage>,
}

// 流式完成响应结构体
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct StreamingCompletionResponse {
    // 使用情况统计
    pub usage: Usage,
}

// 为 StreamingCompletionResponse 实现 GetTokenUsage trait
impl GetTokenUsage for StreamingCompletionResponse {
    // 获取令牌使用情况
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        // 创建新的使用情况统计
        let mut usage = crate::completion::Usage::new();
        // 设置输入令牌数
        usage.input_tokens = self.usage.prompt_tokens as u64;
        // 设置输出令牌数
        usage.output_tokens = self.usage.completion_tokens as u64;
        // 设置总令牌数
        usage.total_tokens = self.usage.total_tokens as u64;

        // 返回使用情况
        Some(usage)
    }
}

// 发送兼容的流式请求
pub async fn send_compatible_streaming_request(
    // 请求构建器
    request_builder: reqwest::RequestBuilder,
) -> Result<
    // 返回流式完成响应
    crate::streaming::StreamingCompletionResponse<StreamingCompletionResponse>,
    // 完成错误
    CompletionError,
> {
    // 获取当前追踪 span
    let span = tracing::Span::current();
    // 创建事件源（SSE 客户端）
    let mut event_source = request_builder
        .eventsource()
        .expect("Cloning request must succeed");

    // 创建流式响应流
    let stream = Box::pin(stream! {
        // 初始化最终使用情况统计
        let mut final_usage = Usage::new();
        // 初始化文本响应累积器
        let mut text_response = String::new();
        // 初始化工具调用映射（索引 -> (ID, 名称, 参数)）
        let mut calls: HashMap<usize, (String, String, String)> = HashMap::new();

        // 循环处理 SSE 事件
        while let Some(event_result) = event_source.next().await {
            match event_result {
                // SSE 连接打开事件
                Ok(Event::Open) => {
                    tracing::trace!("SSE connection opened");
                    continue;
                }
                // SSE 消息事件
                Ok(Event::Message(message)) => {
                    // 跳过空消息或 [DONE] 标记
                    if message.data.trim().is_empty() || message.data == "[DONE]" {
                        continue;
                    }

                    // 解析流式完成块
                    let parsed = serde_json::from_str::<StreamingCompletionChunk>(&message.data);
                    let Ok(data) = parsed else {
                        // 解析失败，记录调试信息并继续
                        let err = parsed.unwrap_err();
                        tracing::debug!("Couldn't parse SSE payload as StreamingCompletionChunk: {:?}", err);
                        continue;
                    };

                    // 处理第一个选择
                    if let Some(choice) = data.choices.first() {
                        let delta = &choice.delta;

                        // 处理工具调用
                        if !delta.tool_calls.is_empty() {
                            for tool_call in &delta.tool_calls {
                                let function = &tool_call.function;

                                // Start of tool call
                                // 工具调用开始（有函数名但无参数）
                                if function.name.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
                                    && function.arguments.is_empty()
                                {
                                    // 获取 ID 和名称
                                    let id = tool_call.id.clone().unwrap_or_default();
                                    let name = function.name.clone().unwrap();
                                    // 插入到工具调用映射
                                    calls.insert(tool_call.index, (id, name, String::new()));
                                }
                                // Continuation of tool call
                                // 工具调用继续（无函数名但有参数）
                                else if function.name.as_ref().map(|s| s.is_empty()).unwrap_or(true)
                                    && !function.arguments.is_empty()
                                {
                                    // 获取现有工具调用
                                    if let Some((id, name, existing_args)) = calls.get(&tool_call.index) {
                                        // 合并参数
                                        let combined = format!("{}{}", existing_args, function.arguments);
                                        // 更新工具调用映射
                                        calls.insert(tool_call.index, (id.clone(), name.clone(), combined));
                                    } else {
                                        // 记录调试信息
                                        tracing::debug!("Partial tool call received but tool call was never started.");
                                    }
                                }
                                // Complete tool call
                                // 完整的工具调用
                                else {
                                    // 获取 ID、名称和参数
                                    let id = tool_call.id.clone().unwrap_or_default();
                                    let name = function.name.clone().unwrap_or_default();
                                    let arguments_str = function.arguments.clone();

                                    // 解析参数 JSON
                                    let Ok(arguments_json) = serde_json::from_str::<serde_json::Value>(&arguments_str) else {
                                        tracing::debug!("Couldn't parse tool call args '{}'", arguments_str);
                                        continue;
                                    };

                                    // 生成工具调用结果
                                    yield Ok(crate::streaming::RawStreamingChoice::ToolCall {
                                        id,
                                        name,
                                        arguments: arguments_json,
                                        call_id: None,
                                    });
                                }
                            }
                        }

                        // DeepSeek-specific reasoning stream
                        // DeepSeek 特定的推理流
                        if let Some(content) = &delta.reasoning_content {
                            // 生成推理内容结果
                            yield Ok(crate::streaming::RawStreamingChoice::Reasoning {
                                reasoning: content.to_string(),
                                id: None,
                            });
                        }

                        // 处理文本内容
                        if let Some(content) = &delta.content {
                            // 累积文本响应
                            text_response += content;
                            // 生成消息结果
                            yield Ok(crate::streaming::RawStreamingChoice::Message(content.clone()));
                        }
                    }

                    // 更新使用情况统计
                    if let Some(usage) = data.usage {
                        final_usage = usage.clone();
                    }
                }
                // 流结束错误
                Err(reqwest_eventsource::Error::StreamEnded) => {
                    // 退出循环
                    break;
                }
                // 其他错误
                Err(err) => {
                    // 记录错误日志
                    tracing::error!(?err, "SSE error");
                    // 生成错误结果
                    yield Err(CompletionError::ResponseError(err.to_string()));
                    // 退出循环
                    break;
                }
            }
        }

        // 初始化工具调用列表
        let mut tool_calls = Vec::new();
        // Flush accumulated tool calls
        // 刷新累积的工具调用
        for (index, (id, name, arguments)) in calls {
            // 解析参数 JSON
            let Ok(arguments_json) = serde_json::from_str::<serde_json::Value>(&arguments) else {
                continue;
            };

            // 添加到工具调用列表
            tool_calls.push(ToolCall {
                id: id.clone(),
                index,
                r#type: ToolType::Function,
                function: Function {
                    name: name.clone(),
                    arguments: arguments_json.clone()
                }
            });
            // 生成工具调用结果
            yield Ok(crate::streaming::RawStreamingChoice::ToolCall {
                id,
                name,
                arguments: arguments_json,
                call_id: None,
            });
        }

        // 构建助手消息
        let message = Message::Assistant {
            content: text_response,
            name: None,
            tool_calls
        };

        // 记录输出消息到 span
        span.record("gen_ai.output.messages", serde_json::to_string(&message).unwrap());

        // 生成最终响应
        yield Ok(crate::streaming::RawStreamingChoice::FinalResponse(
            StreamingCompletionResponse { usage: final_usage.clone() }
        ));
    });

    // 返回流式完成响应
    Ok(crate::streaming::StreamingCompletionResponse::stream(
        stream,
    ))
}

// ================================================================
// DeepSeek Completion API
// DeepSeek 完成 API
// ================================================================

/// `deepseek-chat` completion model
// deepseek-chat 完成模型常量
pub const DEEPSEEK_CHAT: &str = "deepseek-chat";
/// `deepseek-reasoner` completion model
// deepseek-reasoner 完成模型常量
pub const DEEPSEEK_REASONER: &str = "deepseek-reasoner";

// Tests
// 测试模块
#[cfg(test)]
mod tests {

    // 导入父模块的所有公开项
    use super::*;

    // 测试反序列化选择向量
    #[test]
    fn test_deserialize_vec_choice() {
        let data = r#"[{
            "finish_reason": "stop",
            "index": 0,
            "logprobs": null,
            "message":{"role":"assistant","content":"Hello, world!"}
            }]"#;

        let choices: Vec<Choice> = serde_json::from_str(data).unwrap();
        assert_eq!(choices.len(), 1);
        match &choices.first().unwrap().message {
            Message::Assistant { content, .. } => assert_eq!(content, "Hello, world!"),
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_deserialize_deepseek_response() {
        let data = r#"{
            "choices":[{
                "finish_reason": "stop",
                "index": 0,
                "logprobs": null,
                "message":{"role":"assistant","content":"Hello, world!"}
            }],
            "usage": {
                "completion_tokens": 0,
                "prompt_tokens": 0,
                "prompt_cache_hit_tokens": 0,
                "prompt_cache_miss_tokens": 0,
                "total_tokens": 0
            }
        }"#;

        let jd = &mut serde_json::Deserializer::from_str(data);
        let result: Result<CompletionResponse, _> = serde_path_to_error::deserialize(jd);
        match result {
            Ok(response) => match &response.choices.first().unwrap().message {
                Message::Assistant { content, .. } => assert_eq!(content, "Hello, world!"),
                _ => panic!("Expected assistant message"),
            },
            Err(err) => {
                panic!("Deserialization error at {}: {}", err.path(), err);
            }
        }
    }

    #[test]
    fn test_deserialize_example_response() {
        let data = r#"
        {
            "id": "e45f6c68-9d9e-43de-beb4-4f402b850feb",
            "object": "chat.completion",
            "created": 0,
            "model": "deepseek-chat",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Why don’t skeletons fight each other?  \nBecause they don’t have the guts! 😄"
                    },
                    "logprobs": null,
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 13,
                "completion_tokens": 32,
                "total_tokens": 45,
                "prompt_tokens_details": {
                    "cached_tokens": 0
                },
                "prompt_cache_hit_tokens": 0,
                "prompt_cache_miss_tokens": 13
            },
            "system_fingerprint": "fp_4b6881f2c5"
        }
        "#;
        let jd = &mut serde_json::Deserializer::from_str(data);
        let result: Result<CompletionResponse, _> = serde_path_to_error::deserialize(jd);

        match result {
            Ok(response) => match &response.choices.first().unwrap().message {
                Message::Assistant { content, .. } => assert_eq!(
                    content,
                    "Why don’t skeletons fight each other?  \nBecause they don’t have the guts! 😄"
                ),
                _ => panic!("Expected assistant message"),
            },
            Err(err) => {
                panic!("Deserialization error at {}: {}", err.path(), err);
            }
        }
    }

    #[test]
    fn test_serialize_deserialize_tool_call_message() {
        let tool_call_choice_json = r#"
            {
              "finish_reason": "tool_calls",
              "index": 0,
              "logprobs": null,
              "message": {
                "content": "",
                "role": "assistant",
                "tool_calls": [
                  {
                    "function": {
                      "arguments": "{\"x\":2,\"y\":5}",
                      "name": "subtract"
                    },
                    "id": "call_0_2b4a85ee-b04a-40ad-a16b-a405caf6e65b",
                    "index": 0,
                    "type": "function"
                  }
                ]
              }
            }
        "#;

        let choice: Choice = serde_json::from_str(tool_call_choice_json).unwrap();

        let expected_choice: Choice = Choice {
            finish_reason: "tool_calls".to_string(),
            index: 0,
            logprobs: None,
            message: Message::Assistant {
                content: "".to_string(),
                name: None,
                tool_calls: vec![ToolCall {
                    id: "call_0_2b4a85ee-b04a-40ad-a16b-a405caf6e65b".to_string(),
                    function: Function {
                        name: "subtract".to_string(),
                        arguments: serde_json::from_str(r#"{"x":2,"y":5}"#).unwrap(),
                    },
                    index: 0,
                    r#type: ToolType::Function,
                }],
            },
        };

        assert_eq!(choice, expected_choice);
    }
}
