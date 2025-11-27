//! 通义千问（Qwen）API 客户端和 Rig 集成
//!
//! # 示例
//! ```
//! use rig::providers::qwen;
//!
//! let client = qwen::Client::new("YOUR_DASHSCOPE_API_KEY");
//!
//! let qwen_plus = client.completion_model(qwen::QWEN_PLUS);
//! ```

// 通义千问 API 客户端和 Rig 集成模块
// 提供与通义千问 API 的完整集成，包括聊天完成、流式响应和工具调用

// 导入异步流宏
use async_stream::stream;
// 导入 Future 流的扩展方法
use futures::StreamExt;
// 导入事件源相关类型
use crate::http_client::sse::{Event, GenericEventSource};
use crate::http_client::{self, HttpClientExt};
// 导入标准库的 HashMap
use std::collections::HashMap;
// 导入跟踪模块
use tracing::{Instrument, info_span};

// 导入 Rig 核心类型
use crate::{
    client::{ClientBuilderError, CompletionClient, ProviderClient, VerifyClient, VerifyError},
    completion::{self, CompletionError, CompletionRequest, message, MessageError},
    impl_conversion_traits, json_utils,
};

// 导入序列化相关
use serde::{Deserialize, Serialize};
use serde_json::json;

// 导入 JSON 工具
use crate::completion::GetTokenUsage;

// ================================================================
// 主 Qwen 客户端
// ================================================================
// 通义千问 API 基础 URL 常量
const QWEN_API_BASE_URL: &str = "https://dashscope.aliyuncs.com/api/v1/services/aigc";

// 客户端构建器结构体
pub struct ClientBuilder<'a, T = reqwest::Client> {
    // API 密钥
    api_key: &'a str,
    // 基础 URL
    base_url: &'a str,
    // HTTP 客户端
    http_client: T,
}

// ClientBuilder 的实现
impl<'a, T> ClientBuilder<'a, T>
where
    T: Default,
{
    // 创建新的客户端构建器
    pub fn new(api_key: &'a str) -> Self {
        Self {
            // 设置 API 密钥
            api_key,
            // 设置默认基础 URL
            base_url: QWEN_API_BASE_URL,
            // 初始化 HTTP 客户端
            http_client: T::default(),
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
    pub fn with_client<U>(self, http_client: U) -> ClientBuilder<'a, U> {
        ClientBuilder {
            api_key: self.api_key,
            base_url: self.base_url,
            http_client,
        }
    }

    // 构建客户端
    pub fn build(self) -> Result<Client<T>, ClientBuilderError> {
        // 返回构建的客户端
        Ok(Client {
            // 转换基础 URL 为字符串
            base_url: self.base_url.to_string(),
            // 转换 API 密钥为字符串
            api_key: self.api_key.to_string(),
            // 设置 HTTP 客户端
            http_client: self.http_client,
        })
    }
}

// 客户端结构体
#[derive(Clone)]
pub struct Client<T = reqwest::Client> {
    // 基础 URL（公开）
    pub base_url: String,
    // API 密钥
    api_key: String,
    // HTTP 客户端
    pub http_client: T,
}

// 为 Client 实现 Debug trait
impl<T> std::fmt::Debug for Client<T>
where
    T: std::fmt::Debug,
{
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
impl<T> Client<T>
where
    T: Default,
{
    /// Create a new Qwen client builder.
    ///
    /// # Example
    /// ```
    /// use rig::providers::qwen::{ClientBuilder, self};
    ///
    /// // Initialize the Qwen client
    /// let qwen_client = Client::builder("your-dashscope-api-key")
    ///    .build()
    /// ```
    // 创建新的通义千问客户端构建器
    ///
    /// # 示例
    /// ```
    /// use rig::providers::qwen::{ClientBuilder, self};
    ///
    /// // 初始化通义千问客户端
    /// let qwen_client = Client::builder("your-dashscope-api-key")
    ///    .build()
    /// ```
    pub fn builder(api_key: &str) -> ClientBuilder<'_, T> {
        // 创建新的客户端构建器
        ClientBuilder::new(api_key)
    }

    /// Create a new Qwen client. For more control, use the `builder` method.
    ///
    /// # Panics
    /// - If the reqwest client cannot be built (if the TLS backend cannot be initialized).
    // 创建新的通义千问客户端。如需更多控制，请使用 `builder` 方法
    ///
    /// # 恐慌
    /// - 如果无法构建 reqwest 客户端（如果无法初始化 TLS 后端）
    pub fn new(api_key: &str) -> Self {
        Self::builder(api_key)
            .build()
            .expect("Qwen client should build")
    }
}

// 为 reqwest::Client 提供具体的 new_with_api_key 方法实现
impl Client<reqwest::Client> {
    /// Create a new Qwen client with the given API key
    /// This is a convenience method for the common case of using reqwest::Client
    ///
    /// # Example
    /// ```
    /// use rig::providers::qwen;
    ///
    /// let client = qwen::Client::new_with_api_key("your-api-key");
    /// ```
    pub fn new_with_api_key(api_key: &str) -> Self {
        Self::builder(api_key)
            .build()
            .expect("Qwen client should build")
    }
}

// 为 reqwest::Client 默认实现的 Client
impl Default for Client<reqwest::Client> {
    fn default() -> Self {
        Self::new("default-api-key")
    }
}

// 为实现 HttpClientExt 的 Client 提供方法
impl<T> Client<T>
where
    T: HttpClientExt,
{
    // POST 请求方法（包可见）
    pub(crate) fn post(&self, path: &str) -> http_client::Result<http_client::Builder> {
        self.req(http_client::Method::POST, path)
    }

    // 通用请求方法
    fn req(
        &self,
        method: http_client::Method,
        path: &str,
    ) -> http_client::Result<http_client::Builder> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));

        http_client::with_bearer_auth(
            http_client::Request::builder().method(method).uri(url),
            &self.api_key,
        )
    }
}

// 为 Client 实现 ProviderClient trait
impl<T> ProviderClient for Client<T>
where
    T: HttpClientExt + Clone + std::fmt::Debug + Default + Send + 'static,
{
    // 从环境变量创建客户端
    fn from_env() -> Self {
        // 获取 DASHSCOPE_API_KEY 环境变量
        let api_key = std::env::var("DASHSCOPE_API_KEY").expect("DASHSCOPE_API_KEY not set");
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
impl<T> CompletionClient for Client<T>
where
    T: HttpClientExt + Clone + std::fmt::Debug + Default + Send + 'static,
{
    // 完成模型类型
    type CompletionModel = CompletionModel<T>;

    /// Creates a Qwen completion model with the given `model_name`.
    // 使用给定的 `model_name` 创建通义千问完成模型
    fn completion_model(&self, model_name: &str) -> CompletionModel<T> {
        CompletionModel {
            // 克隆客户端
            client: self.clone(),
            // 转换模型名称为字符串
            model: model_name.to_string(),
        }
    }
}

// 为 Client 实现 VerifyClient trait
impl<T> VerifyClient for Client<T>
where
    T: HttpClientExt + Clone + std::fmt::Debug + Default + Send + 'static,
{
    // 支持 worker 特性
    #[cfg_attr(feature = "worker", worker::send)]
    // 验证客户端连接
    async fn verify(&self) -> Result<(), VerifyError> {
        // 通义千问 API 没有专门的验证端点
        // 我们可以发送一个简单的请求来验证 API 密钥
        let request = self
            .post("text-generation/generation")?
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&json!({
                "model": "qwen-turbo",
                "input": {
                    "messages": [
                        {
                            "role": "user",
                            "content": "test"
                        }
                    ]
                },
                "parameters": {
                    "max_tokens": 1
                }
            })).map_err(|e| http_client::Error::from(http_client::Error::Instance(e.into())))?)
            .map_err(|e| VerifyError::ProviderError(e.to_string()))?;

        let response = self.http_client.send(request).await.map_err(http_client::Error::from)?;

        // 匹配响应状态码
        match response.status() {
            // 200 OK - 验证成功
            reqwest::StatusCode::OK => Ok(()),
            // 401 未授权 - 无效认证
            reqwest::StatusCode::UNAUTHORIZED => Err(VerifyError::InvalidAuthentication),
            // 403 禁止访问 - 无效认证
            reqwest::StatusCode::FORBIDDEN => Err(VerifyError::InvalidAuthentication),
            // 其他错误
            _ => Err(VerifyError::ProviderError(http_client::text(response).await?)),
        }
    }
}

// 为 Client 实现转换 traits
// 支持嵌入、转录、图像生成和音频生成
impl_conversion_traits!(
    AsEmbeddings,
    AsTranscription,
    AsImageGeneration,
    AsAudioGeneration for Client<T>
);

// ================================================================
// 通义千问完成 API
// ================================================================

/// `qwen-plus` 完成模型
// qwen-plus 完成模型常量
pub const QWEN_PLUS: &str = "qwen-plus";
/// `qwen-plus-latest` 完成模型
// qwen-plus-latest 完成模型常量
pub const QWEN_PLUS_LATEST: &str = "qwen-plus-latest";
/// `qwen-max` 完成模型
// qwen-max 完成模型常量
pub const QWEN_MAX: &str = "qwen-max";
/// `qwen-max-latest` 完成模型
// qwen-max-latest 完成模型常量
pub const QWEN_MAX_LATEST: &str = "qwen-max-latest";
/// `qwen-turbo` 完成模型
// qwen-turbo 完成模型常量
pub const QWEN_TURBO: &str = "qwen-turbo";
/// `qwen-turbo-latest` 完成模型
// qwen-turbo-latest 完成模型常量
pub const QWEN_TURBO_LATEST: &str = "qwen-turbo-latest";
/// `qwen-flash` 完成模型
// qwen-flash 完成模型常量
pub const QWEN_FLASH: &str = "qwen-flash";
/// `qwen3-max` 完成模型
// qwen3-max 完成模型常量
pub const QWEN3_MAX: &str = "qwen3-max";
/// `qwq-plus` 深度推理模型
// qwq-plus 深度推理模型常量
pub const QWQ_PLUS: &str = "qwq-plus";

// API 错误响应结构体
#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    // 错误代码
    code: String,
    // 错误消息
    message: String,
}

// API 响应枚举
#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum ApiResponse<T> {
    // 成功响应
    Ok(T),
    // 错误响应
    Err {
        // 状态码
        status_code: u16,
        // 错误代码
        code: String,
        // 错误消息
        message: String,
    },
}

// 为 ApiErrorResponse 实现转换到 CompletionError
impl From<ApiErrorResponse> for CompletionError {
    // 转换方法
    fn from(err: ApiErrorResponse) -> Self {
        // 将错误消息包装为 ProviderError
        CompletionError::ProviderError(format!("{}: {}", err.code, err.message))
    }
}

/// The response shape from the Qwen API
// 通义千问 API 的响应结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    // 请求 ID
    pub request_id: String,
    // 输出结果
    pub output: Output,
    // 使用情况统计
    pub usage: Usage,
}

// 输出结构体
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Output {
    // 选择列表
    pub choices: Vec<Choice>,
}

// 使用情况统计结构体
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Usage {
    // 输入令牌数
    pub input_tokens: u32,
    // 输出令牌数
    pub output_tokens: u32,
    // 总令牌数
    pub total_tokens: u32,
}

// Usage 的实现
impl Usage {
    // 创建新的使用情况统计（所有字段初始化为 0）
    fn new() -> Self {
        Self {
            // 输入令牌数初始化为 0
            input_tokens: 0,
            // 输出令牌数初始化为 0
            output_tokens: 0,
            // 总令牌数初始化为 0
            total_tokens: 0,
        }
    }
}

// 选择结构体
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    // 结束原因
    pub finish_reason: String,
    // 消息内容
    pub message: Message,
}

// 消息枚举（按角色标记，重命名为小写）
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    // 系统消息
    System {
        // 消息内容
        content: String,
    },
    // 用户消息
    User {
        // 消息内容
        content: String,
    },
    // 助手消息
    Assistant {
        // 消息内容
        content: String,
        // 推理内容（可选，用于 QwQ 等思考模型）
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
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
        }
    }
}

// 为 message::ToolResult 实现转换到 Message
impl From<message::ToolResult> for Message {
    // 转换方法
    fn from(tool_result: message::ToolResult) -> Self {
        // 提取内容
        let content = match tool_result.content.first() {
            // 文本内容
            message::ToolResultContent::Text(text) => text.text.clone(),
            // 图像内容（转换为占位符）
            message::ToolResultContent::Image(_) => String::from("[Image]"),
        };

        // 返回工具结果消息
        Message::ToolResult {
            tool_call_id: tool_result.id,
            content,
        }
    }
}

// 为 message::ToolCall 实现转换到 ToolCall
impl From<message::ToolCall> for ToolCall {
    // 转换方法
    fn from(tool_call: message::ToolCall) -> Self {
        Self {
            // 工具调用 ID
            id: tool_call.id,
            // 索引（通义千问不使用索引）
            index: 0,
            // 工具类型
            r#type: ToolType::Function,
            // 函数信息
            function: Function {
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            },
        }
    }
}

// 为 message::Message 实现转换到 Vec<Message>
impl TryFrom<message::Message> for Vec<Message> {
    // 错误类型
    type Error = MessageError;

    // 转换方法
    fn try_from(message: message::Message) -> Result<Self, Self::Error> {
        match message {
            // 用户消息
            message::Message::User { content } => {
                // 提取工具结果
                let mut messages = vec![];

                // 收集工具结果消息
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

                // 添加工具结果到消息列表
                messages.extend(tool_results);

                // 提取文本消息
                let text_messages = content
                    .into_iter()
                    .filter_map(|content| match content {
                        message::UserContent::Text(text) => Some(Message::User {
                            content: text.text,
                        }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                
                // 添加文本消息到消息列表
                messages.extend(text_messages);

                // 返回消息列表
                Ok(messages)
            }
            // 助手消息
            message::Message::Assistant { content, .. } => {
                let mut messages = vec![];
                let mut text_content = String::new();
                let mut tool_calls = vec![];

                // 遍历内容
                for item in content {
                    match item {
                        // 文本内容
                        completion::AssistantContent::Text(text) => {
                            text_content.push_str(&text.text);
                        }
                        // 工具调用
                        completion::AssistantContent::ToolCall(call) => {
                            tool_calls.push(ToolCall::from(call));
                        }
                        // 推理内容（暂不处理）
                        _ => {}
                    }
                }

                // 如果有内容或工具调用，添加助手消息
                if !text_content.is_empty() || !tool_calls.is_empty() {
                    messages.push(Message::Assistant {
                        content: text_content,
                        reasoning_content: None,
                        tool_calls,
                    });
                }

                // 返回消息列表
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

// 为 CompletionResponse 实现转换到 completion::CompletionResponse
impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    // 错误类型
    type Error = CompletionError;

    // 转换方法
    fn try_from(response: CompletionResponse) -> Result<Self, Self::Error> {
        // 获取第一个选择
        let choice = response.output.choices.first().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no choices".to_owned())
        })?;

        // 提取内容
        let content = match &choice.message {
            Message::Assistant {
                content,
                tool_calls,
                reasoning_content,
                ..
            } => {
                let mut result = vec![];

                // 添加推理内容（如果有）
                if let Some(reasoning) = reasoning_content {
                    if !reasoning.is_empty() {
                        result.push(completion::AssistantContent::Reasoning(
                            message::Reasoning::new(reasoning)
                        ));
                    }
                }

                // 添加文本内容
                if !content.trim().is_empty() {
                    result.push(completion::AssistantContent::text(content));
                }

                // 添加工具调用
                result.extend(
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

                Ok(result)
            }
            _ => Err(CompletionError::ResponseError(
                "Response did not contain assistant message".to_owned(),
            )),
        }?;

        // 构建使用情况统计
        let usage = completion::Usage {
            input_tokens: response.usage.input_tokens as u64,
            output_tokens: response.usage.output_tokens as u64,
            total_tokens: response.usage.total_tokens as u64,
        };

        // 返回完成响应
        Ok(completion::CompletionResponse {
            choice: crate::OneOrMany::many(content).map_err(|_| {
                CompletionError::ResponseError("Response contained no content".to_owned())
            })?,
            usage,
            raw_response: response,
        })
    }
}

/// The struct implementing the `CompletionModel` trait
// 实现 `CompletionModel` trait 的结构体
#[derive(Clone)]
pub struct CompletionModel<T = reqwest::Client> {
    // 客户端
    pub client: Client<T>,
    // 模型名称
    pub model: String,
}

// CompletionModel 的实现
impl<T> CompletionModel<T>
where
    T: HttpClientExt + Clone + std::fmt::Debug + Default + Send + 'static,
{
    // 创建完成请求
    fn create_completion_request(
        &self,
        // 完成请求参数
        completion_request: CompletionRequest,
    ) -> Result<serde_json::Value, CompletionError> {
        // 构建消息顺序（上下文、聊天历史、提示）
        let mut partial_history = vec![];

        // 如果有标准化文档，添加到历史中
        if let Some(docs) = completion_request.normalized_documents() {
            partial_history.push(docs);
        }

        // 扩展聊天历史
        partial_history.extend(completion_request.chat_history);

        // 使用前言初始化完整历史（如果不存在则为空）
        let mut full_history: Vec<Message> = completion_request
            .preamble
            .map_or_else(Vec::new, |preamble| vec![Message::system(&preamble)]);

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

        // 构建基础请求
        let mut request = json!({
            "model": self.model,
            "input": {
                "messages": full_history
            },
            "parameters": {
                "result_format": "message"
            }
        });

        // 添加温度参数（如果有）
        if let Some(temperature) = completion_request.temperature {
            request["parameters"]["temperature"] = json!(temperature);
        }

        // 添加工具（如果有）
        if !completion_request.tools.is_empty() {
            request["parameters"]["tools"] = json!(
                completion_request.tools
                    .into_iter()
                    .map(ToolDefinition::from)
                    .collect::<Vec<_>>()
            );
        }

        // 合并额外参数（如果有）
        if let Some(params) = completion_request.additional_params {
            // 将额外参数合并到 parameters 对象中
            if let Some(parameters) = request.get_mut("parameters") {
                *parameters = json_utils::merge(parameters.clone(), params);
            }
        }

        // 返回构建的请求
        Ok(request)
    }
}

// 为 CompletionModel 实现 completion::CompletionModel trait
impl<T> completion::CompletionModel for CompletionModel<T>
where
    T: HttpClientExt + Clone + std::fmt::Debug + Default + Send + 'static,
{
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
                gen_ai.provider.name = "qwen",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("input").and_then(|v| v.get("messages"))).unwrap_or_default(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            // 使用当前 span
            tracing::Span::current()
        };

        // 记录调试信息
        tracing::debug!("Qwen completion request: {request:?}");

        // 异步移动块
        async move {
            // 序列化请求体
            let body = serde_json::to_vec(&request)
                .map_err(|e| CompletionError::ResponseError(e.to_string()))?;

            // 构建请求
            let req = self.client
                .post("text-generation/generation")?
                .header("Content-Type", "application/json")
                .body(body)
                .map_err(|e| CompletionError::ResponseError(e.to_string()))?;

            // 发送请求
            let response = self.client.http_client.send::<_, Vec<u8>>(req).await?;

            // 检查响应状态
            if response.status().is_success() {
                // 获取响应文本
                let text = http_client::text(response).await?;
                // 记录调试信息
                tracing::debug!(target: "rig", "Qwen completion response: {text}");

                // 解析响应
                let api_response: CompletionResponse = serde_json::from_str(&text)
                    .map_err(|e| {
                        tracing::error!("Failed to parse response: {}. Response text: {}", e, text);
                        CompletionError::ResponseError(format!("Parse error: {}. Response: {}", e, text))
                    })?;

                // 获取当前 span
                let span = tracing::Span::current();
                // 记录请求 ID
                span.record("gen_ai.response.id", &api_response.request_id);
                // 记录输出消息
                span.record(
                    "gen_ai.output.messages",
                    serde_json::to_string(&api_response.output.choices).unwrap(),
                );
                // 记录输入令牌数
                span.record("gen_ai.usage.input_tokens", api_response.usage.input_tokens);
                // 记录输出令牌数
                span.record("gen_ai.usage.output_tokens", api_response.usage.output_tokens);

                // 转换响应
                api_response.try_into()
            } else {
                // 返回提供商错误
                Err(CompletionError::ProviderError(http_client::text(response).await?))
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

        // 启用增量输出（通义千问推荐设置）
        if let Some(parameters) = request.get_mut("parameters") {
            parameters["incremental_output"] = json!(true);
        }

        // 记录流式请求
        tracing::debug!("Qwen streaming request: {request:?}");

        // 序列化请求体
        let body = serde_json::to_vec(&request)
            .map_err(|e| CompletionError::ResponseError(e.to_string()))?;

        // 构建 HTTP 请求
        let req = self.client
            .post("text-generation/generation")?
            .header("Content-Type", "application/json")
            .header("X-DashScope-SSE", "enable")
            .body(body)
            .map_err(|e| CompletionError::ResponseError(e.to_string()))?;

        // 创建或获取追踪 span
        let span = if tracing::Span::current().is_disabled() {
            // 创建新的信息 span
            info_span!(
                target: "rig::completions",
                "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = "qwen",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("input").and_then(|v| v.get("messages"))).unwrap_or_default(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            // 使用当前 span
            tracing::Span::current()
        };

        // 使用追踪工具发送流式请求
        tracing::Instrument::instrument(send_qwen_streaming_request(self.client.http_client.clone(), req), span).await
    }
}

// ================================================================
// 流式处理
// ================================================================

// 流式工具调用结构体
#[derive(Deserialize, Debug, Clone)]
pub struct StreamingToolCall {
    // 工具调用 ID（可选）
    pub id: Option<String>,
    // 工具调用索引
    pub index: usize,
    // 工具类型（默认）
    #[serde(default)]
    pub r#type: ToolType,
    // 函数信息
    pub function: StreamingFunction,
}

// 流式函数结构体
#[derive(Deserialize, Debug, Clone)]
pub struct StreamingFunction {
    // 函数名称（可选）
    pub name: Option<String>,
    // 函数参数（字符串形式）
    #[serde(default)]
    pub arguments: String,
}

// 流式选择结构体
#[derive(Deserialize, Debug)]
struct StreamingChoice {
    // 消息内容（通义千问使用 message 而不是 delta）
    message: StreamingMessage,
    // 结束原因（可选）
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

// 流式消息结构体
#[derive(Deserialize, Debug)]
struct StreamingMessage {
    // 角色
    #[allow(dead_code)]
    role: String,
    // 内容（可选）
    #[serde(default)]
    content: Option<String>,
    // 推理内容（可选，用于 QwQ 等思考模型）
    #[serde(default)]
    reasoning_content: Option<String>,
    // 工具调用列表（默认为空，使用自定义反序列化）
    #[serde(default, deserialize_with = "json_utils::null_or_vec")]
    tool_calls: Vec<StreamingToolCall>,
}

// 流式完成块结构体
#[derive(Deserialize, Debug)]
struct StreamingCompletionChunk {
    // 输出结果
    output: StreamingOutput,
    // 使用情况统计（可选）
    usage: Option<Usage>,
}

// 流式输出结构体
#[derive(Deserialize, Debug)]
struct StreamingOutput {
    // 选择列表
    choices: Vec<StreamingChoice>,
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
        usage.input_tokens = self.usage.input_tokens as u64;
        // 设置输出令牌数
        usage.output_tokens = self.usage.output_tokens as u64;
        // 设置总令牌数
        usage.total_tokens = self.usage.total_tokens as u64;

        // 返回使用情况
        Some(usage)
    }
}

// 发送通义千问流式请求
pub async fn send_qwen_streaming_request<T>(
    // HTTP 客户端
    http_client: T,
    // 请求
    req: http::Request<Vec<u8>>,
) -> Result<
    // 返回流式完成响应
    crate::streaming::StreamingCompletionResponse<StreamingCompletionResponse>,
    // 完成错误
    CompletionError,
>
where
    T: HttpClientExt + Clone + 'static,
{
    // 获取当前追踪 span
    let span = tracing::Span::current();

    // 记录流式请求开始
    tracing::debug!("Starting Qwen streaming request with X-DashScope-SSE header");

    // 创建事件源（SSE 客户端）
    let mut event_source = GenericEventSource::new(http_client, req);

    tracing::debug!("Event source created successfully");

    // 创建流式响应流
    let stream = Box::pin(stream! {
        // 初始化最终使用情况统计
        let mut final_usage = Usage::new();
        // 初始化文本响应累积器
        let mut text_response = String::new();
        // 初始化推理内容累积器
        let mut reasoning_response = String::new();
        // 初始化工具调用映射（索引 -> (ID, 名称, 参数)）
        let mut calls: HashMap<usize, (String, String, String)> = HashMap::new();

        // 循环处理 SSE 事件
        while let Some(event_result) = event_source.next().await {
            match event_result {
                // SSE 连接打开事件
                Ok(Event::Open) => {
                    tracing::debug!("SSE connection opened");
                    continue;
                }
                // SSE 消息事件
                Ok(Event::Message(message)) => {
                    tracing::debug!("Received SSE message: {}", message.data);
                    
                    // 跳过空消息
                    if message.data.trim().is_empty() {
                        continue;
                    }

                    // 解析流式完成块
                    let parsed = serde_json::from_str::<StreamingCompletionChunk>(&message.data);
                    let Ok(data) = parsed else {
                        // 解析失败，记录调试信息并继续
                        let err = parsed.unwrap_err();
                        tracing::warn!("Couldn't parse SSE payload: {}. Data: {}", err, message.data);
                        continue;
                    };
                    
                    tracing::debug!("Successfully parsed streaming chunk");

                    // 处理第一个选择
                    if let Some(choice) = data.output.choices.first() {
                        let message = &choice.message;

                        // 处理推理内容（QwQ 等思考模型）
                        if let Some(reasoning) = &message.reasoning_content {
                            if !reasoning.is_empty() {
                                // 计算增量推理内容（incremental_output 模式下，API 返回累积文本）
                                let incremental_reasoning = if reasoning.len() >= reasoning_response.len() {
                                    // 当前推理内容长度 >= 累积长度，说明是累积文本，计算增量
                                    let incremental = if reasoning.starts_with(&reasoning_response) {
                                        &reasoning[reasoning_response.len()..]
                                    } else {
                                        // 如果内容不匹配，说明可能是新的响应，使用全部内容
                                        reasoning
                                    };
                                    // 更新累积推理内容
                                    reasoning_response = reasoning.clone();
                                    incremental
                                } else {
                                    // 当前推理内容长度 < 累积长度，说明这是增量文本片段
                                    reasoning_response.push_str(reasoning);
                                    reasoning
                                };
                                
                                // 只在有增量内容时生成推理内容结果
                                if !incremental_reasoning.is_empty() {
                                    yield Ok(crate::streaming::RawStreamingChoice::Reasoning {
                                        reasoning: incremental_reasoning.to_string(),
                                        id: None,
                                        signature: None,
                                    });
                                }
                            }
                        }

                        // 处理工具调用
                        if !message.tool_calls.is_empty() {
                            for tool_call in &message.tool_calls {
                                let function = &tool_call.function;

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
                                // 工具调用继续（无函数名但有参数）或参数增量
                                else if !function.arguments.is_empty() {
                                    // 获取现有工具调用
                                    if let Some((id, name, existing_args)) = calls.get(&tool_call.index) {
                                        // 计算增量参数（通义千问的 arguments 可能是累积的）
                                        let incremental_args = if function.arguments.starts_with(existing_args) {
                                            // arguments 是累积的，计算增量
                                            &function.arguments[existing_args.len()..]
                                        } else {
                                            // arguments 是新的增量片段
                                            &function.arguments
                                        };
                                        
                                        // 如果增量参数不为空，yield 为文本（这样用户能看到工具调用的参数流式输出）
                                        if !incremental_args.is_empty() {
                                            // 将工具调用参数作为文本流式输出，让用户能看到
                                            yield Ok(crate::streaming::RawStreamingChoice::Message(incremental_args.to_string()));
                                        }
                                        
                                        // 合并参数
                                        let combined = if function.arguments.starts_with(existing_args) {
                                            function.arguments.clone()
                                        } else {
                                            format!("{}{}", existing_args, function.arguments)
                                        };
                                        // 更新工具调用映射
                                        calls.insert(tool_call.index, (id.clone(), name.clone(), combined));
                                    } else {
                                        // 工具调用还没开始，但已经有参数了（可能函数名在前面的消息中）
                                        // 先 yield 参数作为文本
                                        if !function.arguments.is_empty() {
                                            yield Ok(crate::streaming::RawStreamingChoice::Message(function.arguments.clone()));
                                        }
                                        
                                        // 尝试从 ID 或索引创建工具调用映射
                                        let id = tool_call.id.clone().unwrap_or_else(|| format!("call_{}", tool_call.index));
                                        let name = function.name.clone().unwrap_or_else(|| String::from("unknown"));
                                        calls.insert(tool_call.index, (id, name, function.arguments.clone()));
                                    }
                                }
                                // 完整的工具调用（有 ID、函数名和完整参数）
                                else if let (Some(id), Some(name)) = (&tool_call.id, &function.name) {
                                    // 获取参数
                                    let arguments_str = function.arguments.clone();

                                    // 解析参数 JSON
                                    let Ok(arguments_json) = serde_json::from_str::<serde_json::Value>(&arguments_str) else {
                                        tracing::debug!("Couldn't parse tool call args '{}'", arguments_str);
                                        continue;
                                    };

                                    // 生成工具调用结果
                                    yield Ok(crate::streaming::RawStreamingChoice::ToolCall {
                                        id: id.clone(),
                                        name: name.clone(),
                                        arguments: arguments_json,
                                        call_id: None,
                                    });
                                }
                            }
                        }

                        // 处理文本内容
                        if let Some(content) = &message.content {
                            if !content.is_empty() {
                                // 计算增量文本内容
                                // 在 incremental_output=true 模式下，通义千问返回累积文本
                                // 我们需要计算增量：新内容 = 当前累积内容 - 之前累积内容
                                let incremental_text = if content.len() >= text_response.len() {
                                    // 当前内容长度 >= 累积长度，说明是累积文本，计算增量
                                    let incremental = if content.starts_with(&text_response) {
                                        &content[text_response.len()..]
                                    } else {
                                        // 如果内容不匹配，说明可能是新的响应，使用全部内容
                                        content
                                    };
                                    // 更新累积文本
                                    text_response = content.clone();
                                    incremental
                                } else {
                                    // 当前内容长度 < 累积长度，说明这是增量文本片段
                                    text_response.push_str(content);
                                    content
                                };
                                
                                // 只在有增量内容时生成消息结果
                                if !incremental_text.is_empty() {
                                    yield Ok(crate::streaming::RawStreamingChoice::Message(incremental_text.to_string()));
                                }
                            }
                        }
                    }

                    // 更新使用情况统计
                    if let Some(usage) = data.usage {
                        final_usage = usage.clone();
                    }
                }
                // 流结束错误
                Err(http_client::Error::StreamEnded) => {
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

        event_source.close();

        // 初始化工具调用列表
        let mut tool_calls = Vec::new();
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
            reasoning_content: if reasoning_response.is_empty() {
                None
            } else {
                Some(reasoning_response)
            },
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
// 测试模块
// ================================================================
#[cfg(test)]
mod tests {
    // 导入父模块的所有公开项
    use super::*;

    // 测试客户端构建器
    #[test]
    fn test_client_builder() {
        let client = Client::builder("test-api-key")
            .base_url("https://test.api.com")
            .build()
            .unwrap();

        assert_eq!(client.base_url, "https://test.api.com");
    }

    // 测试消息序列化
    #[test]
    fn test_message_serialization() {
        let message = Message::User {
            content: "Hello".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    // 测试工具调用序列化
    #[test]
    fn test_tool_call_serialization() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            index: 0,
            r#type: ToolType::Function,
            function: Function {
                name: "get_weather".to_string(),
                arguments: json!({"location": "Beijing"}),
            },
        };

        let json = serde_json::to_string(&tool_call).unwrap();
        assert!(json.contains("call_123"));
        assert!(json.contains("get_weather"));
    }

    // 测试完成响应反序列化
    #[test]
    fn test_completion_response_deserialization() {
        let data = r#"{
            "status_code": 200,
            "request_id": "test-request-id",
            "output": {
                "choices": [{
                    "finish_reason": "stop",
                    "message": {
                        "role": "assistant",
                        "content": "你好！"
                    }
                }]
            },
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "total_tokens": 15
            }
        }"#;

        let response: CompletionResponse = serde_json::from_str(data).unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 5);
    }
}
