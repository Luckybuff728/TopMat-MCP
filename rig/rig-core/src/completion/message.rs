// 导入标准库的转换和字符串解析功能
use std::{convert::Infallible, str::FromStr};

// 导入 OneOrMany 类型
use crate::OneOrMany;
// 导入序列化和反序列化 trait
use serde::{Deserialize, Serialize};
// 导入错误处理宏
use thiserror::Error;

// 导入父模块的完成错误类型
use super::CompletionError;

// ================================================================
// 消息模型
// ================================================================

/// 一个有用的 trait，帮助将 `rig::completion::Message` 转换为您自己的消息类型。
///
/// 如果您不想创建独立函数，这特别有用，因为
/// 当尝试使用 `TryFrom<T>` 时，您通常会遇到孤儿规则，因为 Vec 在
/// 技术上是外部类型（它由 stdlib 拥有）。
// 定义消息转换 trait，用于将 Rig 消息转换为自定义消息类型
pub trait ConvertMessage: Sized + Send + Sync {
    // 定义转换错误类型，必须实现标准错误 trait 和 Send
    type Error: std::error::Error + Send;

    // 将消息转换为自定义消息类型的向量
    fn convert_from_message(message: Message) -> Result<Vec<Self>, Self::Error>;
}

/// 消息表示输入（用户）和输出（助手）的运行。
/// 每种消息类型（基于其 `role`）可以包含至少一个内容位，如文本、
/// 图像、音频、文档或工具相关信息。虽然每种消息类型可以包含
/// 多个内容，但最常见的是，每条消息您只会看到一种内容类型
/// （带有描述的图像等）。
///
/// 每个提供商负责将通用消息转换为其特定的提供商
///  type using `From` or `TryFrom` traits. Since not every provider supports every feature, the
///  conversion can be lossy (providing an image might be discarded for a non-image supporting
///  provider) though the message being converted back and forth should always be the same.
// 派生 Clone、Debug、Deserialize、Serialize 和 PartialEq trait
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 使用 serde 标签序列化，基于 role 字段区分变体，重命名为小写
#[serde(tag = "role", rename_all = "lowercase")]
// 定义消息枚举，表示用户和助手之间的交互
pub enum Message {
    /// User message containing one or more content types defined by `UserContent`.
    // 用户消息，包含一个或多个由 UserContent 定义的内容类型
    User { 
        // 用户内容，可以是单个或多个
        content: OneOrMany<UserContent> 
    },

    /// Assistant message containing one or more content types defined by `AssistantContent`.
    // 助手消息，包含一个或多个由 AssistantContent 定义的内容类型
    Assistant {
        // 可选的助手 ID
        id: Option<String>,
        // 助手内容，可以是单个或多个
        content: OneOrMany<AssistantContent>,
    },
}

/// Describes the content of a message, which can be text, a tool result, an image, audio, or
///  a document. Dependent on provider supporting the content type. Multimedia content is generally
///  base64 (defined by it's format) encoded but additionally supports urls (for some providers).
// 描述消息内容，可以是文本、工具结果、图像、音频或文档
// 依赖于提供商支持的内容类型。多媒体内容通常是 base64 编码的
// （由其格式定义），但也支持 URL（某些提供商）
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 使用 serde 标签序列化，基于 type 字段区分变体，重命名为小写
#[serde(tag = "type", rename_all = "lowercase")]
// 定义用户内容枚举
pub enum UserContent {
    // 文本内容
    Text(Text),
    // 工具结果内容
    ToolResult(ToolResult),
    // 图像内容
    Image(Image),
    // 音频内容
    Audio(Audio),
    // 视频内容
    Video(Video),
    // 文档内容
    Document(Document),
}

/// Describes responses from a provider which is either text or a tool call.
// 描述来自提供商的响应，可以是文本或工具调用
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 使用 serde 无标签序列化
#[serde(untagged)]
// 定义助手内容枚举
pub enum AssistantContent {
    // 文本内容
    Text(Text),
    // 工具调用内容
    ToolCall(ToolCall),
    // 推理内容
    Reasoning(Reasoning),
}

// 派生 Clone、Debug、Deserialize、Serialize 和 PartialEq trait
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 标记为非穷尽结构体，表示未来可能添加更多字段
#[non_exhaustive]
// 定义推理结构体，包含推理过程的步骤
pub struct Reasoning {
    // 可选的推理 ID
    pub id: Option<String>,
    // 推理步骤的字符串向量
    pub reasoning: Vec<String>,
}

// 为 Reasoning 实现方法
impl Reasoning {
    /// Create a new reasoning item from a single item
    // 从单个输入创建新的推理项
    pub fn new(input: &str) -> Self {
        Self {
            // 设置 ID 为 None
            id: None,
            // 将输入转换为字符串向量
            reasoning: vec![input.to_string()],
        }
    }

    // 设置可选的 ID
    pub fn optional_id(mut self, id: Option<String>) -> Self {
        // 设置 ID
        self.id = id;
        // 返回自身
        self
    }
    // 设置 ID
    pub fn with_id(mut self, id: String) -> Self {
        // 设置 ID 为 Some(id)
        self.id = Some(id);
        // 返回自身
        self
    }

    // 从多个输入创建推理项
    pub fn multi(input: Vec<String>) -> Self {
        Self {
            // 设置 ID 为 None
            id: None,
            // 设置推理步骤
            reasoning: input,
        }
    }
}

/// Tool result content containing information about a tool call and it's resulting content.
// 工具结果内容，包含工具调用信息及其结果内容
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ToolResult {
    // 工具结果 ID
    pub id: String,
    // 如果为 None 则跳过序列化
    #[serde(skip_serializing_if = "Option::is_none")]
    // 可选的调用 ID
    pub call_id: Option<String>,
    // 工具结果内容，可以是单个或多个
    pub content: OneOrMany<ToolResultContent>,
}

/// Describes the content of a tool result, which can be text or an image.
// 描述工具结果的内容，可以是文本或图像
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 使用 serde 标签序列化，基于 type 字段区分变体，重命名为小写
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolResultContent {
    // 文本内容
    Text(Text),
    // 图像内容
    Image(Image),
}

/// Describes a tool call with an id and function to call, generally produced by a provider.
// 描述工具调用，包含 ID 和要调用的函数，通常由提供商产生
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ToolCall {
    // 工具调用 ID
    pub id: String,
    // 可选的调用 ID
    pub call_id: Option<String>,
    // 要调用的函数
    pub function: ToolFunction,
}

/// Describes a tool function to call with a name and arguments, generally produced by a provider.
// 描述要调用的工具函数，包含名称和参数，通常由提供商产生
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ToolFunction {
    // 函数名称
    pub name: String,
    // 函数参数（JSON 值）
    pub arguments: serde_json::Value,
}

// ================================================================
// Base content models
// ================================================================

/// Basic text content.
// 基本文本内容
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Text {
    // 文本字符串
    pub text: String,
}

// 为 Text 实现方法
impl Text {
    // 获取文本内容的引用
    pub fn text(&self) -> &str {
        // 返回文本的引用
        &self.text
    }
}

// 为 Text 实现 Display trait
impl std::fmt::Display for Text {
    // 格式化文本内容用于显示
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 解构获取文本
        let Self { text } = self;
        // 写入格式化器
        write!(f, "{text}")
    }
}

/// Image content containing image data and metadata about it.
// 图像内容，包含图像数据及其元数据
#[derive(Default, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Image {
    // 文档源类型（URL、Base64、Raw 等）
    pub data: DocumentSourceKind,
    // 如果为 None 则跳过序列化
    #[serde(skip_serializing_if = "Option::is_none")]
    // 可选的图像媒体类型
    pub media_type: Option<ImageMediaType>,
    // 如果为 None 则跳过序列化
    #[serde(skip_serializing_if = "Option::is_none")]
    // 可选的图像细节级别
    pub detail: Option<ImageDetail>,
    // 展平序列化，如果为 None 则跳过
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    // 可选的附加参数
    pub additional_params: Option<serde_json::Value>,
}

// 为 Image 实现方法
impl Image {
    // 尝试将图像转换为 URL
    pub fn try_into_url(self) -> Result<String, MessageError> {
        // 匹配数据源类型
        match self.data {
            // 如果是 URL，直接返回
            DocumentSourceKind::Url(url) => Ok(url),
            // 如果是 Base64 数据
            DocumentSourceKind::Base64(data) => {
                // 检查是否有媒体类型
                let Some(media_type) = self.media_type else {
                    // 如果没有媒体类型，返回错误
                    return Err(MessageError::ConversionError(
                        "A media type is required to create a valid base64-encoded image URL"
                            .to_string(),
                    ));
                };

                // 创建 data URL 格式
                Ok(format!(
                    "data:image/{ty};base64,{data}",
                    ty = media_type.to_mime_type()
                ))
            }
            // 未知类型，返回错误
            unknown => Err(MessageError::ConversionError(format!(
                "Tried to convert unknown type to a URL: {unknown:?}"
            ))),
        }
    }
}

/// The kind of image source (to be used).
// 文档源类型（要使用的）
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
// 使用 serde 标签序列化，基于 type 字段区分变体，内容存储在 value 字段，重命名为驼峰命名
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
// 标记为非穷尽枚举，表示未来可能添加更多变体
#[non_exhaustive]
pub enum DocumentSourceKind {
    /// A file URL/URI.
    // 文件 URL/URI
    Url(String),
    /// A base-64 encoded string.
    // Base64 编码的字符串
    Base64(String),
    /// Raw bytes
    // 原始字节
    Raw(Vec<u8>),
    /// A string (or a string literal).
    // 字符串（或字符串字面量）
    String(String),
    // 默认变体
    #[default]
    /// An unknown file source (there's nothing there).
    // 未知的文件源（没有内容）
    Unknown,
}

// 为 DocumentSourceKind 实现方法
impl DocumentSourceKind {
    // 从 URL 字符串创建
    pub fn url(url: &str) -> Self {
        // 创建 URL 变体
        Self::Url(url.to_string())
    }

    // 从 Base64 字符串创建
    pub fn base64(base64_string: &str) -> Self {
        // 创建 Base64 变体
        Self::Base64(base64_string.to_string())
    }

    // 从原始字节创建
    pub fn raw(bytes: impl Into<Vec<u8>>) -> Self {
        // 创建 Raw 变体
        Self::Raw(bytes.into())
    }

    // 从字符串创建
    pub fn string(input: &str) -> Self {
        // 创建 String 变体
        Self::String(input.into())
    }

    // 创建未知类型
    pub fn unknown() -> Self {
        // 返回 Unknown 变体
        Self::Unknown
    }

    // 尝试获取内部字符串值
    pub fn try_into_inner(self) -> Option<String> {
        // 匹配变体类型
        match self {
            // URL 和 Base64 变体可以转换为字符串
            Self::Url(s) | Self::Base64(s) => Some(s),
            // 其他变体无法转换为字符串
            _ => None,
        }
    }
}

// 为 DocumentSourceKind 实现 Display trait
impl std::fmt::Display for DocumentSourceKind {
    // 格式化文档源类型用于显示
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 匹配不同的变体类型
        match self {
            // URL 类型：直接显示 URL
            Self::Url(string) => write!(f, "{string}"),
            // Base64 类型：直接显示 Base64 字符串
            Self::Base64(string) => write!(f, "{string}"),
            // String 类型：直接显示字符串
            Self::String(string) => write!(f, "{string}"),
            // Raw 类型：显示二进制数据标识
            Self::Raw(_) => write!(f, "<binary data>"),
            // Unknown 类型：显示未知标识
            Self::Unknown => write!(f, "<unknown>"),
        }
    }
}

/// Audio content containing audio data and metadata about it.
// 音频内容，包含音频数据及其元数据
#[derive(Default, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Audio {
    // 文档源类型（URL、Base64、Raw 等）
    pub data: DocumentSourceKind,
    // 如果为 None 则跳过序列化
    #[serde(skip_serializing_if = "Option::is_none")]
    // 可选的音频媒体类型
    pub media_type: Option<AudioMediaType>,
    // 展平序列化，如果为 None 则跳过
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    // 可选的附加参数
    pub additional_params: Option<serde_json::Value>,
}

/// Video content containing video data and metadata about it.
// 视频内容，包含视频数据及其元数据
#[derive(Default, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Video {
    // 文档源类型（URL、Base64、Raw 等）
    pub data: DocumentSourceKind,
    // 如果为 None 则跳过序列化
    #[serde(skip_serializing_if = "Option::is_none")]
    // 可选的视频媒体类型
    pub media_type: Option<VideoMediaType>,
    // 展平序列化，如果为 None 则跳过
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    // 可选的附加参数
    pub additional_params: Option<serde_json::Value>,
}

/// Document content containing document data and metadata about it.
// 文档内容，包含文档数据及其元数据
#[derive(Default, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Document {
    // 文档源类型（URL、Base64、Raw 等）
    pub data: DocumentSourceKind,
    // 如果为 None 则跳过序列化
    #[serde(skip_serializing_if = "Option::is_none")]
    // 可选的文档媒体类型
    pub media_type: Option<DocumentMediaType>,
    // 展平序列化，如果为 None 则跳过
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    // 可选的附加参数
    pub additional_params: Option<serde_json::Value>,
}

/// Describes the format of the content, which can be base64 or string.
// 描述内容的格式，可以是 base64 或字符串
#[derive(Default, Clone, Debug, Deserialize, Serialize, PartialEq)]
// 序列化时重命名为小写
#[serde(rename_all = "lowercase")]
pub enum ContentFormat {
    // 默认格式
    #[default]
    // Base64 编码格式
    Base64,
    // 字符串格式
    String,
    // URL 格式
    Url,
}

/// Helper enum that tracks the media type of the content.
// 辅助枚举，用于跟踪内容的媒体类型
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum MediaType {
    // 图像媒体类型
    Image(ImageMediaType),
    // 音频媒体类型
    Audio(AudioMediaType),
    // 文档媒体类型
    Document(DocumentMediaType),
    // 视频媒体类型
    Video(VideoMediaType),
}

/// Describes the image media type of the content. Not every provider supports every media type.
/// Convertible to and from MIME type strings.
// 描述内容的图像媒体类型。并非每个提供商都支持每种媒体类型
// 可与 MIME 类型字符串相互转换
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 序列化时重命名为小写
#[serde(rename_all = "lowercase")]
pub enum ImageMediaType {
    // JPEG 格式
    JPEG,
    // PNG 格式
    PNG,
    // GIF 格式
    GIF,
    // WEBP 格式
    WEBP,
    // HEIC 格式
    HEIC,
    // HEIF 格式
    HEIF,
    // SVG 格式
    SVG,
}

/// Describes the document media type of the content. Not every provider supports every media type.
/// Includes also programming languages as document types for providers who support code running.
/// Convertible to and from MIME type strings.
// 描述内容的文档媒体类型。并非每个提供商都支持每种媒体类型
// 还包括编程语言作为文档类型，适用于支持代码运行的提供商
// 可与 MIME 类型字符串相互转换
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 序列化时重命名为小写
#[serde(rename_all = "lowercase")]
pub enum DocumentMediaType {
    // PDF 文档
    PDF,
    // 纯文本文件
    TXT,
    // RTF 富文本格式
    RTF,
    // HTML 超文本标记语言
    HTML,
    // CSS 层叠样式表
    CSS,
    // Markdown 标记语言
    MARKDOWN,
    // CSV 逗号分隔值
    CSV,
    // XML 可扩展标记语言
    XML,
    // JavaScript 编程语言
    Javascript,
    // Python 编程语言
    Python,
}

// 为 DocumentMediaType 实现方法
impl DocumentMediaType {
    // 检查是否为编程语言类型
    pub fn is_code(&self) -> bool {
        // 匹配是否为 JavaScript 或 Python
        matches!(self, Self::Javascript | Self::Python)
    }
}

/// Describes the audio media type of the content. Not every provider supports every media type.
/// Convertible to and from MIME type strings.
// 描述内容的音频媒体类型。并非每个提供商都支持每种媒体类型
// 可与 MIME 类型字符串相互转换
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 序列化时重命名为小写
#[serde(rename_all = "lowercase")]
pub enum AudioMediaType {
    // WAV 音频格式
    WAV,
    // MP3 音频格式
    MP3,
    // AIFF 音频格式
    AIFF,
    // AAC 音频格式
    AAC,
    // OGG 音频格式
    OGG,
    // FLAC 音频格式
    FLAC,
}

/// Describes the video media type of the content. Not every provider supports every media type.
/// Convertible to and from MIME type strings.
// 描述内容的视频媒体类型。并非每个提供商都支持每种媒体类型
// 可与 MIME 类型字符串相互转换
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// 序列化时重命名为小写
#[serde(rename_all = "lowercase")]
pub enum VideoMediaType {
    // AVI 视频格式
    AVI,
    // MP4 视频格式
    MP4,
    // MPEG 视频格式
    MPEG,
}

/// Describes the detail of the image content, which can be low, high, or auto (open-ai specific).
// 描述图像内容的细节级别，可以是低、高或自动（OpenAI 特定）
#[derive(Default, Clone, Debug, Deserialize, Serialize, PartialEq)]
// 序列化时重命名为小写
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    // 低细节级别
    Low,
    // 高细节级别
    High,
    // 默认自动选择
    #[default]
    // 自动细节级别
    Auto,
}

// ================================================================
// Impl. for message models
// ================================================================

// 为 Message 实现方法
impl Message {
    /// This helper method is primarily used to extract the first string prompt from a `Message`.
    /// Since `Message` might have more than just text content, we need to find the first text.
    // 此辅助方法主要用于从消息中提取第一个字符串提示
    // 由于消息可能包含不仅仅是文本内容，我们需要找到第一个文本
    pub(crate) fn rag_text(&self) -> Option<String> {
        // 匹配消息类型
        match self {
            // 用户消息类型
            Message::User { content } => {
                // 遍历内容项
                for item in content.iter() {
                    // 如果是文本内容
                    if let UserContent::Text(Text { text }) = item {
                        // 返回文本的克隆
                        return Some(text.clone());
                    }
                }
                // 没有找到文本内容，返回 None
                None
            }
            // 其他消息类型返回 None
            _ => None,
        }
    }

    /// Helper constructor to make creating user messages easier.
    // 辅助构造函数，使创建用户消息更容易
    pub fn user(text: impl Into<String>) -> Self {
        Message::User {
            // 创建单个用户内容
            content: OneOrMany::one(UserContent::text(text)),
        }
    }

    /// Helper constructor to make creating assistant messages easier.
    // 辅助构造函数，使创建助手消息更容易
    pub fn assistant(text: impl Into<String>) -> Self {
        Message::Assistant {
            // 设置 ID 为 None
            id: None,
            // 创建单个助手内容
            content: OneOrMany::one(AssistantContent::text(text)),
        }
    }

    /// Helper constructor to make creating assistant messages easier.
    // 辅助构造函数，使创建带 ID 的助手消息更容易
    pub fn assistant_with_id(id: String, text: impl Into<String>) -> Self {
        Message::Assistant {
            // 设置指定的 ID
            id: Some(id),
            // 创建单个助手内容
            content: OneOrMany::one(AssistantContent::text(text)),
        }
    }

    /// Helper constructor to make creating tool result messages easier.
    // 辅助构造函数，使创建工具结果消息更容易
    pub fn tool_result(id: impl Into<String>, content: impl Into<String>) -> Self {
        Message::User {
            // 创建单个用户内容，包含工具结果
            content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                // 设置工具结果 ID
                id: id.into(),
                // 设置调用 ID 为 None
                call_id: None,
                // 创建单个工具结果内容
                content: OneOrMany::one(ToolResultContent::text(content)),
            })),
        }
    }

    // 创建带调用 ID 的工具结果消息
    pub fn tool_result_with_call_id(
        // 工具结果 ID
        id: impl Into<String>,
        // 可选的调用 ID
        call_id: Option<String>,
        // 内容
        content: impl Into<String>,
    ) -> Self {
        Message::User {
            // 创建单个用户内容，包含工具结果
            content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                // 设置工具结果 ID
                id: id.into(),
                // 设置调用 ID
                call_id,
                // 创建单个工具结果内容
                content: OneOrMany::one(ToolResultContent::text(content)),
            })),
        }
    }
}

// 为 UserContent 实现方法
impl UserContent {
    /// Helper constructor to make creating user text content easier.
    // 辅助构造函数，使创建用户文本内容更容易
    pub fn text(text: impl Into<String>) -> Self {
        // 创建文本内容
        UserContent::Text(text.into().into())
    }

    /// Helper constructor to make creating user image content easier.
    // 辅助构造函数，使创建用户图像内容更容易（Base64）
    pub fn image_base64(
        // Base64 编码的图像数据
        data: impl Into<String>,
        // 可选的图像媒体类型
        media_type: Option<ImageMediaType>,
        // 可选的图像细节级别
        detail: Option<ImageDetail>,
    ) -> Self {
        UserContent::Image(Image {
            // 设置数据为 Base64 类型
            data: DocumentSourceKind::Base64(data.into()),
            // 设置媒体类型
            media_type,
            // 设置细节级别
            detail,
            // 设置附加参数为 None
            additional_params: None,
        })
    }

    /// Helper constructor to make creating user image content from raw unencoded bytes easier.
    // 辅助构造函数，使从原始未编码字节创建用户图像内容更容易
    pub fn image_raw(
        // 原始字节数据
        data: impl Into<Vec<u8>>,
        // 可选的图像媒体类型
        media_type: Option<ImageMediaType>,
        // 可选的图像细节级别
        detail: Option<ImageDetail>,
    ) -> Self {
        UserContent::Image(Image {
            // 设置数据为 Raw 类型
            data: DocumentSourceKind::Raw(data.into()),
            // 设置媒体类型
            media_type,
            // 设置细节级别
            detail,
            // 使用默认值填充其余字段
            ..Default::default()
        })
    }

    /// Helper constructor to make creating user image content easier.
    // 辅助构造函数，使创建用户图像内容更容易（URL）
    pub fn image_url(
        // 图像 URL
        url: impl Into<String>,
        // 可选的图像媒体类型
        media_type: Option<ImageMediaType>,
        // 可选的图像细节级别
        detail: Option<ImageDetail>,
    ) -> Self {
        UserContent::Image(Image {
            // 设置数据为 URL 类型
            data: DocumentSourceKind::Url(url.into()),
            // 设置媒体类型
            media_type,
            // 设置细节级别
            detail,
            // 设置附加参数为 None
            additional_params: None,
        })
    }

    /// Helper constructor to make creating user audio content easier.
    // 辅助构造函数，使创建用户音频内容更容易（Base64）
    pub fn audio(data: impl Into<String>, media_type: Option<AudioMediaType>) -> Self {
        UserContent::Audio(Audio {
            // 设置数据为 Base64 类型
            data: DocumentSourceKind::Base64(data.into()),
            // 设置媒体类型
            media_type,
            // 设置附加参数为 None
            additional_params: None,
        })
    }

    /// Helper constructor to make creating user audio content from raw unencoded bytes easier.
    // 辅助构造函数，使从原始未编码字节创建用户音频内容更容易
    pub fn audio_raw(data: impl Into<Vec<u8>>, media_type: Option<AudioMediaType>) -> Self {
        UserContent::Audio(Audio {
            // 设置数据为 Raw 类型
            data: DocumentSourceKind::Raw(data.into()),
            // 设置媒体类型
            media_type,
            // 使用默认值填充其余字段
            ..Default::default()
        })
    }

    /// Helper to create an audio resource from a URL
    // 辅助函数，从 URL 创建音频资源
    pub fn audio_url(url: impl Into<String>, media_type: Option<AudioMediaType>) -> Self {
        UserContent::Audio(Audio {
            // 设置数据为 URL 类型
            data: DocumentSourceKind::Url(url.into()),
            // 设置媒体类型
            media_type,
            // 使用默认值填充其余字段
            ..Default::default()
        })
    }

    /// Helper constructor to make creating user document content easier.
    /// This creates a document that assumes the data being passed in is a raw string.
    // 辅助构造函数，使创建用户文档内容更容易
    // 这创建一个文档，假设传入的数据是原始字符串
    pub fn document(data: impl Into<String>, media_type: Option<DocumentMediaType>) -> Self {
        // 转换数据为字符串
        let data: String = data.into();
        UserContent::Document(Document {
            // 设置数据为字符串类型
            data: DocumentSourceKind::string(&data),
            // 设置媒体类型
            media_type,
            // 设置附加参数为 None
            additional_params: None,
        })
    }

    /// Helper to create a document from raw unencoded bytes
    // 辅助函数，从原始未编码字节创建文档
    pub fn document_raw(data: impl Into<Vec<u8>>, media_type: Option<DocumentMediaType>) -> Self {
        UserContent::Document(Document {
            // 设置数据为 Raw 类型
            data: DocumentSourceKind::Raw(data.into()),
            // 设置媒体类型
            media_type,
            // 使用默认值填充其余字段
            ..Default::default()
        })
    }

    /// Helper to create a document from a URL
    // 辅助函数，从 URL 创建文档
    pub fn document_url(url: impl Into<String>, media_type: Option<DocumentMediaType>) -> Self {
        UserContent::Document(Document {
            // 设置数据为 URL 类型
            data: DocumentSourceKind::Url(url.into()),
            // 设置媒体类型
            media_type,
            // 使用默认值填充其余字段
            ..Default::default()
        })
    }

    /// Helper constructor to make creating user tool result content easier.
    // 辅助构造函数，使创建用户工具结果内容更容易
    pub fn tool_result(id: impl Into<String>, content: OneOrMany<ToolResultContent>) -> Self {
        UserContent::ToolResult(ToolResult {
            // 设置工具结果 ID
            id: id.into(),
            // 设置调用 ID 为 None
            call_id: None,
            // 设置内容
            content,
        })
    }

    /// Helper constructor to make creating user tool result content easier.
    // 辅助构造函数，使创建带调用 ID 的用户工具结果内容更容易
    pub fn tool_result_with_call_id(
        // 工具结果 ID
        id: impl Into<String>,
        // 调用 ID
        call_id: String,
        // 内容
        content: OneOrMany<ToolResultContent>,
    ) -> Self {
        UserContent::ToolResult(ToolResult {
            // 设置工具结果 ID
            id: id.into(),
            // 设置调用 ID
            call_id: Some(call_id),
            // 设置内容
            content,
        })
    }
}

// 为 AssistantContent 实现方法
impl AssistantContent {
    /// Helper constructor to make creating assistant text content easier.
    // 辅助构造函数，使创建助手文本内容更容易
    pub fn text(text: impl Into<String>) -> Self {
        // 创建文本内容
        AssistantContent::Text(text.into().into())
    }

    /// Helper constructor to make creating assistant tool call content easier.
    // 辅助构造函数，使创建助手工具调用内容更容易
    pub fn tool_call(
        // 工具调用 ID
        id: impl Into<String>,
        // 函数名称
        name: impl Into<String>,
        // 函数参数（JSON 值）
        arguments: serde_json::Value,
    ) -> Self {
        AssistantContent::ToolCall(ToolCall {
            // 设置工具调用 ID
            id: id.into(),
            // 设置调用 ID 为 None
            call_id: None,
            // 设置工具函数
            function: ToolFunction {
                // 设置函数名称
                name: name.into(),
                // 设置函数参数
                arguments,
            },
        })
    }

    // 创建带调用 ID 的工具调用内容
    pub fn tool_call_with_call_id(
        // 工具调用 ID
        id: impl Into<String>,
        // 调用 ID
        call_id: String,
        // 函数名称
        name: impl Into<String>,
        // 函数参数（JSON 值）
        arguments: serde_json::Value,
    ) -> Self {
        AssistantContent::ToolCall(ToolCall {
            // 设置工具调用 ID
            id: id.into(),
            // 设置调用 ID
            call_id: Some(call_id),
            // 设置工具函数
            function: ToolFunction {
                // 设置函数名称
                name: name.into(),
                // 设置函数参数
                arguments,
            },
        })
    }
}

impl ToolResultContent {
    /// Helper constructor to make creating tool result text content easier.
    pub fn text(text: impl Into<String>) -> Self {
        ToolResultContent::Text(text.into().into())
    }

    /// Helper constructor to make tool result images from a base64-encoded string.
    pub fn image_base64(
        data: impl Into<String>,
        media_type: Option<ImageMediaType>,
        detail: Option<ImageDetail>,
    ) -> Self {
        ToolResultContent::Image(Image {
            data: DocumentSourceKind::Base64(data.into()),
            media_type,
            detail,
            additional_params: None,
        })
    }

    /// Helper constructor to make tool result images from a base64-encoded string.
    pub fn image_raw(
        data: impl Into<Vec<u8>>,
        media_type: Option<ImageMediaType>,
        detail: Option<ImageDetail>,
    ) -> Self {
        ToolResultContent::Image(Image {
            data: DocumentSourceKind::Raw(data.into()),
            media_type,
            detail,
            ..Default::default()
        })
    }

    /// Helper constructor to make tool result images from a URL.
    pub fn image_url(
        url: impl Into<String>,
        media_type: Option<ImageMediaType>,
        detail: Option<ImageDetail>,
    ) -> Self {
        ToolResultContent::Image(Image {
            data: DocumentSourceKind::Url(url.into()),
            media_type,
            detail,
            additional_params: None,
        })
    }
}

/// Trait for converting between MIME types and media types.
// 用于在 MIME 类型和媒体类型之间转换的 trait
pub trait MimeType {
    // 从 MIME 类型字符串创建媒体类型
    fn from_mime_type(mime_type: &str) -> Option<Self>
    where
        // Self 必须具有已知大小
        Self: Sized;
    // 将媒体类型转换为 MIME 类型字符串
    fn to_mime_type(&self) -> &'static str;
}

impl MimeType for MediaType {
    fn from_mime_type(mime_type: &str) -> Option<Self> {
        ImageMediaType::from_mime_type(mime_type)
            .map(MediaType::Image)
            .or_else(|| {
                DocumentMediaType::from_mime_type(mime_type)
                    .map(MediaType::Document)
                    .or_else(|| AudioMediaType::from_mime_type(mime_type).map(MediaType::Audio))
            })
    }

    fn to_mime_type(&self) -> &'static str {
        match self {
            MediaType::Image(media_type) => media_type.to_mime_type(),
            MediaType::Audio(media_type) => media_type.to_mime_type(),
            MediaType::Document(media_type) => media_type.to_mime_type(),
            MediaType::Video(media_type) => media_type.to_mime_type(),
        }
    }
}

impl MimeType for ImageMediaType {
    fn from_mime_type(mime_type: &str) -> Option<Self> {
        match mime_type {
            "image/jpeg" => Some(ImageMediaType::JPEG),
            "image/png" => Some(ImageMediaType::PNG),
            "image/gif" => Some(ImageMediaType::GIF),
            "image/webp" => Some(ImageMediaType::WEBP),
            "image/heic" => Some(ImageMediaType::HEIC),
            "image/heif" => Some(ImageMediaType::HEIF),
            "image/svg+xml" => Some(ImageMediaType::SVG),
            _ => None,
        }
    }

    fn to_mime_type(&self) -> &'static str {
        match self {
            ImageMediaType::JPEG => "image/jpeg",
            ImageMediaType::PNG => "image/png",
            ImageMediaType::GIF => "image/gif",
            ImageMediaType::WEBP => "image/webp",
            ImageMediaType::HEIC => "image/heic",
            ImageMediaType::HEIF => "image/heif",
            ImageMediaType::SVG => "image/svg+xml",
        }
    }
}

impl MimeType for DocumentMediaType {
    fn from_mime_type(mime_type: &str) -> Option<Self> {
        match mime_type {
            "application/pdf" => Some(DocumentMediaType::PDF),
            "text/plain" => Some(DocumentMediaType::TXT),
            "text/rtf" => Some(DocumentMediaType::RTF),
            "text/html" => Some(DocumentMediaType::HTML),
            "text/css" => Some(DocumentMediaType::CSS),
            "text/md" | "text/markdown" => Some(DocumentMediaType::MARKDOWN),
            "text/csv" => Some(DocumentMediaType::CSV),
            "text/xml" => Some(DocumentMediaType::XML),
            "application/x-javascript" | "text/x-javascript" => Some(DocumentMediaType::Javascript),
            "application/x-python" | "text/x-python" => Some(DocumentMediaType::Python),
            _ => None,
        }
    }

    fn to_mime_type(&self) -> &'static str {
        match self {
            DocumentMediaType::PDF => "application/pdf",
            DocumentMediaType::TXT => "text/plain",
            DocumentMediaType::RTF => "text/rtf",
            DocumentMediaType::HTML => "text/html",
            DocumentMediaType::CSS => "text/css",
            DocumentMediaType::MARKDOWN => "text/markdown",
            DocumentMediaType::CSV => "text/csv",
            DocumentMediaType::XML => "text/xml",
            DocumentMediaType::Javascript => "application/x-javascript",
            DocumentMediaType::Python => "application/x-python",
        }
    }
}

impl MimeType for AudioMediaType {
    fn from_mime_type(mime_type: &str) -> Option<Self> {
        match mime_type {
            "audio/wav" => Some(AudioMediaType::WAV),
            "audio/mp3" => Some(AudioMediaType::MP3),
            "audio/aiff" => Some(AudioMediaType::AIFF),
            "audio/aac" => Some(AudioMediaType::AAC),
            "audio/ogg" => Some(AudioMediaType::OGG),
            "audio/flac" => Some(AudioMediaType::FLAC),
            _ => None,
        }
    }

    fn to_mime_type(&self) -> &'static str {
        match self {
            AudioMediaType::WAV => "audio/wav",
            AudioMediaType::MP3 => "audio/mp3",
            AudioMediaType::AIFF => "audio/aiff",
            AudioMediaType::AAC => "audio/aac",
            AudioMediaType::OGG => "audio/ogg",
            AudioMediaType::FLAC => "audio/flac",
        }
    }
}

impl MimeType for VideoMediaType {
    fn from_mime_type(mime_type: &str) -> Option<Self>
    where
        Self: Sized,
    {
        match mime_type {
            "video/avi" => Some(VideoMediaType::AVI),
            "video/mp4" => Some(VideoMediaType::MP4),
            "video/mpeg" => Some(VideoMediaType::MPEG),
            &_ => None,
        }
    }

    fn to_mime_type(&self) -> &'static str {
        match self {
            VideoMediaType::AVI => "video/avi",
            VideoMediaType::MP4 => "video/mp4",
            VideoMediaType::MPEG => "video/mpeg",
        }
    }
}

// 为 ImageDetail 实现 FromStr trait
impl std::str::FromStr for ImageDetail {
    // 定义错误类型为单元类型
    type Err = ();

    // 从字符串解析图像细节级别
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 匹配小写字符串
        match s.to_lowercase().as_str() {
            // 低细节级别
            "low" => Ok(ImageDetail::Low),
            // 高细节级别
            "high" => Ok(ImageDetail::High),
            // 自动细节级别
            "auto" => Ok(ImageDetail::Auto),
            // 其他情况返回错误
            _ => Err(()),
        }
    }
}

// ================================================================
// FromStr, From<String>, and From<&str> impls
// ================================================================

// 为 Text 实现 From trait
impl From<String> for Text {
    // 从 String 创建 Text
    fn from(text: String) -> Self {
        Text { text }
    }
}

// 为 Text 实现 From trait
impl From<&String> for Text {
    // 从 &String 创建 Text
    fn from(text: &String) -> Self {
        // 克隆字符串并转换
        text.to_owned().into()
    }
}

// 为 Text 实现 From trait
impl From<&str> for Text {
    // 从 &str 创建 Text
    fn from(text: &str) -> Self {
        // 转换为拥有的字符串并转换
        text.to_owned().into()
    }
}

// 为 Text 实现 FromStr trait
impl FromStr for Text {
    // 错误类型为 Infallible（永远不会失败）
    type Err = Infallible;

    // 从字符串解析 Text
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 直接转换，不会失败
        Ok(s.into())
    }
}

impl From<String> for Message {
    fn from(text: String) -> Self {
        Message::User {
            content: OneOrMany::one(UserContent::Text(text.into())),
        }
    }
}

impl From<&str> for Message {
    fn from(text: &str) -> Self {
        Message::User {
            content: OneOrMany::one(UserContent::Text(text.into())),
        }
    }
}

impl From<&String> for Message {
    fn from(text: &String) -> Self {
        Message::User {
            content: OneOrMany::one(UserContent::Text(text.into())),
        }
    }
}

impl From<Text> for Message {
    fn from(text: Text) -> Self {
        Message::User {
            content: OneOrMany::one(UserContent::Text(text)),
        }
    }
}

impl From<Image> for Message {
    fn from(image: Image) -> Self {
        Message::User {
            content: OneOrMany::one(UserContent::Image(image)),
        }
    }
}

impl From<Audio> for Message {
    fn from(audio: Audio) -> Self {
        Message::User {
            content: OneOrMany::one(UserContent::Audio(audio)),
        }
    }
}

impl From<Document> for Message {
    fn from(document: Document) -> Self {
        Message::User {
            content: OneOrMany::one(UserContent::Document(document)),
        }
    }
}

impl From<String> for ToolResultContent {
    fn from(text: String) -> Self {
        ToolResultContent::text(text)
    }
}

impl From<String> for AssistantContent {
    fn from(text: String) -> Self {
        AssistantContent::text(text)
    }
}

impl From<String> for UserContent {
    fn from(text: String) -> Self {
        UserContent::text(text)
    }
}

impl From<AssistantContent> for Message {
    fn from(content: AssistantContent) -> Self {
        Message::Assistant {
            id: None,
            content: OneOrMany::one(content),
        }
    }
}

impl From<UserContent> for Message {
    fn from(content: UserContent) -> Self {
        Message::User {
            content: OneOrMany::one(content),
        }
    }
}

impl From<OneOrMany<AssistantContent>> for Message {
    fn from(content: OneOrMany<AssistantContent>) -> Self {
        Message::Assistant { id: None, content }
    }
}

impl From<OneOrMany<UserContent>> for Message {
    fn from(content: OneOrMany<UserContent>) -> Self {
        Message::User { content }
    }
}

impl From<ToolCall> for Message {
    fn from(tool_call: ToolCall) -> Self {
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::ToolCall(tool_call)),
        }
    }
}

impl From<ToolResult> for Message {
    fn from(tool_result: ToolResult) -> Self {
        Message::User {
            content: OneOrMany::one(UserContent::ToolResult(tool_result)),
        }
    }
}

impl From<ToolResultContent> for Message {
    fn from(tool_result_content: ToolResultContent) -> Self {
        Message::User {
            content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                id: String::new(),
                call_id: None,
                content: OneOrMany::one(tool_result_content),
            })),
        }
    }
}

// 工具选择枚举，定义如何使用工具
#[derive(Default, Clone, Debug, Deserialize, Serialize, PartialEq)]
// 序列化时重命名为蛇形命名
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    // 默认自动选择
    #[default]
    // 自动选择工具
    Auto,
    // 不使用工具
    None,
    // 必须使用工具
    Required,
    // 指定特定函数
    Specific {
        // 函数名称列表
        function_names: Vec<String>,
    },
}

// ================================================================
// Error types
// ================================================================

/// Error type to represent issues with converting messages to and from specific provider messages.
// 错误类型，用于表示在特定提供商消息之间转换时出现的问题
#[derive(Debug, Error)]
pub enum MessageError {
    // 消息转换错误，包含错误消息
    #[error("Message conversion error: {0}")]
    ConversionError(String),
}

// 为 MessageError 实现从 CompletionError 的转换
impl From<MessageError> for CompletionError {
    // 将消息错误转换为完成错误
    fn from(error: MessageError) -> Self {
        // 包装为请求错误
        CompletionError::RequestError(error.into())
    }
}
