// 音频生成模块（条件编译）
#[cfg(feature = "audio")]
use super::audio_generation::AudioGenerationModel;
// 导入嵌入模块的相关类型
use super::embedding::{
    EmbeddingModel, TEXT_EMBEDDING_3_LARGE, TEXT_EMBEDDING_3_SMALL, TEXT_EMBEDDING_ADA_002,
};

// 图像生成模块（条件编译）
#[cfg(feature = "image")]
use super::image_generation::ImageGenerationModel;
// 导入转录模块
use super::transcription::TranscriptionModel;

// 导入客户端相关的 trait 和类型
use crate::{
    client::{
        ClientBuilderError, CompletionClient, EmbeddingsClient, ProviderClient,
        TranscriptionClient, VerifyClient, VerifyError,
    },
    extractor::ExtractorBuilder,
    providers::openai::CompletionModel,
};

// 音频生成客户端（条件编译）
#[cfg(feature = "audio")]
use crate::client::AudioGenerationClient;
// 图像生成客户端（条件编译）
#[cfg(feature = "image")]
use crate::client::ImageGenerationClient;

// 导入 JSON Schema 宏
use schemars::JsonSchema;
// 导入序列化和反序列化宏
use serde::{Deserialize, Serialize};

// ================================================================
// 主 OpenAI 客户端
// ================================================================
// OpenAI API 基础 URL 常量
const OPENAI_API_BASE_URL: &str = "https://api.openai.com/v1";

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
            base_url: OPENAI_API_BASE_URL,
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
    // 基础 URL
    base_url: String,
    // API 密钥
    api_key: String,
    // HTTP 客户端
    http_client: reqwest::Client,
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
    /// Create a new OpenAI client builder.
    ///
    /// # Example
    /// ```
    /// use rig::providers::openai::{ClientBuilder, self};
    ///
    /// // Initialize the OpenAI client
    /// let openai_client = Client::builder("your-open-ai-api-key")
    ///    .build()
    /// ```
    // 创建新的 OpenAI 客户端构建器
    ///
    /// # 示例
    /// ```
    /// use rig::providers::openai::{ClientBuilder, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai_client = Client::builder("your-open-ai-api-key")
    ///    .build()
    /// ```
    pub fn builder(api_key: &str) -> ClientBuilder<'_> {
        // 创建新的客户端构建器
        ClientBuilder::new(api_key)
    }

    /// Create a new OpenAI client. For more control, use the `builder` method.
    ///
    /// # Panics
    /// - If the reqwest client cannot be built (if the TLS backend cannot be initialized).
    // 创建新的 OpenAI 客户端。如需更多控制，请使用 `builder` 方法
    ///
    /// # 恐慌
    /// - 如果无法构建 reqwest 客户端（如果无法初始化 TLS 后端）
    pub fn new(api_key: &str) -> Self {
        Self::builder(api_key)
            .build()
            .expect("OpenAI client should build")
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

    /// Create an extractor builder with the given completion model.
    /// Intended for use exclusively with the Chat Completions API.
    /// Useful for using extractors with Chat Completion compliant APIs.
    // 使用给定的完成模型创建提取器构建器
    // 专门用于聊天完成 API
    // 适用于与聊天完成兼容的 API 一起使用提取器
    pub fn extractor_completions_api<T>(&self, model: &str) -> ExtractorBuilder<CompletionModel, T>
    where
        T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync,
    {
        // 创建提取器构建器
        ExtractorBuilder::new(self.completion_model(model).completions_api())
    }
}

// 为 Client 实现 ProviderClient trait
impl ProviderClient for Client {
    /// Create a new OpenAI client from the `OPENAI_API_KEY` environment variable.
    /// Panics if the environment variable is not set.
    // 从 `OPENAI_API_KEY` 环境变量创建新的 OpenAI 客户端
    // 如果环境变量未设置，则恐慌
    fn from_env() -> Self {
        // 尝试获取基础 URL 环境变量
        let base_url: Option<String> = std::env::var("OPENAI_BASE_URL").ok();
        // 获取 API 密钥环境变量（如果未设置则恐慌）
        let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

        // 根据是否有基础 URL 创建客户端
        match base_url {
            // 如果有基础 URL，使用构建器创建客户端
            Some(url) => Self::builder(&api_key).base_url(&url).build().unwrap(),
            // 如果没有基础 URL，使用默认设置创建客户端
            None => Self::new(&api_key),
        }
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
    type CompletionModel = super::responses_api::ResponsesCompletionModel;
    /// Create a completion model with the given name.
    ///
    /// # Example
    /// ```
    /// use rig::providers::openai::{Client, self};
    ///
    /// // Initialize the OpenAI client
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let gpt4 = openai.completion_model(openai::GPT_4);
    /// ```
    // 使用给定名称创建完成模型
    ///
    /// # 示例
    /// ```
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let gpt4 = openai.completion_model(openai::GPT_4);
    /// ```
    fn completion_model(&self, model: &str) -> super::responses_api::ResponsesCompletionModel {
        // 创建新的响应完成模型
        super::responses_api::ResponsesCompletionModel::new(self.clone(), model)
    }
}

// 为 Client 实现 EmbeddingsClient trait
impl EmbeddingsClient for Client {
    // 嵌入模型类型
    type EmbeddingModel = EmbeddingModel;
    // 创建嵌入模型
    fn embedding_model(&self, model: &str) -> Self::EmbeddingModel {
        // 根据模型名称确定维度数
        let ndims = match model {
            // TEXT_EMBEDDING_3_LARGE 模型的维度
            TEXT_EMBEDDING_3_LARGE => 3072,
            // TEXT_EMBEDDING_3_SMALL 和 TEXT_EMBEDDING_ADA_002 模型的维度
            TEXT_EMBEDDING_3_SMALL | TEXT_EMBEDDING_ADA_002 => 1536,
            // 其他模型的维度（未知）
            _ => 0,
        };
        // 创建新的嵌入模型
        EmbeddingModel::new(self.clone(), model, ndims)
    }

    // 使用指定维度创建嵌入模型
    fn embedding_model_with_ndims(&self, model: &str, ndims: usize) -> Self::EmbeddingModel {
        // 创建新的嵌入模型
        EmbeddingModel::new(self.clone(), model, ndims)
    }
}

// 为 Client 实现 TranscriptionClient trait
impl TranscriptionClient for Client {
    // 转录模型类型
    type TranscriptionModel = TranscriptionModel;
    /// Create a transcription model with the given name.
    ///
    /// # Example
    /// ```
    /// use rig::providers::openai::{Client, self};
    ///
    /// // Initialize the OpenAI client
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let gpt4 = openai.transcription_model(openai::WHISPER_1);
    /// ```
    // 使用给定名称创建转录模型
    ///
    /// # 示例
    /// ```
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let gpt4 = openai.transcription_model(openai::WHISPER_1);
    /// ```
    fn transcription_model(&self, model: &str) -> TranscriptionModel {
        // 创建新的转录模型
        TranscriptionModel::new(self.clone(), model)
    }
}

// 图像生成客户端实现（条件编译）
#[cfg(feature = "image")]
impl ImageGenerationClient for Client {
    // 图像生成模型类型
    type ImageGenerationModel = ImageGenerationModel;
    /// Create an image generation model with the given name.
    ///
    /// # Example
    /// ```
    /// use rig::providers::openai::{Client, self};
    ///
    /// // Initialize the OpenAI client
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let gpt4 = openai.image_generation_model(openai::DALL_E_3);
    /// ```
    // 使用给定名称创建图像生成模型
    ///
    /// # 示例
    /// ```
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let gpt4 = openai.image_generation_model(openai::DALL_E_3);
    /// ```
    fn image_generation_model(&self, model: &str) -> Self::ImageGenerationModel {
        // 创建新的图像生成模型
        ImageGenerationModel::new(self.clone(), model)
    }
}

// 音频生成客户端实现（条件编译）
#[cfg(feature = "audio")]
impl AudioGenerationClient for Client {
    // 音频生成模型类型
    type AudioGenerationModel = AudioGenerationModel;
    /// Create an audio generation model with the given name.
    ///
    /// # Example
    /// ```
    /// use rig::providers::openai::{Client, self};
    ///
    /// // Initialize the OpenAI client
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let gpt4 = openai.audio_generation_model(openai::TTS_1);
    /// ```
    // 使用给定名称创建音频生成模型
    ///
    /// # 示例
    /// ```
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let gpt4 = openai.audio_generation_model(openai::TTS_1);
    /// ```
    fn audio_generation_model(&self, model: &str) -> Self::AudioGenerationModel {
        // 创建新的音频生成模型
        AudioGenerationModel::new(self.clone(), model)
    }
}

// 为 Client 实现 VerifyClient trait
impl VerifyClient for Client {
    // 验证方法（支持 worker 特性）
    #[cfg_attr(feature = "worker", worker::send)]
    async fn verify(&self) -> Result<(), VerifyError> {
        // 发送 GET 请求到 /models 端点
        let response = self.get("/models").send().await?;
        // 根据响应状态码处理结果
        match response.status() {
            // 成功状态
            reqwest::StatusCode::OK => Ok(()),
            // 未授权状态
            reqwest::StatusCode::UNAUTHORIZED => Err(VerifyError::InvalidAuthentication),
            // 内部服务器错误
            reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                Err(VerifyError::ProviderError(response.text().await?))
            }
            // 其他状态码
            _ => {
                // 检查状态码并返回错误（如果有）
                response.error_for_status()?;
                Ok(())
            }
        }
    }
}

// 派生 Debug 和 Deserialize trait
#[derive(Debug, Deserialize)]
// API 错误响应结构体
pub struct ApiErrorResponse {
    // 错误消息（包可见）
    pub(crate) message: String,
}

// 派生 Debug 和 Deserialize trait
#[derive(Debug, Deserialize)]
#[serde(untagged)]
// API 响应枚举（包可见）
pub(crate) enum ApiResponse<T> {
    // 成功响应
    Ok(T),
    // 错误响应
    Err(ApiErrorResponse),
}

// 测试模块（条件编译）
#[cfg(test)]
mod tests {
    // 导入消息图像详情
    use crate::message::ImageDetail;
    // 导入 OpenAI 提供商的类型
    use crate::providers::openai::{
        AssistantContent, Function, ImageUrl, Message, ToolCall, ToolType, UserContent,
    };
    // 导入 OneOrMany 和 message 模块
    use crate::{OneOrMany, message};
    // 导入路径错误反序列化
    use serde_path_to_error::deserialize;

    // 测试消息反序列化
    #[test]
    fn test_deserialize_message() {
        let assistant_message_json = r#"
        {
            "role": "assistant",
            "content": "\n\nHello there, how may I assist you today?"
        }
        "#;

        let assistant_message_json2 = r#"
        {
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "\n\nHello there, how may I assist you today?"
                }
            ],
            "tool_calls": null
        }
        "#;

        let assistant_message_json3 = r#"
        {
            "role": "assistant",
            "tool_calls": [
                {
                    "id": "call_h89ipqYUjEpCPI6SxspMnoUU",
                    "type": "function",
                    "function": {
                        "name": "subtract",
                        "arguments": "{\"x\": 2, \"y\": 5}"
                    }
                }
            ],
            "content": null,
            "refusal": null
        }
        "#;

        let user_message_json = r#"
        {
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "What's in this image?"
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg"
                    }
                },
                {
                    "type": "audio",
                    "input_audio": {
                        "data": "...",
                        "format": "mp3"
                    }
                }
            ]
        }
        "#;

        let assistant_message: Message = {
            let jd = &mut serde_json::Deserializer::from_str(assistant_message_json);
            deserialize(jd).unwrap_or_else(|err| {
                panic!(
                    "Deserialization error at {} ({}:{}): {}",
                    err.path(),
                    err.inner().line(),
                    err.inner().column(),
                    err
                );
            })
        };

        let assistant_message2: Message = {
            let jd = &mut serde_json::Deserializer::from_str(assistant_message_json2);
            deserialize(jd).unwrap_or_else(|err| {
                panic!(
                    "Deserialization error at {} ({}:{}): {}",
                    err.path(),
                    err.inner().line(),
                    err.inner().column(),
                    err
                );
            })
        };

        let assistant_message3: Message = {
            let jd: &mut serde_json::Deserializer<serde_json::de::StrRead<'_>> =
                &mut serde_json::Deserializer::from_str(assistant_message_json3);
            deserialize(jd).unwrap_or_else(|err| {
                panic!(
                    "Deserialization error at {} ({}:{}): {}",
                    err.path(),
                    err.inner().line(),
                    err.inner().column(),
                    err
                );
            })
        };

        let user_message: Message = {
            let jd = &mut serde_json::Deserializer::from_str(user_message_json);
            deserialize(jd).unwrap_or_else(|err| {
                panic!(
                    "Deserialization error at {} ({}:{}): {}",
                    err.path(),
                    err.inner().line(),
                    err.inner().column(),
                    err
                );
            })
        };

        match assistant_message {
            Message::Assistant { content, .. } => {
                assert_eq!(
                    content[0],
                    AssistantContent::Text {
                        text: "\n\nHello there, how may I assist you today?".to_string()
                    }
                );
            }
            _ => panic!("Expected assistant message"),
        }

        match assistant_message2 {
            Message::Assistant {
                content,
                tool_calls,
                ..
            } => {
                assert_eq!(
                    content[0],
                    AssistantContent::Text {
                        text: "\n\nHello there, how may I assist you today?".to_string()
                    }
                );

                assert_eq!(tool_calls, vec![]);
            }
            _ => panic!("Expected assistant message"),
        }

        match assistant_message3 {
            Message::Assistant {
                content,
                tool_calls,
                refusal,
                ..
            } => {
                assert!(content.is_empty());
                assert!(refusal.is_none());
                assert_eq!(
                    tool_calls[0],
                    ToolCall {
                        id: "call_h89ipqYUjEpCPI6SxspMnoUU".to_string(),
                        r#type: ToolType::Function,
                        function: Function {
                            name: "subtract".to_string(),
                            arguments: serde_json::json!({"x": 2, "y": 5}),
                        },
                    }
                );
            }
            _ => panic!("Expected assistant message"),
        }

        match user_message {
            Message::User { content, .. } => {
                let (first, second) = {
                    let mut iter = content.into_iter();
                    (iter.next().unwrap(), iter.next().unwrap())
                };
                assert_eq!(
                    first,
                    UserContent::Text {
                        text: "What's in this image?".to_string()
                    }
                );
                assert_eq!(second, UserContent::Image { image_url: ImageUrl { url: "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg".to_string(), detail: ImageDetail::default() } });
            }
            _ => panic!("Expected user message"),
        }
    }

    // 测试消息到消息的转换
    #[test]
    fn test_message_to_message_conversion() {
        let user_message = message::Message::User {
            content: OneOrMany::one(message::UserContent::text("Hello")),
        };

        let assistant_message = message::Message::Assistant {
            id: None,
            content: OneOrMany::one(message::AssistantContent::text("Hi there!")),
        };

        let converted_user_message: Vec<Message> = user_message.clone().try_into().unwrap();
        let converted_assistant_message: Vec<Message> =
            assistant_message.clone().try_into().unwrap();

        match converted_user_message[0].clone() {
            Message::User { content, .. } => {
                assert_eq!(
                    content.first(),
                    UserContent::Text {
                        text: "Hello".to_string()
                    }
                );
            }
            _ => panic!("Expected user message"),
        }

        match converted_assistant_message[0].clone() {
            Message::Assistant { content, .. } => {
                assert_eq!(
                    content[0].clone(),
                    AssistantContent::Text {
                        text: "Hi there!".to_string()
                    }
                );
            }
            _ => panic!("Expected assistant message"),
        }

        let original_user_message: message::Message =
            converted_user_message[0].clone().try_into().unwrap();
        let original_assistant_message: message::Message =
            converted_assistant_message[0].clone().try_into().unwrap();

        assert_eq!(original_user_message, user_message);
        assert_eq!(original_assistant_message, assistant_message);
    }

    // 测试消息从消息的转换
    #[test]
    fn test_message_from_message_conversion() {
        let user_message = Message::User {
            content: OneOrMany::one(UserContent::Text {
                text: "Hello".to_string(),
            }),
            name: None,
        };

        let assistant_message = Message::Assistant {
            content: vec![AssistantContent::Text {
                text: "Hi there!".to_string(),
            }],
            refusal: None,
            audio: None,
            name: None,
            tool_calls: vec![],
        };

        let converted_user_message: message::Message = user_message.clone().try_into().unwrap();
        let converted_assistant_message: message::Message =
            assistant_message.clone().try_into().unwrap();

        match converted_user_message.clone() {
            message::Message::User { content } => {
                assert_eq!(content.first(), message::UserContent::text("Hello"));
            }
            _ => panic!("Expected user message"),
        }

        match converted_assistant_message.clone() {
            message::Message::Assistant { content, .. } => {
                assert_eq!(
                    content.first(),
                    message::AssistantContent::text("Hi there!")
                );
            }
            _ => panic!("Expected assistant message"),
        }

        let original_user_message: Vec<Message> = converted_user_message.try_into().unwrap();
        let original_assistant_message: Vec<Message> =
            converted_assistant_message.try_into().unwrap();

        assert_eq!(original_user_message[0], user_message);
        assert_eq!(original_assistant_message[0], assistant_message);
    }
}
