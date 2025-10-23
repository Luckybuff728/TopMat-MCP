//! OpenAI 响应 API。
//!
//! 默认情况下，创建完成客户端时使用的是此 API。
//!
//! 如果您想切换回常规完成 API，可以使用 `.completions_api()` 函数 - 请参见下面的示例：
//! ```rust
//! let openai_client = rig::providers::openai::Client::from_env();
//! let model = openai_client.completion_model("gpt-4o").completions_api();
//! ```

// OpenAI 响应 API 模块
// 提供与 OpenAI 响应 API 的交互功能，包括请求构建、响应处理和流式支持

// 导入 OpenAI 完成模块的工具选择类型
use super::completion::ToolChoice;
// 导入 OpenAI 客户端和响应 API 流式完成响应类型
use super::{Client, responses_api::streaming::StreamingCompletionResponse};
// 导入输入音频和系统内容类型
use super::{InputAudio, SystemContent};
// 导入完成模块的错误类型
use crate::completion::CompletionError;
// 导入 JSON 工具模块
use crate::json_utils;
// 导入消息模块的各种类型
use crate::message::{
    AudioMediaType, Document, DocumentMediaType, DocumentSourceKind, ImageDetail, MessageError,
    MimeType, Text,
};
// 导入一个或多个模块的字符串反序列化函数
use crate::one_or_many::string_or_one_or_many;

// 导入一个或多个、完成和消息模块
use crate::{OneOrMany, completion, message};
// 导入序列化和反序列化宏
use serde::{Deserialize, Serialize};
// 导入 JSON 映射和值类型
use serde_json::{Map, Value};
// 导入跟踪模块的工具化和信息跨度宏
use tracing::{Instrument, info_span};

// 导入标准库的转换和操作 trait
use std::convert::Infallible;
use std::ops::Add;
use std::str::FromStr;

// 导出流式模块
pub mod streaming;

/// The completion request type for OpenAI's Response API: <https://platform.openai.com/docs/api-reference/responses/create>
/// Intended to be derived from [`crate::completion::request::CompletionRequest`].
// OpenAI 响应 API 的完成请求类型
// 旨在从 crate::completion::request::CompletionRequest 派生
#[derive(Debug, Deserialize, Serialize, Clone)]
// 完成请求结构体
pub struct CompletionRequest {
    /// Message inputs
    // 消息输入
    pub input: OneOrMany<InputItem>,
    /// The model name
    // 模型名称
    pub model: String,
    /// Instructions (also referred to as preamble, although in other APIs this would be the "system prompt")
    // 指令（也称为前言，尽管在其他 API 中这将是"系统提示"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// The maximum number of output tokens.
    // 输出令牌的最大数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    /// Toggle to true for streaming responses.
    // 切换为 true 以获取流式响应
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// The temperature. Set higher (up to a max of 1.0) for more creative responses.
    // 温度。设置更高（最大 1.0）以获得更有创意的响应
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Whether the LLM should be forced to use a tool before returning a response.
    /// If none provided, the default option is "auto".
    // LLM 是否应该在返回响应之前强制使用工具
    // 如果未提供，默认选项是"auto"
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice>,
    /// The tools you want to use. Currently this is limited to functions, but will be expanded on in future.
    // 您想要使用的工具。目前这仅限于函数，但将来会扩展
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ResponsesToolDefinition>,
    /// Additional parameters
    // 额外参数
    #[serde(flatten)]
    pub additional_parameters: AdditionalParameters,
}

// CompletionRequest 结构体的实现
impl CompletionRequest {
    // 添加结构化输出配置
    pub fn with_structured_outputs<S>(mut self, schema_name: S, schema: serde_json::Value) -> Self
    where
        S: Into<String>,
    {
        // 设置结构化输出配置
        self.additional_parameters.text = Some(TextConfig::structured_output(schema_name, schema));

        // 返回修改后的请求
        self
    }

    // 添加推理配置
    pub fn with_reasoning(mut self, reasoning: Reasoning) -> Self {
        // 设置推理配置
        self.additional_parameters.reasoning = Some(reasoning);

        // 返回修改后的请求
        self
    }
}

/// An input item for [`CompletionRequest`].
// CompletionRequest 的输入项
#[derive(Debug, Deserialize, Serialize, Clone)]
// 输入项结构体
pub struct InputItem {
    /// The role of an input item/message.
    /// Input messages should be Some(Role::User), and output messages should be Some(Role::Assistant).
    /// Everything else should be None.
    // 输入项/消息的角色
    // 输入消息应该是 Some(Role::User)，输出消息应该是 Some(Role::Assistant)
    // 其他一切都应该是 None
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<Role>,
    /// The input content itself.
    // 输入内容本身
    #[serde(flatten)]
    input: InputContent,
}

/// Message roles. Used by OpenAI Responses API to determine who created a given message.
// 消息角色。由 OpenAI 响应 API 用于确定谁创建了给定消息
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
// 角色枚举
pub enum Role {
    User,
    Assistant,
    System,
}

/// The type of content used in an [`InputItem`]. Additionally holds data for each type of input content.
// InputItem 中使用的内容类型。此外还保存每种输入内容类型的数据
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
// 输入内容枚举
pub enum InputContent {
    Message(Message),
    Reasoning(OpenAIReasoning),
    FunctionCall(OutputFunctionCall),
    FunctionCallOutput(ToolResult),
}

// 派生 Debug、Deserialize、Serialize、Clone 和 PartialEq trait
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
// OpenAI 推理结构体
pub struct OpenAIReasoning {
    // 推理 ID
    id: String,
    // 推理摘要列表
    pub summary: Vec<ReasoningSummary>,
    // 加密内容（可选）
    pub encrypted_content: Option<String>,
    // 工具状态（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolStatus>,
}

// 派生 Debug、Deserialize、Serialize、Clone 和 PartialEq trait
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
// 推理摘要枚举
pub enum ReasoningSummary {
    SummaryText { text: String },
}

// ReasoningSummary 的实现
impl ReasoningSummary {
    // 创建新的推理摘要
    fn new(input: &str) -> Self {
        Self::SummaryText {
            text: input.to_string(),
        }
    }

    // 获取文本内容
    pub fn text(&self) -> String {
        let ReasoningSummary::SummaryText { text } = self;
        text.clone()
    }
}

/// A tool result.
// 工具结果
#[derive(Debug, Deserialize, Serialize, Clone)]
// 工具结果结构体
pub struct ToolResult {
    /// The call ID of a tool (this should be linked to the call ID for a tool call, otherwise an error will be received)
    // 工具的调用 ID（这应该与工具调用的调用 ID 链接，否则会收到错误）
    call_id: String,
    /// The result of a tool call.
    // 工具调用的结果
    output: String,
    /// The status of a tool call (if used in a completion request, this should always be Completed)
    // 工具调用的状态（如果在完成请求中使用，这应该始终是 Completed）
    status: ToolStatus,
}

// 为 Message 实现 From trait，转换为 InputItem
impl From<Message> for InputItem {
    // 从 Message 转换为 InputItem
    fn from(value: Message) -> Self {
        match value {
            // 用户消息
            Message::User { .. } => Self {
                role: Some(Role::User),
                input: InputContent::Message(value),
            },
            // 助手消息
            Message::Assistant { ref content, .. } => {
                // 检查是否包含推理内容
                let role = if content
                    .clone()
                    .iter()
                    .any(|x| matches!(x, AssistantContentType::Reasoning(_)))
                {
                    None
                } else {
                    Some(Role::Assistant)
                };
                Self {
                    role,
                    input: InputContent::Message(value),
                }
            }
            // 系统消息
            Message::System { .. } => Self {
                role: Some(Role::System),
                input: InputContent::Message(value),
            },
            // 工具结果消息
            Message::ToolResult {
                tool_call_id,
                output,
            } => Self {
                role: None,
                input: InputContent::FunctionCallOutput(ToolResult {
                    call_id: tool_call_id,
                    output,
                    status: ToolStatus::Completed,
                }),
            },
        }
    }
}

// 为 crate::completion::Message 实现 TryFrom trait，转换为 Vec<InputItem>
impl TryFrom<crate::completion::Message> for Vec<InputItem> {
    // 错误类型为完成错误
    type Error = CompletionError;

    // 从完成消息转换为输入项向量
    fn try_from(value: crate::completion::Message) -> Result<Self, Self::Error> {
        match value {
            // 用户消息
            crate::completion::Message::User { content } => {
                // 创建输入项向量
                let mut items = Vec::new();

                // 遍历用户内容
                for user_content in content {
                    match user_content {
                        // 文本内容
                        crate::message::UserContent::Text(Text { text }) => {
                            items.push(InputItem {
                                role: Some(Role::User),
                                input: InputContent::Message(Message::User {
                                    content: OneOrMany::one(UserContent::InputText { text }),
                                    name: None,
                                }),
                            });
                        }
                        // 工具结果内容
                        crate::message::UserContent::ToolResult(
                            crate::completion::message::ToolResult {
                                call_id,
                                content: tool_content,
                                ..
                            },
                        ) => {
                            // 遍历工具结果内容
                            for tool_result_content in tool_content {
                                // 匹配文本内容
                                let crate::completion::message::ToolResultContent::Text(Text {
                                    text,
                                }) = tool_result_content
                                else {
                                    return Err(CompletionError::ProviderError(
                                        "This thing only supports text!".to_string(),
                                    ));
                                };
                                // let output = serde_json::from_str(&text)?;
                                // 添加工具结果输入项
                                items.push(InputItem {
                                    role: None,
                                    input: InputContent::FunctionCallOutput(ToolResult {
                                        call_id: call_id
                                            .clone()
                                            .expect("The call ID of this tool should exist!"),
                                        output: text,
                                        status: ToolStatus::Completed,
                                    }),
                                });
                            }
                        }
                        // PDF 文档内容
                        crate::message::UserContent::Document(Document {
                            data,
                            media_type: Some(DocumentMediaType::PDF),
                            ..
                        }) => {
                            // 处理文档数据
                            let (file_data, file_url) = match data {
                                // Base64 编码的数据
                                DocumentSourceKind::Base64(data) => {
                                    (Some(format!("data:application/pdf;base64,{data}")), None)
                                }
                                // URL 数据
                                DocumentSourceKind::Url(url) => (None, Some(url)),
                                // 原始数据不支持
                                DocumentSourceKind::Raw(_) => {
                                    return Err(CompletionError::RequestError(
                                        "Raw file data not supported, encode as base64 first"
                                            .into(),
                                    ));
                                }
                                // 不支持的文档类型
                                doc => {
                                    return Err(CompletionError::RequestError(
                                        format!("Unsupported document type: {doc}").into(),
                                    ));
                                }
                            };

                            // 添加文档输入项
                            items.push(InputItem {
                                role: Some(Role::User),
                                input: InputContent::Message(Message::User {
                                    content: OneOrMany::one(UserContent::InputFile {
                                        file_data,
                                        file_url,
                                        filename: Some("document.pdf".to_string()),
                                    }),
                                    name: None,
                                }),
                            })
                        }
                        // todo: should we ensure this takes into account file size?
                        // Base64 文档内容
                        crate::message::UserContent::Document(Document {
                            data: DocumentSourceKind::Base64(text),
                            ..
                        }) => items.push(InputItem {
                            role: Some(Role::User),
                            input: InputContent::Message(Message::User {
                                content: OneOrMany::one(UserContent::InputText { text }),
                                name: None,
                            }),
                        }),
                        // 字符串文档内容
                        crate::message::UserContent::Document(Document {
                            data: DocumentSourceKind::String(text),
                            ..
                        }) => items.push(InputItem {
                            role: Some(Role::User),
                            input: InputContent::Message(Message::User {
                                content: OneOrMany::one(UserContent::InputText { text }),
                                name: None,
                            }),
                        }),
                        // 图像内容
                        crate::message::UserContent::Image(crate::message::Image {
                            data,
                            media_type,
                            detail,
                            ..
                        }) => {
                            // 处理图像数据
                            let url = match data {
                                // Base64 编码的图像数据
                                DocumentSourceKind::Base64(data) => {
                                    let media_type = if let Some(media_type) = media_type {
                                        media_type.to_mime_type().to_string()
                                    } else {
                                        String::new()
                                    };
                                    format!("data:{media_type};base64,{data}")
                                }
                                // URL 图像数据
                                DocumentSourceKind::Url(url) => url,
                                // 原始数据不支持
                                DocumentSourceKind::Raw(_) => {
                                    return Err(CompletionError::RequestError(
                                        "Raw file data not supported, encode as base64 first"
                                            .into(),
                                    ));
                                }
                                // 不支持的文档类型
                                doc => {
                                    return Err(CompletionError::RequestError(
                                        format!("Unsupported document type: {doc}").into(),
                                    ));
                                }
                            };
                            // 添加图像输入项
                            items.push(InputItem {
                                role: Some(Role::User),
                                input: InputContent::Message(Message::User {
                                    content: OneOrMany::one(UserContent::InputImage {
                                        image_url: url,
                                        detail: detail.unwrap_or_default(),
                                    }),
                                    name: None,
                                }),
                            });
                        }
                        // 不支持的消息类型
                        message => {
                            return Err(CompletionError::ProviderError(format!(
                                "Unsupported message: {message:?}"
                            )));
                        }
                    }
                }

                // 返回输入项列表
                Ok(items)
            }
            // 助手消息
            crate::completion::Message::Assistant { id, content } => {
                // 创建输入项向量
                let mut items = Vec::new();

                // 遍历助手内容
                for assistant_content in content {
                    match assistant_content {
                        // 文本内容
                        crate::message::AssistantContent::Text(Text { text }) => {
                            // 获取消息 ID
                            let id = id.as_ref().unwrap_or(&String::default()).clone();
                            // 添加文本输入项
                            items.push(InputItem {
                                role: Some(Role::Assistant),
                                input: InputContent::Message(Message::Assistant {
                                    content: OneOrMany::one(AssistantContentType::Text(
                                        AssistantContent::OutputText(Text { text }),
                                    )),
                                    id,
                                    name: None,
                                    status: ToolStatus::Completed,
                                }),
                            });
                        }
                        // 工具调用内容
                        crate::message::AssistantContent::ToolCall(crate::message::ToolCall {
                            id: tool_id,
                            call_id,
                            function,
                        }) => {
                            // 添加工具调用输入项
                            items.push(InputItem {
                                role: None,
                                input: InputContent::FunctionCall(OutputFunctionCall {
                                    arguments: function.arguments,
                                    call_id: call_id.expect("The tool call ID should exist!"),
                                    id: tool_id,
                                    name: function.name,
                                    status: ToolStatus::Completed,
                                }),
                            });
                        }
                        // 推理内容
                        crate::message::AssistantContent::Reasoning(
                            crate::message::Reasoning { id, reasoning },
                        ) => {
                            // 添加推理输入项
                            items.push(InputItem {
                                role: None,
                                input: InputContent::Reasoning(OpenAIReasoning {
                                    id: id
                                        .expect("An OpenAI-generated ID is required when using OpenAI reasoning items"),
                                    summary: reasoning.into_iter().map(|x| ReasoningSummary::new(&x)).collect(),
                                    encrypted_content: None,
                                    status: None,
                                }),
                            });
                        }
                    }
                }

                // 返回输入项列表
                Ok(items)
            }
        }
    }
}

// 为 OneOrMany<String> 实现 From trait，转换为 Vec<ReasoningSummary>
impl From<OneOrMany<String>> for Vec<ReasoningSummary> {
    // 从字符串列表转换为推理摘要列表
    fn from(value: OneOrMany<String>) -> Self {
        value.iter().map(|x| ReasoningSummary::new(x)).collect()
    }
}

/// The definition of a tool response, repurposed for OpenAI's Responses API.
// 工具响应的定义，重新用于 OpenAI 的响应 API
#[derive(Debug, Deserialize, Serialize, Clone)]
// 响应工具定义结构体
pub struct ResponsesToolDefinition {
    /// Tool name
    // 工具名称
    pub name: String,
    /// Parameters - this should be a JSON schema. Tools should additionally ensure an "additionalParameters" field has been added with the value set to false, as this is required if using OpenAI's strict mode (enabled by default).
    // 参数 - 这应该是一个 JSON 模式。工具还应该确保添加了一个"additionalParameters"字段，值设置为 false，因为如果使用 OpenAI 的严格模式（默认启用），这是必需的
    pub parameters: serde_json::Value,
    /// Whether to use strict mode. Enabled by default as it allows for improved efficiency.
    // 是否使用严格模式。默认启用，因为它允许提高效率
    pub strict: bool,
    /// The type of tool. This should always be "function".
    // 工具的类型。这应该始终是"function"
    #[serde(rename = "type")]
    pub kind: String,
    /// Tool description.
    // 工具描述
    pub description: String,
}

/// Recursively ensures all object schemas in a JSON schema have `additionalProperties: false`.
/// Nested arrays, schema $defs, object properties and enums should be handled through this method
/// This seems to be required by OpenAI's Responses API when using strict mode.
// 递归确保 JSON 模式中的所有对象模式都有 `additionalProperties: false`
// 嵌套数组、模式 $defs、对象属性和枚举应该通过此方法处理
// 这似乎是 OpenAI 响应 API 在使用严格模式时所需要的
fn add_props_false(schema: &mut serde_json::Value) {
    // 如果是对象类型
    if let Value::Object(obj) = schema {
        // 检查是否是对象模式
        let is_object_schema = obj.get("type") == Some(&Value::String("object".to_string()))
            || obj.contains_key("properties");

        // 如果是对象模式且没有 additionalProperties，则添加
        if is_object_schema && !obj.contains_key("additionalProperties") {
            obj.insert("additionalProperties".to_string(), Value::Bool(false));
        }

        // 处理 $defs
        if let Some(defs) = obj.get_mut("$defs")
            && let Value::Object(defs_obj) = defs
        {
            for (_, def_schema) in defs_obj.iter_mut() {
                add_props_false(def_schema);
            }
        }

        // 处理属性
        if let Some(properties) = obj.get_mut("properties")
            && let Value::Object(props) = properties
        {
            for (_, prop_value) in props.iter_mut() {
                add_props_false(prop_value);
            }
        }

        // 处理数组项
        if let Some(items) = obj.get_mut("items") {
            add_props_false(items);
        }

        // should handle Enums (anyOf/oneOf)
        // 应该处理枚举（anyOf/oneOf）
        for key in ["anyOf", "oneOf", "allOf"] {
            if let Some(variants) = obj.get_mut(key)
                && let Value::Array(variants_array) = variants
            {
                for variant in variants_array.iter_mut() {
                    add_props_false(variant);
                }
            }
        }
    }
}

// 为 completion::ToolDefinition 实现 From trait，转换为 ResponsesToolDefinition
impl From<completion::ToolDefinition> for ResponsesToolDefinition {
    // 从完成工具定义转换为响应工具定义
    fn from(value: completion::ToolDefinition) -> Self {
        // 解构完成工具定义
        let completion::ToolDefinition {
            name,
            mut parameters,
            description,
        } = value;

        // 添加 additionalProperties: false
        add_props_false(&mut parameters);

        // 创建响应工具定义
        Self {
            name,
            parameters,
            description,
            kind: "function".to_string(),
            strict: true,
        }
    }
}

/// Token usage.
/// Token usage from the OpenAI Responses API generally shows the input tokens and output tokens (both with more in-depth details) as well as a total tokens field.
// 令牌使用情况
// OpenAI 响应 API 的令牌使用情况通常显示输入令牌和输出令牌（都有更深入的详细信息）以及总令牌字段
#[derive(Clone, Debug, Serialize, Deserialize)]
// 响应使用情况结构体
pub struct ResponsesUsage {
    /// Input tokens
    // 输入令牌
    pub input_tokens: u64,
    /// In-depth detail on input tokens (cached tokens)
    // 输入令牌的深入详细信息（缓存令牌）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens_details: Option<InputTokensDetails>,
    /// Output tokens
    // 输出令牌
    pub output_tokens: u64,
    /// In-depth detail on output tokens (reasoning tokens)
    // 输出令牌的深入详细信息（推理令牌）
    pub output_tokens_details: OutputTokensDetails,
    /// Total tokens used (for a given prompt)
    // 使用的总令牌（对于给定提示）
    pub total_tokens: u64,
}

// ResponsesUsage 的实现
impl ResponsesUsage {
    /// Create a new ResponsesUsage instance
    // 创建新的 ResponsesUsage 实例
    pub(crate) fn new() -> Self {
        Self {
            input_tokens: 0,
            input_tokens_details: Some(InputTokensDetails::new()),
            output_tokens: 0,
            output_tokens_details: OutputTokensDetails::new(),
            total_tokens: 0,
        }
    }
}

// 为 ResponsesUsage 实现 Add trait
impl Add for ResponsesUsage {
    // 输出类型为自身
    type Output = Self;

    // 加法运算
    fn add(self, rhs: Self) -> Self::Output {
        // 计算输入令牌
        let input_tokens = self.input_tokens + rhs.input_tokens;
        // 计算输入令牌详细信息
        let input_tokens_details = self.input_tokens_details.map(|lhs| {
            if let Some(tokens) = rhs.input_tokens_details {
                lhs + tokens
            } else {
                lhs
            }
        });
        // 计算输出令牌
        let output_tokens = self.output_tokens + rhs.output_tokens;
        // 计算输出令牌详细信息
        let output_tokens_details = self.output_tokens_details + rhs.output_tokens_details;
        // 计算总令牌
        let total_tokens = self.total_tokens + rhs.total_tokens;
        // 返回新的使用情况
        Self {
            input_tokens,
            input_tokens_details,
            output_tokens,
            output_tokens_details,
            total_tokens,
        }
    }
}

/// In-depth details on input tokens.
// 输入令牌的深入详细信息
#[derive(Clone, Debug, Serialize, Deserialize)]
// 输入令牌详细信息结构体
pub struct InputTokensDetails {
    /// Cached tokens from OpenAI
    // 来自 OpenAI 的缓存令牌
    pub cached_tokens: u64,
}

// InputTokensDetails 的实现
impl InputTokensDetails {
    // 创建新的输入令牌详细信息
    pub(crate) fn new() -> Self {
        Self { cached_tokens: 0 }
    }
}

// 为 InputTokensDetails 实现 Add trait
impl Add for InputTokensDetails {
    // 输出类型为自身
    type Output = Self;
    // 加法运算
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cached_tokens: self.cached_tokens + rhs.cached_tokens,
        }
    }
}

/// In-depth details on output tokens.
// 输出令牌的深入详细信息
#[derive(Clone, Debug, Serialize, Deserialize)]
// 输出令牌详细信息结构体
pub struct OutputTokensDetails {
    /// Reasoning tokens
    // 推理令牌
    pub reasoning_tokens: u64,
}

// OutputTokensDetails 的实现
impl OutputTokensDetails {
    // 创建新的输出令牌详细信息
    pub(crate) fn new() -> Self {
        Self {
            reasoning_tokens: 0,
        }
    }
}

// 为 OutputTokensDetails 实现 Add trait
impl Add for OutputTokensDetails {
    // 输出类型为自身
    type Output = Self;
    // 加法运算
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            reasoning_tokens: self.reasoning_tokens + rhs.reasoning_tokens,
        }
    }
}

/// Occasionally, when using OpenAI's Responses API you may get an incomplete response. This struct holds the reason as to why it happened.
// 有时，在使用 OpenAI 的响应 API 时，您可能会收到不完整的响应。此结构体保存发生这种情况的原因
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
// 不完整详细信息原因结构体
pub struct IncompleteDetailsReason {
    /// The reason for an incomplete [`CompletionResponse`].
    // 不完整的 CompletionResponse 的原因
    pub reason: String,
}

/// A response error from OpenAI's Response API.
// 来自 OpenAI 响应 API 的响应错误
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
// 响应错误结构体
pub struct ResponseError {
    /// Error code
    // 错误代码
    pub code: String,
    /// Error message
    // 错误消息
    pub message: String,
}

/// A response object as an enum (ensures type validation)
// 作为枚举的响应对象（确保类型验证）
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
// 响应对象枚举
pub enum ResponseObject {
    Response,
}

/// The response status as an enum (ensures type validation)
// 作为枚举的响应状态（确保类型验证）
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
// 响应状态枚举
pub enum ResponseStatus {
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Queued,
    Incomplete,
}

/// Attempt to try and create a `NewCompletionRequest` from a model name and [`crate::completion::CompletionRequest`]
// 尝试从模型名称和 crate::completion::CompletionRequest 创建 NewCompletionRequest
impl TryFrom<(String, crate::completion::CompletionRequest)> for CompletionRequest {
    // 错误类型为完成错误
    type Error = CompletionError;
    // 从模型名称和完成请求转换为完成请求
    fn try_from(
        (model, req): (String, crate::completion::CompletionRequest),
    ) -> Result<Self, Self::Error> {
        // 构建输入数据
        let input = {
            // 创建部分历史记录向量
            let mut partial_history = vec![];
            // 如果有标准化文档，添加到历史记录
            if let Some(docs) = req.normalized_documents() {
                partial_history.push(docs);
            }
            // 扩展聊天历史记录
            partial_history.extend(req.chat_history);

            // Initialize full history with preamble (or empty if non-existent)
            // 使用前言初始化完整历史记录（如果不存在则为空）
            let mut full_history: Vec<InputItem> = Vec::new();

            // Convert and extend the rest of the history
            // 转换并扩展其余历史记录
            full_history.extend(
                partial_history
                    .into_iter()
                    .map(|x| <Vec<InputItem>>::try_from(x).unwrap())
                    .collect::<Vec<Vec<InputItem>>>()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<InputItem>>(),
            );

            // 返回完整历史记录
            full_history
        };

        // 将输入转换为 OneOrMany
        let input = OneOrMany::many(input)
            .expect("This should never panic - if it does, please file a bug report");

        // 获取流式参数
        let stream = req
            .additional_params
            .clone()
            .unwrap_or(Value::Null)
            .as_bool();

        // 处理额外参数
        let additional_parameters = if let Some(map) = req.additional_params {
            serde_json::from_value::<AdditionalParameters>(map).expect("Converting additional parameters to AdditionalParameters should never fail as every field is an Option")
        } else {
            // If there's no additional parameters, initialise an empty object
            // 如果没有额外参数，初始化一个空对象
            AdditionalParameters::default()
        };

        // 处理工具选择
        let tool_choice = req.tool_choice.map(ToolChoice::try_from).transpose()?;

        // 返回完成请求
        Ok(Self {
            input,
            model,
            instructions: req.preamble,
            max_output_tokens: req.max_tokens,
            stream,
            tool_choice,
            tools: req
                .tools
                .into_iter()
                .map(ResponsesToolDefinition::from)
                .collect(),
            temperature: req.temperature,
            additional_parameters,
        })
    }
}

/// The completion model struct for OpenAI's response API.
// OpenAI 响应 API 的完成模型结构体
#[derive(Clone)]
// 响应完成模型结构体
pub struct ResponsesCompletionModel {
    /// The OpenAI client
    // OpenAI 客户端
    pub(crate) client: Client,
    /// Name of the model (e.g.: gpt-3.5-turbo-1106)
    // 模型名称（例如：gpt-3.5-turbo-1106）
    pub model: String,
}

// ResponsesCompletionModel 的实现
impl ResponsesCompletionModel {
    /// Creates a new [`ResponsesCompletionModel`].
    // 创建新的 ResponsesCompletionModel
    pub fn new(client: Client, model: &str) -> Self {
        Self {
            client,
            model: model.to_string(),
        }
    }

    /// Use the Completions API instead of Responses.
    // 使用完成 API 而不是响应 API
    pub fn completions_api(self) -> crate::providers::openai::completion::CompletionModel {
        crate::providers::openai::completion::CompletionModel::new(self.client, &self.model)
    }

    /// Attempt to create a completion request from [`crate::completion::CompletionRequest`].
    // 尝试从 crate::completion::CompletionRequest 创建完成请求
    pub(crate) fn create_completion_request(
        &self,
        completion_request: crate::completion::CompletionRequest,
    ) -> Result<CompletionRequest, CompletionError> {
        // 转换完成请求
        let req = CompletionRequest::try_from((self.model.clone(), completion_request))?;

        // 返回请求
        Ok(req)
    }
}

/// The standard response format from OpenAI's Responses API.
// OpenAI 响应 API 的标准响应格式
#[derive(Clone, Debug, Serialize, Deserialize)]
// 完成响应结构体
pub struct CompletionResponse {
    /// The ID of a completion response.
    // 完成响应的 ID
    pub id: String,
    /// The type of the object.
    // 对象的类型
    pub object: ResponseObject,
    /// The time at which a given response has been created, in seconds from the UNIX epoch (01/01/1970 00:00:00).
    // 创建给定响应的时间，以 UNIX 纪元（1970年1月1日 00:00:00）以来的秒数表示
    pub created_at: u64,
    /// The status of the response.
    // 响应的状态
    pub status: ResponseStatus,
    /// Response error (optional)
    // 响应错误（可选）
    pub error: Option<ResponseError>,
    /// Incomplete response details (optional)
    // 不完整响应详细信息（可选）
    pub incomplete_details: Option<IncompleteDetailsReason>,
    /// System prompt/preamble
    // 系统提示/前言
    pub instructions: Option<String>,
    /// The maximum number of tokens the model should output
    // 模型应该输出的最大令牌数
    pub max_output_tokens: Option<u64>,
    /// The model name
    // 模型名称
    pub model: String,
    /// Token usage
    // 令牌使用情况
    pub usage: Option<ResponsesUsage>,
    /// The model output (messages, etc will go here)
    // 模型输出（消息等将在这里）
    pub output: Vec<Output>,
    /// Tools
    // 工具
    pub tools: Vec<ResponsesToolDefinition>,
    /// Additional parameters
    // 额外参数
    #[serde(flatten)]
    pub additional_parameters: AdditionalParameters,
}

/// Additional parameters for the completion request type for OpenAI's Response API: <https://platform.openai.com/docs/api-reference/responses/create>
/// Intended to be derived from [`crate::completion::request::CompletionRequest`].
// OpenAI 响应 API 完成请求类型的额外参数
// 旨在从 crate::completion::request::CompletionRequest 派生
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
// 额外参数结构体
pub struct AdditionalParameters {
    /// Whether or not a given model task should run in the background (ie a detached process).
    // 给定的模型任务是否应该在后台运行（即分离的进程）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    /// The text response format. This is where you would add structured outputs (if you want them).
    // 文本响应格式。这是您添加结构化输出的地方（如果您想要的话）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextConfig>,
    /// What types of extra data you would like to include. This is mostly useless at the moment since the types of extra data to add is currently unsupported, but this will be coming soon!
    // 您想要包含哪些类型的额外数据。目前这基本上是无用的，因为要添加的额外数据类型目前不受支持，但这很快就会到来！
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<Include>>,
    /// `top_p`. Mutually exclusive with the `temperature` argument.
    // `top_p`。与 `temperature` 参数互斥
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Whether or not the response should be truncated.
    // 响应是否应该被截断
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<TruncationStrategy>,
    /// The username of the user (that you want to use).
    // 用户的用户名（您想要使用的）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Any additional metadata you'd like to add. This will additionally be returned by the response.
    // 您想要添加的任何额外元数据。这将由响应额外返回
    #[serde(skip_serializing_if = "Map::is_empty", default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
    /// Whether or not you want tool calls to run in parallel.
    // 您是否希望工具调用并行运行
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Previous response ID. If you are not sending a full conversation, this can help to track the message flow.
    // 先前的响应 ID。如果您没有发送完整的对话，这可以帮助跟踪消息流
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    /// Add thinking/reasoning to your response. The response will be emitted as a list member of the `output` field.
    // 为您的响应添加思考/推理。响应将作为 `output` 字段的列表成员发出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
    /// The service tier you're using.
    // 您正在使用的服务层级
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<OpenAIServiceTier>,
    /// Whether or not to store the response for later retrieval by API.
    // 是否存储响应以供 API 稍后检索
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
}

// AdditionalParameters 的实现
impl AdditionalParameters {
    // 转换为 JSON 值
    pub fn to_json(self) -> serde_json::Value {
        serde_json::to_value(self).expect("this should never fail since a struct that impls Deserialize will always be valid JSON")
    }
}

/// The truncation strategy.
/// When using auto, if the context of this response and previous ones exceeds the model's context window size, the model will truncate the response to fit the context window by dropping input items in the middle of the conversation.
/// Otherwise, does nothing (and is disabled by default).
// 截断策略
// 使用自动时，如果此响应和先前响应的上下文超过模型的上下文窗口大小，模型将通过删除对话中间的输入项来截断响应以适应上下文窗口
// 否则，什么都不做（默认禁用）
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
// 截断策略枚举
pub enum TruncationStrategy {
    Auto,
    #[default]
    Disabled,
}

/// The model output format configuration.
/// You can either have plain text by default, or attach a JSON schema for the purposes of structured outputs.
// 模型输出格式配置
// 您可以默认使用纯文本，或者为了结构化输出的目的附加 JSON 模式
#[derive(Clone, Debug, Serialize, Deserialize)]
// 文本配置结构体
pub struct TextConfig {
    // 格式类型
    pub format: TextFormat,
}

// TextConfig 的实现
impl TextConfig {
    // 创建结构化输出配置
    pub(crate) fn structured_output<S>(name: S, schema: serde_json::Value) -> Self
    where
        S: Into<String>,
    {
        Self {
            format: TextFormat::JsonSchema(StructuredOutputsInput {
                name: name.into(),
                schema,
                strict: true,
            }),
        }
    }
}

/// The text format (contained by [`TextConfig`]).
/// You can either have plain text by default, or attach a JSON schema for the purposes of structured outputs.
// 文本格式（包含在 TextConfig 中）
// 您可以默认使用纯文本，或者为了结构化输出的目的附加 JSON 模式
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
// 文本格式枚举
pub enum TextFormat {
    JsonSchema(StructuredOutputsInput),
    #[default]
    Text,
}

/// The inputs required for adding structured outputs.
// 添加结构化输出所需的输入
#[derive(Clone, Debug, Serialize, Deserialize)]
// 结构化输出输入结构体
pub struct StructuredOutputsInput {
    /// The name of your schema.
    // 您的模式名称
    pub name: String,
    /// Your required output schema. It is recommended that you use the JsonSchema macro, which you can check out at <https://docs.rs/schemars/latest/schemars/trait.JsonSchema.html>.
    // 您需要的输出模式。建议您使用 JsonSchema 宏，您可以在 <https://docs.rs/schemars/latest/schemars/trait.JsonSchema.html> 查看
    pub schema: serde_json::Value,
    /// Enable strict output. If you are using your AI agent in a data pipeline or another scenario that requires the data to be absolutely fixed to a given schema, it is recommended to set this to true.
    // 启用严格输出。如果您在数据管道或另一个需要数据绝对固定到给定模式的场景中使用 AI 代理，建议将此设置为 true
    pub strict: bool,
}

/// Add reasoning to a [`CompletionRequest`].
// 为 CompletionRequest 添加推理
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
// 推理结构体
pub struct Reasoning {
    /// How much effort you want the model to put into thinking/reasoning.
    // 您希望模型在思考/推理上投入多少努力
    pub effort: Option<ReasoningEffort>,
    /// How much effort you want the model to put into writing the reasoning summary.
    // 您希望模型在编写推理摘要上投入多少努力
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<ReasoningSummaryLevel>,
}

// Reasoning 的实现
impl Reasoning {
    /// Creates a new Reasoning instantiation (with empty values).
    // 创建新的推理实例（使用空值）
    pub fn new() -> Self {
        Self {
            effort: None,
            summary: None,
        }
    }

    /// Adds reasoning effort.
    // 添加推理努力
    pub fn with_effort(mut self, reasoning_effort: ReasoningEffort) -> Self {
        self.effort = Some(reasoning_effort);

        self
    }

    /// Adds summary level (how detailed the reasoning summary will be).
    // 添加摘要级别（推理摘要的详细程度）
    pub fn with_summary_level(mut self, reasoning_summary_level: ReasoningSummaryLevel) -> Self {
        self.summary = Some(reasoning_summary_level);

        self
    }
}

/// The billing service tier that will be used. On auto by default.
// 将使用的计费服务层级。默认自动
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
// OpenAI 服务层级枚举
pub enum OpenAIServiceTier {
    #[default]
    Auto,
    Default,
    Flex,
}

/// The amount of reasoning effort that will be used by a given model.
// 给定模型将使用的推理努力量
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
// 推理努力枚举
pub enum ReasoningEffort {
    Minimal,
    Low,
    #[default]
    Medium,
    High,
}

/// The amount of effort that will go into a reasoning summary by a given model.
// 给定模型将在推理摘要中投入的努力量
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
// 推理摘要级别枚举
pub enum ReasoningSummaryLevel {
    #[default]
    Auto,
    Concise,
    Detailed,
}

/// Results to additionally include in the OpenAI Responses API.
/// Note that most of these are currently unsupported, but have been added for completeness.
// 在 OpenAI 响应 API 中额外包含的结果
// 请注意，这些大多数目前不受支持，但为了完整性而添加
#[derive(Clone, Debug, Deserialize, Serialize)]
// 包含枚举
pub enum Include {
    #[serde(rename = "file_search_call.results")]
    FileSearchCallResults,
    #[serde(rename = "message.input_image.image_url")]
    MessageInputImageImageUrl,
    #[serde(rename = "computer_call.output.image_url")]
    ComputerCallOutputOutputImageUrl,
    #[serde(rename = "reasoning.encrypted_content")]
    ReasoningEncryptedContent,
    #[serde(rename = "code_interpreter_call.outputs")]
    CodeInterpreterCallOutputs,
}

/// A currently non-exhaustive list of output types.
// 当前非穷尽的输出类型列表
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
// 输出枚举
pub enum Output {
    Message(OutputMessage),
    #[serde(alias = "function_call")]
    FunctionCall(OutputFunctionCall),
    Reasoning {
        id: String,
        summary: Vec<ReasoningSummary>,
    },
}

// 为 Output 实现 From trait，转换为 Vec<completion::AssistantContent>
impl From<Output> for Vec<completion::AssistantContent> {
    // 从输出转换为助手内容向量
    fn from(value: Output) -> Self {
        // 匹配输出类型
        let res: Vec<completion::AssistantContent> = match value {
            // 消息输出
            Output::Message(OutputMessage { content, .. }) => content
                .into_iter()
                .map(completion::AssistantContent::from)
                .collect(),
            // 函数调用输出
            Output::FunctionCall(OutputFunctionCall {
                id,
                arguments,
                call_id,
                name,
                ..
            }) => vec![completion::AssistantContent::tool_call_with_call_id(
                id, call_id, name, arguments,
            )],
            // 推理输出
            Output::Reasoning { id, summary } => {
                // 转换摘要为字符串向量
                let summary: Vec<String> = summary.into_iter().map(|x| x.text()).collect();

                vec![completion::AssistantContent::Reasoning(
                    message::Reasoning::multi(summary).with_id(id),
                )]
            }
        };

        // 返回结果
        res
    }
}

// 派生 Debug、Deserialize、Serialize、Clone 和 PartialEq trait
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 输出推理结构体
pub struct OutputReasoning {
    // 推理 ID
    id: String,
    // 推理摘要列表
    summary: Vec<ReasoningSummary>,
    // 工具状态
    status: ToolStatus,
}

/// An OpenAI Responses API tool call. A call ID will be returned that must be used when creating a tool result to send back to OpenAI as a message input, otherwise an error will be received.
// OpenAI 响应 API 工具调用。将返回一个调用 ID，在创建工具结果以作为消息输入发送回 OpenAI 时必须使用，否则将收到错误
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 输出函数调用结构体
pub struct OutputFunctionCall {
    // 工具调用 ID
    pub id: String,
    // 使用字符串化 JSON 的参数
    #[serde(with = "json_utils::stringified_json")]
    pub arguments: serde_json::Value,
    // 调用 ID
    pub call_id: String,
    // 函数名称
    pub name: String,
    // 工具状态
    pub status: ToolStatus,
}

/// The status of a given tool.
// 给定工具的状态
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
// 工具状态枚举
pub enum ToolStatus {
    InProgress,
    Completed,
    Incomplete,
}

/// An output message from OpenAI's Responses API.
// 来自 OpenAI 响应 API 的输出消息
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 输出消息结构体
pub struct OutputMessage {
    /// The message ID. Must be included when sending the message back to OpenAI
    // 消息 ID。在将消息发送回 OpenAI 时必须包含
    pub id: String,
    /// The role (currently only Assistant is available as this struct is only created when receiving an LLM message as a response)
    // 角色（目前只有 Assistant 可用，因为此结构体仅在接收 LLM 消息作为响应时创建）
    pub role: OutputRole,
    /// The status of the response
    // 响应的状态
    pub status: ResponseStatus,
    /// The actual message content
    // 实际的消息内容
    pub content: Vec<AssistantContent>,
}

/// The role of an output message.
// 输出消息的角色
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
// 输出角色枚举
pub enum OutputRole {
    Assistant,
}

// 为 ResponsesCompletionModel 实现 completion::CompletionModel trait
impl completion::CompletionModel for ResponsesCompletionModel {
    // 响应类型为完成响应
    type Response = CompletionResponse;
    // 流式响应类型为流式完成响应
    type StreamingResponse = StreamingCompletionResponse;

    // 完成方法（支持 worker 特性）
    #[cfg_attr(feature = "worker", worker::send)]
    async fn completion(
        &self,
        completion_request: crate::completion::CompletionRequest,
    ) -> Result<completion::CompletionResponse<Self::Response>, CompletionError> {
        // 创建跟踪跨度
        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = tracing::field::Empty,
                gen_ai.request.model = tracing::field::Empty,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = tracing::field::Empty,
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        // 记录提供商名称和模型
        span.record("gen_ai.provider.name", "openai");
        span.record("gen_ai.request.model", &self.model);
        // 创建完成请求
        let request = self.create_completion_request(completion_request)?;
        // 记录输入消息
        span.record(
            "gen_ai.input.messages",
            serde_json::to_string(&request.input)
                .expect("openai request to successfully turn into a JSON value"),
        );
        // 转换为 JSON 值
        let request_json = serde_json::to_value(request.clone())?;

        // 异步执行请求
        async move {
            // 发送 POST 请求到响应端点
            let response = self
                .client
                .post("/responses")
                .json(&request_json)
                .send()
                .await?;

            // 检查响应状态
            if response.status().is_success() {
                // 获取响应文本
                let t = response.text().await?;
                // 解析响应
                let response = serde_json::from_str::<Self::Response>(&t)?;
                // 获取当前跨度
                let span = tracing::Span::current();
                // 记录输出消息
                span.record(
                    "gen_ai.output.messages",
                    serde_json::to_string(&response.output).unwrap(),
                );
                // 记录响应 ID 和模型
                span.record("gen_ai.response.id", &response.id);
                span.record("gen_ai.response.model", &response.model);
                // 记录使用情况
                if let Some(ref usage) = response.usage {
                    span.record("gen_ai.usage.output_tokens", usage.output_tokens);
                    span.record("gen_ai.usage.input_tokens", usage.input_tokens);
                }
                // We need to call the event here to get the span to actually send anything
                // 我们需要在这里调用事件以使跨度实际发送任何内容
                tracing::info!("API successfully called");
                // 转换为完成响应
                response.try_into()
            } else {
                // 返回错误
                Err(CompletionError::ProviderError(response.text().await?))
            }
        }
        // 使用跨度进行工具化
        .instrument(span)
        .await
    }

    // 流式方法（支持 worker 特性）
    #[cfg_attr(feature = "worker", worker::send)]
    async fn stream(
        &self,
        request: crate::completion::CompletionRequest,
    ) -> Result<
        crate::streaming::StreamingCompletionResponse<Self::StreamingResponse>,
        CompletionError,
    > {
        // 调用流式方法
        Self::stream(self, request).await
    }
}

// 为 CompletionResponse 实现 TryFrom trait，转换为 completion::CompletionResponse
impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    // 错误类型为完成错误
    type Error = CompletionError;

    // 从完成响应转换为完成响应
    fn try_from(response: CompletionResponse) -> Result<Self, Self::Error> {
        // 检查输出是否为空
        if response.output.is_empty() {
            return Err(CompletionError::ResponseError(
                "Response contained no parts".to_owned(),
            ));
        }

        // 转换输出内容
        let content: Vec<completion::AssistantContent> = response
            .output
            .iter()
            .cloned()
            .flat_map(<Vec<completion::AssistantContent>>::from)
            .collect();

        // 创建选择
        let choice = OneOrMany::many(content).map_err(|_| {
            CompletionError::ResponseError(
                "Response contained no message or tool call (empty)".to_owned(),
            )
        })?;

        // 转换使用情况
        let usage = response
            .usage
            .as_ref()
            .map(|usage| completion::Usage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                total_tokens: usage.total_tokens,
            })
            .unwrap_or_default();

        // 返回完成响应
        Ok(completion::CompletionResponse {
            choice,
            usage,
            raw_response: response,
        })
    }
}

/// An OpenAI Responses API message.
// OpenAI 响应 API 消息
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
// 消息枚举
pub enum Message {
    // 系统消息
    #[serde(alias = "developer")]
    System {
        // 使用自定义反序列化器的系统内容
        #[serde(deserialize_with = "string_or_one_or_many")]
        content: OneOrMany<SystemContent>,
        // 名称（可选）
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    // 用户消息
    User {
        // 使用自定义反序列化器的用户内容
        #[serde(deserialize_with = "string_or_one_or_many")]
        content: OneOrMany<UserContent>,
        // 名称（可选）
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    // 助手消息
    Assistant {
        // 助手内容类型
        content: OneOrMany<AssistantContentType>,
        // ID（如果为空则跳过序列化）
        #[serde(skip_serializing_if = "String::is_empty")]
        id: String,
        // 名称（可选）
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        // 工具状态
        status: ToolStatus,
    },
    // 工具结果消息
    #[serde(rename = "tool")]
    ToolResult {
        // 工具调用 ID
        tool_call_id: String,
        // 输出
        output: String,
    },
}

/// The type of a tool result content item.
// 工具结果内容项的类型
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
// 工具结果内容类型枚举
pub enum ToolResultContentType {
    #[default]
    Text,
}

// Message 的实现
impl Message {
    // 创建系统消息
    pub fn system(content: &str) -> Self {
        Message::System {
            content: OneOrMany::one(content.to_owned().into()),
            name: None,
        }
    }
}

/// Text assistant content.
/// Note that the text type in comparison to the Completions API is actually `output_text` rather than `text`.
// 文本助手内容
// 请注意，与完成 API 相比，文本类型实际上是 `output_text` 而不是 `text`
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
// 助手内容枚举
pub enum AssistantContent {
    OutputText(Text),
    Refusal { refusal: String },
}

// 为 AssistantContent 实现 From trait，转换为 completion::AssistantContent
impl From<AssistantContent> for completion::AssistantContent {
    // 从助手内容转换为完成助手内容
    fn from(value: AssistantContent) -> Self {
        match value {
            // 拒绝内容
            AssistantContent::Refusal { refusal } => {
                completion::AssistantContent::Text(Text { text: refusal })
            }
            // 输出文本内容
            AssistantContent::OutputText(Text { text }) => {
                completion::AssistantContent::Text(Text { text })
            }
        }
    }
}

/// The type of assistant content.
// 助手内容的类型
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
// 助手内容类型枚举
pub enum AssistantContentType {
    Text(AssistantContent),
    ToolCall(OutputFunctionCall),
    Reasoning(OpenAIReasoning),
}

/// Different types of user content.
// 不同类型的用户内容
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
// 用户内容枚举
pub enum UserContent {
    // 输入文本
    InputText {
        text: String,
    },
    // 输入图像
    InputImage {
        image_url: String,
        #[serde(default)]
        detail: ImageDetail,
    },
    // 输入文件
    InputFile {
        #[serde(skip_serializing_if = "Option::is_none")]
        file_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_data: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
    // 音频
    Audio {
        input_audio: InputAudio,
    },
    // 工具结果
    #[serde(rename = "tool")]
    ToolResult {
        tool_call_id: String,
        output: String,
    },
}

// 为 message::Message 实现 TryFrom trait，转换为 Vec<Message>
impl TryFrom<message::Message> for Vec<Message> {
    // 错误类型为消息错误
    type Error = message::MessageError;

    // 从消息转换为消息向量
    fn try_from(message: message::Message) -> Result<Self, Self::Error> {
        match message {
            message::Message::User { content } => {
                let (tool_results, other_content): (Vec<_>, Vec<_>) = content
                    .into_iter()
                    .partition(|content| matches!(content, message::UserContent::ToolResult(_)));

                // If there are messages with both tool results and user content, openai will only
                //  handle tool results. It's unlikely that there will be both.
                if !tool_results.is_empty() {
                    tool_results
                        .into_iter()
                        .map(|content| match content {
                            message::UserContent::ToolResult(message::ToolResult {
                                call_id,
                                content,
                                ..
                            }) => Ok::<_, message::MessageError>(Message::ToolResult {
                                tool_call_id: call_id.expect("The tool call ID should exist"),
                                output: {
                                    let res = content.first();
                                    match res {
                                        completion::message::ToolResultContent::Text(Text {
                                            text,
                                        }) => text,
                                        _ => return  Err(MessageError::ConversionError("This API only currently supports text tool results".into()))
                                    }
                                },
                            }),
                            _ => unreachable!(),
                        })
                        .collect::<Result<Vec<_>, _>>()
                } else {
                    let other_content = other_content
                        .into_iter()
                        .map(|content| match content {
                            message::UserContent::Text(message::Text { text }) => {
                                Ok(UserContent::InputText { text })
                            }
                            message::UserContent::Image(message::Image {
                                data,
                                detail,
                                media_type,
                                ..
                            }) => {
                                let url = match data {
                                    DocumentSourceKind::Base64(data) => {
                                        let media_type = if let Some(media_type) = media_type {
                                            media_type.to_mime_type().to_string()
                                        } else {
                                            String::new()
                                        };
                                        format!("data:{media_type};base64,{data}")
                                    }
                                    DocumentSourceKind::Url(url) => url,
                                    DocumentSourceKind::Raw(_) => {
                                        return Err(MessageError::ConversionError(
                                            "Raw files not supported, encode as base64 first"
                                                .into(),
                                        ));
                                    }
                                    doc => {
                                        return Err(MessageError::ConversionError(format!(
                                            "Unsupported document type: {doc}"
                                        )));
                                    }
                                };

                                Ok(UserContent::InputImage {
                                    image_url: url,
                                    detail: detail.unwrap_or_default(),
                                })
                            }
                            message::UserContent::Document(message::Document {
                                media_type: Some(DocumentMediaType::PDF),
                                data,
                                ..
                            }) => {
                                let (file_data, file_url) = match data {
                                    DocumentSourceKind::Base64(data) => {
                                        (Some(format!("data:application/pdf;base64,{data}")), None)
                                    }
                                    DocumentSourceKind::Url(url) => (None, Some(url)),
                                    DocumentSourceKind::Raw(_) => {
                                        return Err(MessageError::ConversionError(
                                            "Raw files not supported, encode as base64 first"
                                                .into(),
                                        ));
                                    }
                                    doc => {
                                        return Err(MessageError::ConversionError(format!(
                                            "Unsupported document type: {doc}"
                                        )));
                                    }
                                };

                                Ok(UserContent::InputFile {
                                    file_url,
                                    file_data,
                                    filename: Some("document.pdf".into()),
                                })
                            }
                            message::UserContent::Document(message::Document {
                                data: DocumentSourceKind::Base64(text),
                                ..
                            }) => Ok(UserContent::InputText { text }),
                            message::UserContent::Audio(message::Audio {
                                data: DocumentSourceKind::Base64(data),
                                media_type,
                                ..
                            }) => Ok(UserContent::Audio {
                                input_audio: InputAudio {
                                    data,
                                    format: match media_type {
                                        Some(media_type) => media_type,
                                        None => AudioMediaType::MP3,
                                    },
                                },
                            }),
                            message::UserContent::Audio(_) => Err(MessageError::ConversionError(
                                "Audio must be base64 encoded data".into(),
                            )),
                            _ => unreachable!(),
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    let other_content = OneOrMany::many(other_content).expect(
                        "There must be other content here if there were no tool result content",
                    );

                    Ok(vec![Message::User {
                        content: other_content,
                        name: None,
                    }])
                }
            }
            message::Message::Assistant { content, id } => {
                let assistant_message_id = id;

                match content.first() {
                    crate::message::AssistantContent::Text(Text { text }) => {
                        Ok(vec![Message::Assistant {
                            id: assistant_message_id
                                .expect("The assistant message ID should exist"),
                            status: ToolStatus::Completed,
                            content: OneOrMany::one(AssistantContentType::Text(
                                AssistantContent::OutputText(Text { text }),
                            )),
                            name: None,
                        }])
                    }
                    crate::message::AssistantContent::ToolCall(crate::message::ToolCall {
                        id,
                        call_id,
                        function,
                    }) => Ok(vec![Message::Assistant {
                        content: OneOrMany::one(AssistantContentType::ToolCall(
                            OutputFunctionCall {
                                call_id: call_id.expect("The call ID should exist"),
                                arguments: function.arguments,
                                id,
                                name: function.name,
                                status: ToolStatus::Completed,
                            },
                        )),
                        id: assistant_message_id.expect("The assistant message ID should exist!"),
                        name: None,
                        status: ToolStatus::Completed,
                    }]),
                    crate::message::AssistantContent::Reasoning(crate::message::Reasoning {
                        id,
                        reasoning,
                    }) => Ok(vec![Message::Assistant {
                        content: OneOrMany::one(AssistantContentType::Reasoning(OpenAIReasoning {
                            id: id.expect("An OpenAI-generated ID is required when using OpenAI reasoning items"),
                            summary: reasoning.into_iter().map(|x| ReasoningSummary::SummaryText { text: x }).collect(),
                            encrypted_content: None,
                            status: Some(ToolStatus::Completed),
                        })),
                        id: assistant_message_id.expect("The assistant message ID should exist!"),
                        name: None,
                        status: (ToolStatus::Completed),
                    }]),
                }
            }
        }
    }
}

// 为 UserContent 实现 FromStr trait
impl FromStr for UserContent {
    // 错误类型为不可失败
    type Err = Infallible;

    // 从字符串转换为用户内容
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UserContent::InputText {
            text: s.to_string(),
        })
    }
}
