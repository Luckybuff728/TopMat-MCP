// ================================================================
// OpenAI 完成 API
// ================================================================
// OpenAI 完成 API 模块
// 提供与 OpenAI 聊天完成 API 的交互功能

// 导入父模块的 API 错误响应、API 响应、客户端和流式完成响应
use super::{ApiErrorResponse, ApiResponse, Client, streaming::StreamingCompletionResponse};
// 导入完成模块的错误类型、完成请求和获取令牌使用量 trait
use crate::completion::{
    CompletionError, CompletionRequest as CoreCompletionRequest, GetTokenUsage,
};
// 导入消息模块的音频媒体类型、文档源类型、图像详情和 MIME 类型
use crate::message::{AudioMediaType, DocumentSourceKind, ImageDetail, MimeType};
// 导入一个或多个字符串解析函数
use crate::one_or_many::string_or_one_or_many;
// 导入遥测模块的提供商响应扩展和跨度组合器
use crate::telemetry::{ProviderResponseExt, SpanCombinator};
// 导入一个或多个、完成、JSON 工具和消息模块
use crate::{OneOrMany, completion, json_utils, message};
// 导入序列化和反序列化宏
use serde::{Deserialize, Serialize};
// 导入不可失败转换类型
use std::convert::Infallible;
// 导入格式化 trait
use std::fmt;
// 导入跟踪模块的工具和跨度
use tracing::{Instrument, info_span};

// 导入字符串解析 trait
use std::str::FromStr;

// 声明流式处理子模块
pub mod streaming;

/// `o4-mini-2025-04-16` 完成模型
// O4 Mini 2025-04-16 完成模型常量
pub const O4_MINI_2025_04_16: &str = "o4-mini-2025-04-16";
/// `o4-mini` 完成模型
// O4 Mini 完成模型常量
pub const O4_MINI: &str = "o4-mini";
/// `o3` 完成模型
// O3 完成模型常量
pub const O3: &str = "o3";
/// `o3-mini` 完成模型
// O3 Mini 完成模型常量
pub const O3_MINI: &str = "o3-mini";
/// `o3-mini-2025-01-31` 完成模型
// O3 Mini 2025-01-31 完成模型常量
pub const O3_MINI_2025_01_31: &str = "o3-mini-2025-01-31";
/// `o1-pro` 完成模型
// O1 Pro 完成模型常量
pub const O1_PRO: &str = "o1-pro";
/// `o1` 完成模型
// O1 完成模型常量
pub const O1: &str = "o1";
/// `o1-2024-12-17` 完成模型
// O1 2024-12-17 完成模型常量
pub const O1_2024_12_17: &str = "o1-2024-12-17";
/// `o1-preview` 完成模型
// O1 Preview 完成模型常量
pub const O1_PREVIEW: &str = "o1-preview";
/// `o1-preview-2024-09-12` 完成模型
// O1 Preview 2024-09-12 完成模型常量
pub const O1_PREVIEW_2024_09_12: &str = "o1-preview-2024-09-12";
/// `o1-mini` 完成模型
// O1 Mini 完成模型常量
pub const O1_MINI: &str = "o1-mini";
/// `o1-mini-2024-09-12` 完成模型
// O1 Mini 2024-09-12 完成模型常量
pub const O1_MINI_2024_09_12: &str = "o1-mini-2024-09-12";

/// `gpt-4.1-mini` 完成模型
// GPT-4.1 Mini 完成模型常量
pub const GPT_4_1_MINI: &str = "gpt-4.1-mini";
/// `gpt-4.1-nano` 完成模型
// GPT-4.1 Nano 完成模型常量
pub const GPT_4_1_NANO: &str = "gpt-4.1-nano";
/// `gpt-4.1-2025-04-14` 完成模型
// GPT-4.1 2025-04-14 完成模型常量
pub const GPT_4_1_2025_04_14: &str = "gpt-4.1-2025-04-14";
/// `gpt-4.1` 完成模型
// GPT-4.1 完成模型常量
pub const GPT_4_1: &str = "gpt-4.1";
/// `gpt-4.5-preview` 完成模型
// GPT-4.5 Preview 完成模型常量
pub const GPT_4_5_PREVIEW: &str = "gpt-4.5-preview";
/// `gpt-4.5-preview-2025-02-27` 完成模型
// GPT-4.5 Preview 2025-02-27 完成模型常量
pub const GPT_4_5_PREVIEW_2025_02_27: &str = "gpt-4.5-preview-2025-02-27";
/// `gpt-4o-2024-11-20` 完成模型（比 4o 更新）
// GPT-4o 2024-11-20 完成模型常量（比 4o 更新）
pub const GPT_4O_2024_11_20: &str = "gpt-4o-2024-11-20";
/// `gpt-4o` 完成模型
// GPT-4o 完成模型常量
pub const GPT_4O: &str = "gpt-4o";
/// `gpt-4o-mini` completion model
// GPT-4o Mini 完成模型常量
pub const GPT_4O_MINI: &str = "gpt-4o-mini";
/// `gpt-4o-2024-05-13` completion model
// GPT-4o 2024-05-13 完成模型常量
pub const GPT_4O_2024_05_13: &str = "gpt-4o-2024-05-13";
/// `gpt-4-turbo` completion model
// GPT-4 Turbo 完成模型常量
pub const GPT_4_TURBO: &str = "gpt-4-turbo";
/// `gpt-4-turbo-2024-04-09` completion model
// GPT-4 Turbo 2024-04-09 完成模型常量
pub const GPT_4_TURBO_2024_04_09: &str = "gpt-4-turbo-2024-04-09";
/// `gpt-4-turbo-preview` completion model
// GPT-4 Turbo Preview 完成模型常量
pub const GPT_4_TURBO_PREVIEW: &str = "gpt-4-turbo-preview";
/// `gpt-4-0125-preview` completion model
// GPT-4 0125 Preview 完成模型常量
pub const GPT_4_0125_PREVIEW: &str = "gpt-4-0125-preview";
/// `gpt-4-1106-preview` completion model
// GPT-4 1106 Preview 完成模型常量
pub const GPT_4_1106_PREVIEW: &str = "gpt-4-1106-preview";
/// `gpt-4-vision-preview` completion model
// GPT-4 Vision Preview 完成模型常量
pub const GPT_4_VISION_PREVIEW: &str = "gpt-4-vision-preview";
/// `gpt-4-1106-vision-preview` completion model
// GPT-4 1106 Vision Preview 完成模型常量
pub const GPT_4_1106_VISION_PREVIEW: &str = "gpt-4-1106-vision-preview";
/// `gpt-4` completion model
// GPT-4 完成模型常量
pub const GPT_4: &str = "gpt-4";
/// `gpt-4-0613` completion model
// GPT-4 0613 完成模型常量
pub const GPT_4_0613: &str = "gpt-4-0613";
/// `gpt-4-32k` completion model
// GPT-4 32k 完成模型常量
pub const GPT_4_32K: &str = "gpt-4-32k";
/// `gpt-4-32k-0613` completion model
// GPT-4 32k 0613 完成模型常量
pub const GPT_4_32K_0613: &str = "gpt-4-32k-0613";
/// `gpt-3.5-turbo` completion model
// GPT-3.5 Turbo 完成模型常量
pub const GPT_35_TURBO: &str = "gpt-3.5-turbo";
/// `gpt-3.5-turbo-0125` completion model
// GPT-3.5 Turbo 0125 完成模型常量
pub const GPT_35_TURBO_0125: &str = "gpt-3.5-turbo-0125";
/// `gpt-3.5-turbo-1106` completion model
// GPT-3.5 Turbo 1106 完成模型常量
pub const GPT_35_TURBO_1106: &str = "gpt-3.5-turbo-1106";
/// `gpt-3.5-turbo-instruct` completion model
// GPT-3.5 Turbo Instruct 完成模型常量
pub const GPT_35_TURBO_INSTRUCT: &str = "gpt-3.5-turbo-instruct";

// 从 API 错误响应转换为完成错误的实现
impl From<ApiErrorResponse> for CompletionError {
    // 将 API 错误响应转换为完成错误
    fn from(err: ApiErrorResponse) -> Self {
        // 创建提供商错误，使用错误消息
        CompletionError::ProviderError(err.message)
    }
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 使用 role 字段作为标签，将所有字段名转换为小写
#[serde(tag = "role", rename_all = "lowercase")]
// OpenAI 消息枚举
pub enum Message {
    // 系统消息，支持开发者别名
    #[serde(alias = "developer")]
    System {
        // 使用自定义反序列化器处理字符串或一个或多个内容
        #[serde(deserialize_with = "string_or_one_or_many")]
        content: OneOrMany<SystemContent>,
        // 如果名称为 None 则跳过序列化
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    // 用户消息
    User {
        // 使用自定义反序列化器处理字符串或一个或多个内容
        #[serde(deserialize_with = "string_or_one_or_many")]
        content: OneOrMany<UserContent>,
        // 如果名称为 None 则跳过序列化
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    // 助手消息
    Assistant {
        // 使用默认值和自定义反序列化器处理字符串或向量
        #[serde(default, deserialize_with = "json_utils::string_or_vec")]
        content: Vec<AssistantContent>,
        // 如果拒绝消息为 None 则跳过序列化
        #[serde(skip_serializing_if = "Option::is_none")]
        refusal: Option<String>,
        // 如果音频为 None 则跳过序列化
        #[serde(skip_serializing_if = "Option::is_none")]
        audio: Option<AudioAssistant>,
        // 如果名称为 None 则跳过序列化
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        // 工具调用列表，使用默认值、自定义反序列化器和跳过空向量序列化
        #[serde(
            default,
            deserialize_with = "json_utils::null_or_vec",
            skip_serializing_if = "Vec::is_empty"
        )]
        tool_calls: Vec<ToolCall>,
    },
    // 工具结果消息，重命名为 tool
    #[serde(rename = "tool")]
    ToolResult {
        // 工具调用 ID
        tool_call_id: String,
        // 工具结果内容
        content: OneOrMany<ToolResultContent>,
    },
}

// Message 枚举的实现
impl Message {
    // 创建系统消息的便捷方法
    pub fn system(content: &str) -> Self {
        // 创建系统消息，将内容转换为 SystemContent 并包装在 OneOrMany 中
        Message::System {
            content: OneOrMany::one(content.to_owned().into()),
            name: None,
        }
    }
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 音频助手结构体
pub struct AudioAssistant {
    // 音频助手 ID
    pub id: String,
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 系统内容结构体
pub struct SystemContent {
    // 使用默认值的系统内容类型
    #[serde(default)]
    pub r#type: SystemContentType,
    // 文本内容
    pub text: String,
}

// 派生 Default、Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
// 将所有字段名转换为小写
#[serde(rename_all = "lowercase")]
// 系统内容类型枚举
pub enum SystemContentType {
    // 默认的文本类型
    #[default]
    Text,
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 使用 type 字段作为标签，将所有字段名转换为小写
#[serde(tag = "type", rename_all = "lowercase")]
// 助手内容枚举
pub enum AssistantContent {
    // 文本内容
    Text { text: String },
    // 拒绝内容
    Refusal { refusal: String },
}

// 从 AssistantContent 转换为 completion::AssistantContent 的实现
impl From<AssistantContent> for completion::AssistantContent {
    // 将 AssistantContent 转换为 completion::AssistantContent
    fn from(value: AssistantContent) -> Self {
        // 匹配助手内容类型
        match value {
            // 文本内容转换为文本助手内容
            AssistantContent::Text { text } => completion::AssistantContent::text(text),
            // 拒绝内容转换为文本助手内容
            AssistantContent::Refusal { refusal } => completion::AssistantContent::text(refusal),
        }
    }
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 使用 type 字段作为标签，将所有字段名转换为小写
#[serde(tag = "type", rename_all = "lowercase")]
// 用户内容枚举
pub enum UserContent {
    // 文本内容
    Text {
        // 文本字符串
        text: String,
    },
    // 图像内容，重命名为 image_url
    #[serde(rename = "image_url")]
    Image {
        // 图像 URL
        image_url: ImageUrl,
    },
    // 音频内容
    Audio {
        // 输入音频
        input_audio: InputAudio,
    },
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 图像 URL 结构体
pub struct ImageUrl {
    // URL 字符串
    pub url: String,
    // 使用默认值的图像详情
    #[serde(default)]
    pub detail: ImageDetail,
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 输入音频结构体
pub struct InputAudio {
    // 音频数据字符串
    pub data: String,
    // 音频媒体类型
    pub format: AudioMediaType,
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 工具结果内容结构体
pub struct ToolResultContent {
    // 使用默认值的工具结果内容类型
    #[serde(default)]
    r#type: ToolResultContentType,
    // 文本内容
    pub text: String,
}

// 派生 Default、Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
// 将所有字段名转换为小写
#[serde(rename_all = "lowercase")]
// 工具结果内容类型枚举
pub enum ToolResultContentType {
    // 默认的文本类型
    #[default]
    Text,
}

// 为 ToolResultContent 实现 FromStr trait
impl FromStr for ToolResultContent {
    // 错误类型为不可失败类型
    type Err = Infallible;

    // 从字符串创建 ToolResultContent
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 将字符串转换为 ToolResultContent
        Ok(s.to_owned().into())
    }
}

// 为 ToolResultContent 实现 From<String> trait
impl From<String> for ToolResultContent {
    // 从字符串创建 ToolResultContent
    fn from(s: String) -> Self {
        // 创建工具结果内容，使用默认类型和提供的文本
        ToolResultContent {
            r#type: ToolResultContentType::default(),
            text: s,
        }
    }
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 工具调用结构体
pub struct ToolCall {
    // 工具调用 ID
    pub id: String,
    // 使用默认值的工具类型
    #[serde(default)]
    pub r#type: ToolType,
    // 函数信息
    pub function: Function,
}

// 派生 Default、Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
// 将所有字段名转换为小写
#[serde(rename_all = "lowercase")]
// 工具类型枚举
pub enum ToolType {
    // 默认的函数类型
    #[default]
    Function,
}

// 派生 Debug、Deserialize、Serialize 和 Clone trait
#[derive(Debug, Deserialize, Serialize, Clone)]
// 工具定义结构体
pub struct ToolDefinition {
    // 工具类型字符串
    pub r#type: String,
    // 完成工具定义
    pub function: completion::ToolDefinition,
}

// 从 completion::ToolDefinition 转换为 ToolDefinition 的实现
impl From<completion::ToolDefinition> for ToolDefinition {
    // 将完成工具定义转换为工具定义
    fn from(tool: completion::ToolDefinition) -> Self {
        // 创建工具定义，设置类型为函数
        Self {
            r#type: "function".into(),
            function: tool,
        }
    }
}

// 派生 Default、Clone、Debug、Deserialize、Serialize 和 PartialEq trait
#[derive(Default, Clone, Debug, Deserialize, Serialize, PartialEq)]
// 将所有字段名转换为蛇形命名法
#[serde(rename_all = "snake_case")]
// 工具选择枚举
pub enum ToolChoice {
    // 默认的自动选择
    #[default]
    Auto,
    // 不选择工具
    None,
    // 必须使用工具
    Required,
}

// 从 crate::message::ToolChoice 转换为 ToolChoice 的实现
impl TryFrom<crate::message::ToolChoice> for ToolChoice {
    // 错误类型为完成错误
    type Error = CompletionError;
    // 尝试将消息工具选择转换为工具选择
    fn try_from(value: crate::message::ToolChoice) -> Result<Self, Self::Error> {
        // 匹配消息工具选择类型
        let res = match value {
            // 特定工具选择不支持，返回错误
            message::ToolChoice::Specific { .. } => {
                return Err(CompletionError::ProviderError(
                    "Provider doesn't support only using specific tools".to_string(),
                ));
            }
            // 自动选择
            message::ToolChoice::Auto => Self::Auto,
            // 不选择工具
            message::ToolChoice::None => Self::None,
            // 必须使用工具
            message::ToolChoice::Required => Self::Required,
        };

        // 返回转换结果
        Ok(res)
    }
}

// 派生 Debug、Serialize、Deserialize、PartialEq 和 Clone trait
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// 函数结构体
pub struct Function {
    // 函数名称
    pub name: String,
    // 使用字符串化 JSON 序列化的参数
    #[serde(with = "json_utils::stringified_json")]
    pub arguments: serde_json::Value,
}

// 从 message::Message 转换为 Vec<Message> 的实现
impl TryFrom<message::Message> for Vec<Message> {
    // 错误类型为消息错误
    type Error = message::MessageError;

    // 尝试将消息转换为消息向量
    fn try_from(message: message::Message) -> Result<Self, Self::Error> {
        // 匹配消息类型
        match message {
            // 用户消息
            message::Message::User { content } => {
                // 将内容分为工具结果和其他内容
                let (tool_results, other_content): (Vec<_>, Vec<_>) = content
                    .into_iter()
                    .partition(|content| matches!(content, message::UserContent::ToolResult(_)));

                // 如果有工具结果和用户内容的消息，OpenAI 只会处理工具结果
                // 不太可能同时有两者
                if !tool_results.is_empty() {
                    // 处理工具结果
                    tool_results
                        .into_iter()
                        .map(|content| match content {
                            // 工具结果内容
                            message::UserContent::ToolResult(message::ToolResult {
                                id,
                                content,
                                .. 
                            }) => Ok::<_, message::MessageError>(Message::ToolResult {
                                // 设置工具调用 ID
                                tool_call_id: id,
                                // 映射工具结果内容
                                content: content.try_map(|content| match content {
                                    // 文本内容
                                    message::ToolResultContent::Text(message::Text { text }) => {
                                        Ok(text.into())
                                    }
                                    // 不支持非文本内容
                                    _ => Err(message::MessageError::ConversionError(
                                        "Tool result content does not support non-text".into(),
                                    )),
                                })?,
                            }),
                            _ => unreachable!(),
                        })
                        .collect::<Result<Vec<_>, _>>()
                } else {
                    // 处理其他用户内容
                    let other_content: Vec<UserContent> = other_content
                        .into_iter()
                        .map(|content| match content {
                            // 文本内容
                            message::UserContent::Text(message::Text { text }) => {
                                Ok(UserContent::Text { text })
                            }
                            // 图像内容
                            message::UserContent::Image(message::Image {
                                data,
                                detail,
                                media_type,
                                ..
                            }) => match data {
                                // URL 数据源
                                DocumentSourceKind::Url(url) => Ok(UserContent::Image {
                                    image_url: ImageUrl {
                                        url,
                                        detail: detail.unwrap_or_default(),
                                    },
                                }),
                                // Base64 数据源
                                DocumentSourceKind::Base64(data) => {
                                    // 构建数据 URL
                                    let url = format!(
                                        "data:{};base64,{}",
                                        media_type.map(|i| i.to_mime_type()).ok_or(
                                            message::MessageError::ConversionError(
                                                "OpenAI Image URI must have media type".into()
                                            )
                                        )?,
                                        data
                                    );

                                    // 获取图像详情
                                    let detail =
                                        detail.ok_or(message::MessageError::ConversionError(
                                            "OpenAI image URI must have image detail".into(),
                                        ))?;

                                    // 创建图像用户内容
                                    Ok(UserContent::Image {
                                        image_url: ImageUrl { url, detail },
                                    })
                                }
                                // 原始文件不支持
                                DocumentSourceKind::Raw(_) => {
                                    Err(message::MessageError::ConversionError(
                                        "Raw files not supported, encode as base64 first".into(),
                                    ))
                                }
                                // 未知文档源
                                DocumentSourceKind::Unknown => {
                                    Err(message::MessageError::ConversionError(
                                        "Document has no body".into(),
                                    ))
                                }
                                // 不支持的文档类型
                                doc => Err(message::MessageError::ConversionError(format!(
                                    "Unsupported document type: {doc:?}"
                                ))),
                            },
                            // 文档内容
                            message::UserContent::Document(message::Document { data, .. }) => {
                                // 只有 Base64 数据支持
                                if let DocumentSourceKind::Base64(text) = data {
                                    Ok(UserContent::Text { text })
                                } else {
                                    Err(message::MessageError::ConversionError(
                                        "Documents must be base64".into(),
                                    ))
                                }
                            }
                            // 音频内容
                            message::UserContent::Audio(message::Audio {
                                data: DocumentSourceKind::Base64(data),
                                media_type,
                                ..
                            }) => Ok(UserContent::Audio {
                                input_audio: InputAudio {
                                    data,
                                    // 设置音频格式，默认为 MP3
                                    format: match media_type {
                                        Some(media_type) => media_type,
                                        None => AudioMediaType::MP3,
                                    },
                                },
                            }),
                            // 不支持的工具结果格式
                            _ => Err(message::MessageError::ConversionError(
                                "Tool result is in unsupported format".into(),
                            )),
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    // 创建其他内容的一个或多个包装
                    let other_content = OneOrMany::many(other_content).expect(
                        "There must be other content here if there were no tool result content",
                    );

                    // 返回用户消息
                    Ok(vec![Message::User {
                        content: other_content,
                        name: None,
                    }])
                }
            }
            // 助手消息
            message::Message::Assistant { content, .. } => {
                // 将内容分为文本内容和工具调用
                let (text_content, tool_calls) = content.into_iter().fold(
                    (Vec::new(), Vec::new()),
                    |(mut texts, mut tools), content| {
                        // 匹配助手内容类型
                        match content {
                            // 文本内容
                            message::AssistantContent::Text(text) => texts.push(text),
                            // 工具调用
                            message::AssistantContent::ToolCall(tool_call) => tools.push(tool_call),
                            // 推理内容不支持
                            message::AssistantContent::Reasoning(_) => {
                                unimplemented!(
                                    "The OpenAI Completions API doesn't support reasoning!"
                                );
                            }
                        }
                        (texts, tools)
                    },
                );

                // `OneOrMany` 确保至少存在一个 `AssistantContent::Text` 或 `ToolCall`
                // 所以 `content` 或 `tool_calls` 中至少有一个有内容
                Ok(vec![Message::Assistant {
                    // 转换文本内容
                    content: text_content
                        .into_iter()
                        .map(|content| content.text.into())
                        .collect::<Vec<_>>(),
                    // 设置拒绝消息为 None
                    refusal: None,
                    // 设置音频为 None
                    audio: None,
                    // 设置名称为 None
                    name: None,
                    // 转换工具调用
                    tool_calls: tool_calls
                        .into_iter()
                        .map(|tool_call| tool_call.into())
                        .collect::<Vec<_>>(),
                }])
            }
        }
    }
}

// 从 message::ToolCall 转换为 ToolCall 的实现
impl From<message::ToolCall> for ToolCall {
    // 将消息工具调用转换为工具调用
    fn from(tool_call: message::ToolCall) -> Self {
        // 创建工具调用
        Self {
            // 设置工具调用 ID
            id: tool_call.id,
            // 设置工具类型为默认值
            r#type: ToolType::default(),
            // 创建函数信息
            function: Function {
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            },
        }
    }
}

// 从 ToolCall 转换为 message::ToolCall 的实现
impl From<ToolCall> for message::ToolCall {
    // 将工具调用转换为消息工具调用
    fn from(tool_call: ToolCall) -> Self {
        // 创建消息工具调用
        Self {
            // 设置工具调用 ID
            id: tool_call.id,
            // 设置调用 ID 为 None
            call_id: None,
            // 创建工具函数
            function: message::ToolFunction {
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            },
        }
    }
}

// 从 Message 转换为 message::Message 的实现
impl TryFrom<Message> for message::Message {
    // 错误类型为消息错误
    type Error = message::MessageError;

    // 尝试将消息转换为消息
    fn try_from(message: Message) -> Result<Self, Self::Error> {
        // 匹配消息类型
        Ok(match message {
            // 用户消息
            Message::User { content, .. } => message::Message::User {
                content: content.map(|content| content.into()),
            },
            // 助手消息
            Message::Assistant {
                content,
                tool_calls,
                ..
            } => {
                // 转换助手内容
                let mut content = content
                    .into_iter()
                    .map(|content| match content {
                        // 文本内容
                        AssistantContent::Text { text } => message::AssistantContent::text(text),

                        // TODO: 目前拒绝消息被转换为文本，但应该调查是否进行泛化
                        // 拒绝内容
                        AssistantContent::Refusal { refusal } => {
                            message::AssistantContent::text(refusal)
                        }
                    })
                    .collect::<Vec<_>>();

                // 扩展工具调用内容
                content.extend(
                    tool_calls
                        .into_iter()
                        .map(|tool_call| Ok(message::AssistantContent::ToolCall(tool_call.into())))
                        .collect::<Result<Vec<_>, _>>()?,
                );

                // 创建助手消息
                message::Message::Assistant {
                    id: None,
                    content: OneOrMany::many(content).map_err(|_| {
                        message::MessageError::ConversionError(
                            "Neither `content` nor `tool_calls` was provided to the Message"
                                .to_owned(),
                        )
                    })?,
                }
            }

            // 工具结果消息
            Message::ToolResult {
                tool_call_id,
                content,
            } => message::Message::User {
                content: OneOrMany::one(message::UserContent::tool_result(
                    tool_call_id,
                    content.map(|content| message::ToolResultContent::text(content.text)),
                )),
            },

            // 系统消息在转换消息时应该被移除，这只是一个临时措施
            // 以避免令人讨厌的错误处理或 panic 发生
            Message::System { content, .. } => message::Message::User {
                content: content.map(|content| message::UserContent::text(content.text)),
            },
        })
    }
}

// 从 UserContent 转换为 message::UserContent 的实现
impl From<UserContent> for message::UserContent {
    // 将用户内容转换为消息用户内容
    fn from(content: UserContent) -> Self {
        // 匹配用户内容类型
        match content {
            // 文本内容
            UserContent::Text { text } => message::UserContent::text(text),
            // 图像内容
            UserContent::Image { image_url } => {
                message::UserContent::image_url(image_url.url, None, Some(image_url.detail))
            }
            // 音频内容
            UserContent::Audio { input_audio } => {
                message::UserContent::audio(input_audio.data, Some(input_audio.format))
            }
        }
    }
}

// 从 String 转换为 UserContent 的实现
impl From<String> for UserContent {
    // 从字符串创建用户内容
    fn from(s: String) -> Self {
        UserContent::Text { text: s }
    }
}

// 为 UserContent 实现 FromStr trait
impl FromStr for UserContent {
    // 错误类型为不可失败类型
    type Err = Infallible;

    // 从字符串创建用户内容
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UserContent::Text {
            text: s.to_string(),
        })
    }
}

// 从 String 转换为 AssistantContent 的实现
impl From<String> for AssistantContent {
    // 从字符串创建助手内容
    fn from(s: String) -> Self {
        AssistantContent::Text { text: s }
    }
}

// 为 AssistantContent 实现 FromStr trait
impl FromStr for AssistantContent {
    // 错误类型为不可失败类型
    type Err = Infallible;

    // 从字符串创建助手内容
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(AssistantContent::Text {
            text: s.to_string(),
        })
    }
}
// 从 String 转换为 SystemContent 的实现
impl From<String> for SystemContent {
    // 从字符串创建系统内容
    fn from(s: String) -> Self {
        SystemContent {
            r#type: SystemContentType::default(),
            text: s,
        }
    }
}

// 为 SystemContent 实现 FromStr trait
impl FromStr for SystemContent {
    // 错误类型为不可失败类型
    type Err = Infallible;

    // 从字符串创建系统内容
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SystemContent {
            r#type: SystemContentType::default(),
            text: s.to_string(),
        })
    }
}

// 派生 Debug、Deserialize 和 Serialize trait
#[derive(Debug, Deserialize, Serialize)]
// 完成响应结构体
pub struct CompletionResponse {
    // 响应 ID
    pub id: String,
    // 对象类型
    pub object: String,
    // 创建时间戳
    pub created: u64,
    // 模型名称
    pub model: String,
    // 系统指纹（可选）
    pub system_fingerprint: Option<String>,
    // 选择列表
    pub choices: Vec<Choice>,
    // 使用情况（可选）
    pub usage: Option<Usage>,
}

// 从 CompletionResponse 转换为 completion::CompletionResponse 的实现
impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    // 错误类型为完成错误
    type Error = CompletionError;

    // 尝试将完成响应转换为完成响应
    fn try_from(response: CompletionResponse) -> Result<Self, Self::Error> {
        // 获取第一个选择
        let choice = response.choices.first().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no choices".to_owned())
        })?;

        // 处理消息内容
        let content = match &choice.message {
            // 助手消息
            Message::Assistant {
                content,
                tool_calls,
                ..
            } => {
                // 过滤和处理内容
                let mut content = content
                    .iter()
                    .filter_map(|c| {
                        // 获取文本内容
                        let s = match c {
                            AssistantContent::Text { text } => text,
                            AssistantContent::Refusal { refusal } => refusal,
                        };
                        // 如果内容不为空，则创建助手内容
                        if s.is_empty() {
                            None
                        } else {
                            Some(completion::AssistantContent::text(s))
                        }
                    })
                    .collect::<Vec<_>>();

                // 扩展工具调用内容
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
            // 其他消息类型不支持
            _ => Err(CompletionError::ResponseError(
                "Response did not contain a valid message or tool call".into(),
            )),
        }?;

        // 创建选择的一个或多个包装
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
                input_tokens: usage.prompt_tokens as u64,
                output_tokens: (usage.total_tokens - usage.prompt_tokens) as u64,
                total_tokens: usage.total_tokens as u64,
            })
            .unwrap_or_default();

        // 创建完成响应
        Ok(completion::CompletionResponse {
            choice,
            usage,
            raw_response: response,
        })
    }
}

// 为 CompletionResponse 实现 ProviderResponseExt trait
impl ProviderResponseExt for CompletionResponse {
    // 输出消息类型为选择
    type OutputMessage = Choice;
    // 使用情况类型为使用情况
    type Usage = Usage;

    // 获取响应 ID
    fn get_response_id(&self) -> Option<String> {
        Some(self.id.to_owned())
    }

    // 获取响应模型名称
    fn get_response_model_name(&self) -> Option<String> {
        Some(self.model.to_owned())
    }

    // 获取输出消息
    fn get_output_messages(&self) -> Vec<Self::OutputMessage> {
        self.choices.clone()
    }

    // 获取文本响应
    fn get_text_response(&self) -> Option<String> {
        // 获取最后一个选择的消息
        let Message::User { ref content, .. } = self.choices.last()?.message.clone() else {
            return None;
        };

        // 获取第一个文本内容
        let UserContent::Text { text } = content.first() else {
            return None;
        };

        Some(text)
    }

    // 获取使用情况
    fn get_usage(&self) -> Option<Self::Usage> {
        self.usage.clone()
    }
}

// 派生 Clone、Debug、Serialize 和 Deserialize trait
#[derive(Clone, Debug, Serialize, Deserialize)]
// 选择结构体
pub struct Choice {
    // 选择索引
    pub index: usize,
    // 消息
    pub message: Message,
    // 对数概率（可选）
    pub logprobs: Option<serde_json::Value>,
    // 完成原因
    pub finish_reason: String,
}

// 派生 Clone、Debug、Deserialize 和 Serialize trait
#[derive(Clone, Debug, Deserialize, Serialize)]
// 使用情况结构体
pub struct Usage {
    // 提示令牌数
    pub prompt_tokens: usize,
    // 总令牌数
    pub total_tokens: usize,
}

// Usage 结构体的实现
impl Usage {
    // 创建新的使用情况
    pub fn new() -> Self {
        Self {
            prompt_tokens: 0,
            total_tokens: 0,
        }
    }
}

// 为 Usage 实现 Default trait
impl Default for Usage {
    // 返回默认使用情况
    fn default() -> Self {
        Self::new()
    }
}

// 为 Usage 实现 fmt::Display trait
impl fmt::Display for Usage {
    // 格式化使用情况显示
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 解构使用情况
        let Usage {
            prompt_tokens,
            total_tokens,
        } = self;
        // 写入格式化的使用情况信息
        write!(
            f,
            "Prompt tokens: {prompt_tokens} Total tokens: {total_tokens}"
        )
    }
}

// 为 Usage 实现 GetTokenUsage trait
impl GetTokenUsage for Usage {
    // 获取令牌使用情况
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        // 创建新的使用情况
        let mut usage = crate::completion::Usage::new();
        // 设置输入令牌数
        usage.input_tokens = self.prompt_tokens as u64;
        // 设置输出令牌数
        usage.output_tokens = (self.total_tokens - self.prompt_tokens) as u64;
        // 设置总令牌数
        usage.total_tokens = self.total_tokens as u64;

        Some(usage)
    }
}

// 派生 Clone trait
#[derive(Clone)]
// 完成模型结构体
pub struct CompletionModel {
    // 客户端（包可见）
    pub(crate) client: Client,
    /// Name of the model (e.g.: gpt-3.5-turbo-1106)
    // 模型名称（例如：gpt-3.5-turbo-1106）
    pub model: String,
}

// CompletionModel 结构体的实现
impl CompletionModel {
    // 创建新的完成模型
    pub fn new(client: Client, model: &str) -> Self {
        Self {
            client,
            model: model.to_string(),
        }
    }

    // 转换为代理构建器
    pub fn into_agent_builder(self) -> crate::agent::AgentBuilder<Self> {
        crate::agent::AgentBuilder::new(self)
    }
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 完成请求结构体
pub struct CompletionRequest {
    // 模型名称
    model: String,
    // 消息列表
    messages: Vec<Message>,
    // 工具定义列表
    tools: Vec<ToolDefinition>,
    // 工具选择（可选）
    tool_choice: Option<ToolChoice>,
    // 温度参数（可选）
    temperature: Option<f64>,
    // 额外参数（扁平化）
    #[serde(flatten)]
    additional_params: Option<serde_json::Value>,
}

// 从 (String, CoreCompletionRequest) 转换为 CompletionRequest 的实现
impl TryFrom<(String, CoreCompletionRequest)> for CompletionRequest {
    // 错误类型为完成错误
    type Error = CompletionError;

    // 尝试将模型和核心完成请求转换为完成请求
    fn try_from((model, req): (String, CoreCompletionRequest)) -> Result<Self, Self::Error> {
        // 创建部分历史记录
        let mut partial_history = vec![];
        // 如果有标准化文档，添加到历史记录中
        if let Some(docs) = req.normalized_documents() {
            partial_history.push(docs);
        }
        // 解构核心完成请求
        let CoreCompletionRequest {
            preamble,
            chat_history,
            tools,
            temperature,
            additional_params,
            tool_choice,
            ..
        } = req;

        // 扩展部分历史记录
        partial_history.extend(chat_history);

        // 创建完整历史记录
        let mut full_history: Vec<Message> =
            preamble.map_or_else(Vec::new, |preamble| vec![Message::system(&preamble)]);

        // 转换并扩展其余历史记录
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
        let tool_choice = tool_choice.map(ToolChoice::try_from).transpose()?;

        // 创建完成请求
        let res = Self {
            model,
            messages: full_history,
            tools: tools
                .into_iter()
                .map(ToolDefinition::from)
                .collect::<Vec<_>>(),
            tool_choice,
            temperature,
            additional_params,
        };

        Ok(res)
    }
}

// 为 CompletionRequest 实现 ProviderRequestExt trait
impl crate::telemetry::ProviderRequestExt for CompletionRequest {
    // 输入消息类型为消息
    type InputMessage = Message;

    // 获取输入消息
    fn get_input_messages(&self) -> Vec<Self::InputMessage> {
        self.messages.clone()
    }

    // 获取系统提示
    fn get_system_prompt(&self) -> Option<String> {
        // 获取第一条消息
        let first_message = self.messages.first()?;

        // 检查是否为系统消息
        let Message::System { ref content, .. } = first_message.clone() else {
            return None;
        };

        // 获取系统内容的文本
        let SystemContent { text, .. } = content.first();

        Some(text)
    }

    // 获取提示
    fn get_prompt(&self) -> Option<String> {
        // 获取最后一条消息
        let last_message = self.messages.last()?;

        // 检查是否为用户消息
        let Message::User { ref content, .. } = last_message.clone() else {
            return None;
        };

        // 获取用户内容的文本
        let UserContent::Text { text } = content.first() else {
            return None;
        };

        Some(text)
    }

    // 获取模型名称
    fn get_model_name(&self) -> String {
        self.model.clone()
    }
}

// 为 CompletionModel 实现 completion::CompletionModel trait
impl completion::CompletionModel for CompletionModel {
    // 响应类型为完成响应
    type Response = CompletionResponse;
    // 流式响应类型为流式完成响应
    type StreamingResponse = StreamingCompletionResponse;

    // 完成方法（支持 worker 特性）
    #[cfg_attr(feature = "worker", worker::send)]
    async fn completion(
        &self,
        completion_request: CoreCompletionRequest,
    ) -> Result<completion::CompletionResponse<CompletionResponse>, CompletionError> {
        // 创建跟踪跨度
        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = "openai",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = &completion_request.preamble,
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

        // 转换完成请求
        let request = CompletionRequest::try_from((self.model.to_owned(), completion_request))?;
        // 记录模型输入
        span.record_model_input(&request.messages);

        // 异步执行请求
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
                // 解析 API 响应
                match serde_json::from_str::<ApiResponse<CompletionResponse>>(&t)? {
                    // 成功响应
                    ApiResponse::Ok(response) => {
                        // 获取当前跨度
                        let span = tracing::Span::current();
                        // 记录模型输出
                        span.record_model_output(&response.choices);
                        // 记录响应元数据
                        span.record_response_metadata(&response);
                        // 记录令牌使用情况
                        span.record_token_usage(&response.usage);
                        // 调试日志
                        tracing::debug!("OpenAI response: {response:?}");
                        // 转换响应
                        response.try_into()
                    }
                    // 错误响应
                    ApiResponse::Err(err) => Err(CompletionError::ProviderError(err.message)),
                }
            } else {
                // 非成功状态
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
        request: CoreCompletionRequest,
    ) -> Result<
        crate::streaming::StreamingCompletionResponse<Self::StreamingResponse>,
        CompletionError,
    > {
        // 调用完成模型的流式方法
        CompletionModel::stream(self, request).await
    }
}
