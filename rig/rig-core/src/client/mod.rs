//! 此模块提供用于定义和创建提供商客户端的 trait。
//! 客户端用于创建完成、嵌入等模型。
//! 提供了 dyn 兼容的 trait 以允许更多提供商无关的代码。

// 导出音频生成模块
pub mod audio_generation;
// 导出构建器模块
pub mod builder;
// 导出完成模块
pub mod completion;
// 导出嵌入模块
pub mod embeddings;
// 导出图像生成模块
pub mod image_generation;
// 导出转录模块
pub mod transcription;
// 导出验证模块
pub mod verify;

// 当启用 "derive" 功能时，导出派生宏
#[cfg(feature = "derive")]
pub use rig_derive::ProviderClient;
// 导入调试格式化 trait
use std::fmt::Debug;
// 导入错误处理宏
use thiserror::Error;

// 派生 Debug 和 Error trait 用于错误处理
#[derive(Debug, Error)]
// 标记为非穷尽枚举，表示未来可能添加更多变体
#[non_exhaustive]
// 定义客户端构建错误枚举
pub enum ClientBuilderError {
    // HTTP 请求错误，包含 reqwest 错误信息
    #[error("reqwest error: {0}")]
    HttpError(
        // 自动从 reqwest::Error 转换
        #[from]
        // 标记为错误源
        #[source]
        reqwest::Error,
    ),
    // 无效属性错误，包含属性名称
    #[error("invalid property: {0}")]
    InvalidProperty(&'static str),
}

/// 基础 ProviderClient trait，促进客户端类型之间的转换
/// 以及从环境创建客户端。
///
/// 必须实现所有转换 trait，如果实现了相应的客户端 trait，
/// 它们会自动实现。
// 定义提供商客户端 trait，要求实现所有转换 trait 和调试格式化
pub trait ProviderClient:
    AsCompletion + AsTranscription + AsEmbeddings + AsImageGeneration + AsAudioGeneration + Debug
{
    /// 从进程环境创建客户端。
    /// 如果环境配置不当会 panic。
    // 从进程环境创建客户端实例
    fn from_env() -> Self
    where
        // Self 必须具有已知大小
        Self: Sized;

    /// 用于装箱客户端的辅助方法。
    // 将客户端装箱为动态 trait 对象
    fn boxed(self) -> Box<dyn ProviderClient>
    where
        // Self 必须具有已知大小和静态生命周期
        Self: Sized + 'static,
    {
        // 将客户端包装在 Box 中
        Box::new(self)
    }

    /// 从进程环境创建装箱的客户端。
    /// 如果环境配置不当会 panic。
    // 从进程环境创建装箱的客户端
    fn from_env_boxed<'a>() -> Box<dyn ProviderClient + 'a>
    where
        // Self 必须具有已知大小和生命周期 'a
        Self: Sized,
        Self: 'a,
    {
        // 创建客户端并装箱
        Box::new(Self::from_env())
    }

    // 从提供商值创建客户端实例
    fn from_val(input: ProviderValue) -> Self
    where
        // Self 必须具有已知大小
        Self: Sized;

    /// 从进程环境创建装箱的客户端。
    /// 如果环境配置不当会 panic。
    // 从提供商值创建装箱的客户端
    fn from_val_boxed<'a>(input: ProviderValue) -> Box<dyn ProviderClient + 'a>
    where
        // Self 必须具有已知大小和生命周期 'a
        Self: Sized,
        Self: 'a,
    {
        // 从提供商值创建客户端并装箱
        Box::new(Self::from_val(input))
    }
}

// 派生 Clone trait 用于提供商值枚举
#[derive(Clone)]
// 定义提供商值枚举，用于表示不同类型的提供商配置
pub enum ProviderValue {
    // 简单的字符串值
    Simple(String),
    // API 密钥和可选密钥
    ApiKeyWithOptionalKey(String, Option<String>),
    // API 密钥、版本和头部信息
    ApiKeyWithVersionAndHeader(String, String, String),
}

// 为 &str 实现 From trait，转换为 ProviderValue
impl From<&str> for ProviderValue {
    // 从字符串切片创建简单的提供商值
    fn from(value: &str) -> Self {
        // 将字符串切片转换为字符串并创建简单值
        Self::Simple(value.to_string())
    }
}

// 为 String 实现 From trait，转换为 ProviderValue
impl From<String> for ProviderValue {
    // 从字符串创建简单的提供商值
    fn from(value: String) -> Self {
        // 直接使用字符串创建简单值
        Self::Simple(value)
    }
}

// 为 (P, Option<P>) 实现 From trait，转换为 ProviderValue
impl<P> From<(P, Option<P>)> for ProviderValue
where
    // P 必须能够转换为字符串引用
    P: AsRef<str>,
{
    // 从 API 密钥和可选密钥元组创建提供商值
    fn from((api_key, optional_key): (P, Option<P>)) -> Self {
        // 创建带可选密钥的 API 密钥值
        Self::ApiKeyWithOptionalKey(
            // 转换 API 密钥为字符串
            api_key.as_ref().to_string(),
            // 如果可选密钥存在，则转换为字符串
            optional_key.map(|x| x.as_ref().to_string()),
        )
    }
}

// 为 (P, P, P) 实现 From trait，转换为 ProviderValue
impl<P> From<(P, P, P)> for ProviderValue
where
    // P 必须能够转换为字符串引用
    P: AsRef<str>,
{
    // 从 API 密钥、版本和头部元组创建提供商值
    fn from((api_key, version, header): (P, P, P)) -> Self {
        // 创建带版本和头部的 API 密钥值
        Self::ApiKeyWithVersionAndHeader(
            // 转换 API 密钥为字符串
            api_key.as_ref().to_string(),
            // 转换版本为字符串
            version.as_ref().to_string(),
            // 转换头部为字符串
            header.as_ref().to_string(),
        )
    }
}

/// Attempt to convert a ProviderClient to a CompletionClient
// 尝试将 ProviderClient 转换为 CompletionClient
pub trait AsCompletion {
    // 尝试转换为完成客户端，默认返回 None
    fn as_completion(&self) -> Option<Box<dyn CompletionClientDyn>> {
        // 默认实现返回 None
        None
    }
}

/// Attempt to convert a ProviderClient to a TranscriptionClient
// 尝试将 ProviderClient 转换为 TranscriptionClient
pub trait AsTranscription {
    // 尝试转换为转录客户端，默认返回 None
    fn as_transcription(&self) -> Option<Box<dyn TranscriptionClientDyn>> {
        // 默认实现返回 None
        None
    }
}

/// Attempt to convert a ProviderClient to a EmbeddingsClient
// 尝试将 ProviderClient 转换为 EmbeddingsClient
pub trait AsEmbeddings {
    // 尝试转换为嵌入客户端，默认返回 None
    fn as_embeddings(&self) -> Option<Box<dyn EmbeddingsClientDyn>> {
        // 默认实现返回 None
        None
    }
}

/// Attempt to convert a ProviderClient to a AudioGenerationClient
// 尝试将 ProviderClient 转换为 AudioGenerationClient
pub trait AsAudioGeneration {
    // 当启用 "audio" 功能时
    #[cfg(feature = "audio")]
    // 尝试转换为音频生成客户端，默认返回 None
    fn as_audio_generation(&self) -> Option<Box<dyn AudioGenerationClientDyn>> {
        // 默认实现返回 None
        None
    }
}

/// Attempt to convert a ProviderClient to a ImageGenerationClient
// 尝试将 ProviderClient 转换为 ImageGenerationClient
pub trait AsImageGeneration {
    // 当启用 "image" 功能时
    #[cfg(feature = "image")]
    // 尝试转换为图像生成客户端，默认返回 None
    fn as_image_generation(&self) -> Option<Box<dyn ImageGenerationClientDyn>> {
        // 默认实现返回 None
        None
    }
}

/// Attempt to convert a ProviderClient to a VerifyClient
// 尝试将 ProviderClient 转换为 VerifyClient
pub trait AsVerify {
    // 尝试转换为验证客户端，默认返回 None
    fn as_verify(&self) -> Option<Box<dyn VerifyClientDyn>> {
        // 默认实现返回 None
        None
    }
}

// 当未启用 "audio" 功能时，为所有 ProviderClient 实现 AsAudioGeneration
#[cfg(not(feature = "audio"))]
impl<T: ProviderClient> AsAudioGeneration for T {}

// 当未启用 "image" 功能时，为所有 ProviderClient 实现 AsImageGeneration
#[cfg(not(feature = "image"))]
impl<T: ProviderClient> AsImageGeneration for T {}

/// Implements the conversion traits for a given struct
/// ```rust
/// pub struct Client;
/// impl ProviderClient for Client {
///     ...
/// }
/// impl_conversion_traits!(AsCompletion, AsEmbeddings for Client);
/// ```
// 为给定结构体实现转换 trait 的宏
#[macro_export]
macro_rules! impl_conversion_traits {
    // 匹配多个 trait 和一个结构体
    ($( $trait_:ident ),* for $struct_:ident ) => {
        // 为每个 trait 调用内部实现宏
        $(
            impl_conversion_traits!(@impl $trait_ for $struct_);
        )*
    };

    // 特殊处理 AsAudioGeneration trait
    (@impl AsAudioGeneration for $struct_:ident ) => {
        // 调用音频生成实现宏
        rig::client::impl_audio_generation!($struct_);
    };

    // 特殊处理 AsImageGeneration trait
    (@impl AsImageGeneration for $struct_:ident ) => {
        // 调用图像生成实现宏
        rig::client::impl_image_generation!($struct_);
    };

    // 通用 trait 实现
    (@impl $trait_:ident for $struct_:ident) => {
        // 为结构体实现指定的 trait
        impl rig::client::$trait_ for $struct_ {}
    };
}

// 当启用 "audio" 功能时
#[cfg(feature = "audio")]
// 导出音频生成实现宏
#[macro_export]
macro_rules! impl_audio_generation {
    // 为结构体实现 AsAudioGeneration trait
    ($struct_:ident) => {
        // 为指定结构体实现音频生成转换 trait
        impl rig::client::AsAudioGeneration for $struct_ {}
    };
}

// 当未启用 "audio" 功能时
#[cfg(not(feature = "audio"))]
// 导出空的音频生成实现宏
#[macro_export]
macro_rules! impl_audio_generation {
    // 空实现，不生成任何代码
    ($struct_:ident) => {};
}

// 当启用 "image" 功能时
#[cfg(feature = "image")]
// 导出图像生成实现宏
#[macro_export]
macro_rules! impl_image_generation {
    // 为结构体实现 AsImageGeneration trait
    ($struct_:ident) => {
        // 为指定结构体实现图像生成转换 trait
        impl rig::client::AsImageGeneration for $struct_ {}
    };
}

// 当未启用 "image" 功能时
#[cfg(not(feature = "image"))]
// 导出空的图像生成实现宏
#[macro_export]
macro_rules! impl_image_generation {
    // 空实现，不生成任何代码
    ($struct_:ident) => {};
}

// 重新导出音频生成实现宏
pub use impl_audio_generation;
// 重新导出转换 trait 实现宏
pub use impl_conversion_traits;
// 重新导出图像生成实现宏
pub use impl_image_generation;

// 当启用 "audio" 功能时，导入音频生成客户端动态类型
#[cfg(feature = "audio")]
use crate::client::audio_generation::AudioGenerationClientDyn;
// 导入完成客户端动态类型
use crate::client::completion::CompletionClientDyn;
// 导入嵌入客户端动态类型
use crate::client::embeddings::EmbeddingsClientDyn;
// 当启用 "image" 功能时，导入图像生成客户端动态类型
#[cfg(feature = "image")]
use crate::client::image_generation::ImageGenerationClientDyn;
// 导入转录客户端动态类型
use crate::client::transcription::TranscriptionClientDyn;
// 导入验证客户端动态类型
use crate::client::verify::VerifyClientDyn;

// 当启用 "audio" 功能时，重新导出音频生成客户端
#[cfg(feature = "audio")]
pub use crate::client::audio_generation::AudioGenerationClient;
// 重新导出完成客户端
pub use crate::client::completion::CompletionClient;
// 重新导出嵌入客户端
pub use crate::client::embeddings::EmbeddingsClient;
// 当启用 "image" 功能时，重新导出图像生成客户端
#[cfg(feature = "image")]
pub use crate::client::image_generation::ImageGenerationClient;
// 重新导出转录客户端
pub use crate::client::transcription::TranscriptionClient;
// 重新导出验证客户端和错误类型
pub use crate::client::verify::{VerifyClient, VerifyError};

// 当运行测试时
#[cfg(test)]
// 测试模块
mod tests {
    // 导入 OneOrMany 类型
    use crate::OneOrMany;
    // 导入提供商客户端 trait
    use crate::client::ProviderClient;
    // 导入完成相关类型
    use crate::completion::{Completion, CompletionRequest, ToolDefinition};
    // 导入图像生成请求类型
    use crate::image_generation::ImageGenerationRequest;
    // 导入助手内容类型
    use crate::message::AssistantContent;
    // 导入各种提供商模块
    use crate::providers::{
        anthropic, azure, cohere, deepseek, galadriel, gemini, huggingface, hyperbolic, mira,
        moonshot, openai, openrouter, together, xai,
    };
    // 导入流式完成 trait
    use crate::streaming::StreamingCompletion;
    // 导入工具 trait
    use crate::tool::Tool;
    // 导入转录请求类型
    use crate::transcription::TranscriptionRequest;
    // 导入流扩展方法
    use futures::StreamExt;
    // 导入消息类型
    use rig::message::Message;
    // 导入更多提供商模块
    use rig::providers::{groq, ollama, perplexity};
    // 导入序列化和反序列化 trait
    use serde::{Deserialize, Serialize};
    // 导入 JSON 宏
    use serde_json::json;
    // 导入文件操作
    use std::fs::File;
    // 导入 IO 读取 trait
    use std::io::Read;

    // 导入父模块的 ProviderValue 类型
    use super::ProviderValue;

    // 定义客户端配置结构体
    struct ClientConfig {
        // 客户端名称
        name: &'static str,
        // 从环境变量创建客户端的工厂函数
        factory_env: Box<dyn Fn() -> Box<dyn ProviderClient>>,
        // Not sure where we're going to be using this but I've added it for completeness
        // 不确定我们将在哪里使用这个，但为了完整性我添加了它
        // 从提供商值创建客户端的工厂函数（允许未使用）
        #[allow(dead_code)]
        factory_val: Box<dyn Fn(ProviderValue) -> Box<dyn ProviderClient>>,
        // 环境变量名称
        env_variable: &'static str,
        // 完成模型名称（可选）
        completion_model: Option<&'static str>,
        // 嵌入模型名称（可选）
        embeddings_model: Option<&'static str>,
        // 转录模型名称（可选）
        transcription_model: Option<&'static str>,
        // 图像生成模型名称（可选）
        image_generation_model: Option<&'static str>,
        // 音频生成模型和语音（可选）
        audio_generation_model: Option<(&'static str, &'static str)>,
    }

    // 为 ClientConfig 实现 Default trait
    impl Default for ClientConfig {
        // 提供默认实现
        fn default() -> Self {
            Self {
                // 设置默认名称为空字符串
                name: "",
                // 设置默认环境工厂函数，未实现时 panic
                factory_env: Box::new(|| panic!("Not implemented")),
                // 设置默认值工厂函数，未实现时 panic
                factory_val: Box::new(|_| panic!("Not implemented")),
                // 设置默认环境变量为空字符串
                env_variable: "",
                // 设置默认完成模型为 None
                completion_model: None,
                // 设置默认嵌入模型为 None
                embeddings_model: None,
                // 设置默认转录模型为 None
                transcription_model: None,
                // 设置默认图像生成模型为 None
                image_generation_model: None,
                // 设置默认音频生成模型为 None
                audio_generation_model: None,
            }
        }
    }

    // 为 ClientConfig 实现方法
    impl ClientConfig {
        // 检查环境变量是否已设置
        fn is_env_var_set(&self) -> bool {
            // 如果环境变量为空或者环境变量存在，则返回 true
            self.env_variable.is_empty() || std::env::var(self.env_variable).is_ok()
        }

        // 从环境变量创建客户端
        fn factory_env(&self) -> Box<dyn ProviderClient + '_> {
            // 调用环境工厂函数创建客户端
            self.factory_env.as_ref()()
        }
    }

    fn providers() -> Vec<ClientConfig> {
        vec![
            ClientConfig {
                name: "Anthropic",
                factory_env: Box::new(anthropic::Client::from_env_boxed),
                factory_val: Box::new(anthropic::Client::from_val_boxed),
                env_variable: "ANTHROPIC_API_KEY",
                completion_model: Some(anthropic::CLAUDE_3_5_SONNET),
                ..Default::default()
            },
            ClientConfig {
                name: "Cohere",
                factory_env: Box::new(cohere::Client::from_env_boxed),
                factory_val: Box::new(cohere::Client::from_val_boxed),
                env_variable: "COHERE_API_KEY",
                completion_model: Some(cohere::COMMAND_R),
                embeddings_model: Some(cohere::EMBED_ENGLISH_LIGHT_V2),
                ..Default::default()
            },
            ClientConfig {
                name: "Gemini",
                factory_env: Box::new(gemini::Client::from_env_boxed),
                factory_val: Box::new(gemini::Client::from_val_boxed),
                env_variable: "GEMINI_API_KEY",
                completion_model: Some(gemini::completion::GEMINI_2_0_FLASH),
                embeddings_model: Some(gemini::embedding::EMBEDDING_001),
                transcription_model: Some(gemini::transcription::GEMINI_2_0_FLASH),
                ..Default::default()
            },
            ClientConfig {
                name: "Huggingface",
                factory_env: Box::new(huggingface::Client::from_env_boxed),
                factory_val: Box::new(huggingface::Client::from_val_boxed),
                env_variable: "HUGGINGFACE_API_KEY",
                completion_model: Some(huggingface::PHI_4),
                transcription_model: Some(huggingface::WHISPER_SMALL),
                image_generation_model: Some(huggingface::STABLE_DIFFUSION_3),
                ..Default::default()
            },
            ClientConfig {
                name: "OpenAI",
                factory_env: Box::new(openai::Client::from_env_boxed),
                factory_val: Box::new(openai::Client::from_val_boxed),
                env_variable: "OPENAI_API_KEY",
                completion_model: Some(openai::GPT_4O),
                embeddings_model: Some(openai::TEXT_EMBEDDING_ADA_002),
                transcription_model: Some(openai::WHISPER_1),
                image_generation_model: Some(openai::DALL_E_2),
                audio_generation_model: Some((openai::TTS_1, "onyx")),
            },
            ClientConfig {
                name: "OpenRouter",
                factory_env: Box::new(openrouter::Client::from_env_boxed),
                factory_val: Box::new(openrouter::Client::from_val_boxed),
                env_variable: "OPENROUTER_API_KEY",
                completion_model: Some(openrouter::CLAUDE_3_7_SONNET),
                ..Default::default()
            },
            ClientConfig {
                name: "Together",
                factory_env: Box::new(together::Client::from_env_boxed),
                factory_val: Box::new(together::Client::from_val_boxed),
                env_variable: "TOGETHER_API_KEY",
                completion_model: Some(together::ALPACA_7B),
                embeddings_model: Some(together::BERT_BASE_UNCASED),
                ..Default::default()
            },
            ClientConfig {
                name: "XAI",
                factory_env: Box::new(xai::Client::from_env_boxed),
                factory_val: Box::new(xai::Client::from_val_boxed),
                env_variable: "XAI_API_KEY",
                completion_model: Some(xai::GROK_3_MINI),
                embeddings_model: None,
                ..Default::default()
            },
            ClientConfig {
                name: "Azure",
                factory_env: Box::new(azure::Client::from_env_boxed),
                factory_val: Box::new(azure::Client::from_val_boxed),
                env_variable: "AZURE_API_KEY",
                completion_model: Some(azure::GPT_4O),
                embeddings_model: Some(azure::TEXT_EMBEDDING_ADA_002),
                transcription_model: Some("whisper-1"),
                image_generation_model: Some("dalle-2"),
                audio_generation_model: Some(("tts-1", "onyx")),
            },
            ClientConfig {
                name: "Deepseek",
                factory_env: Box::new(deepseek::Client::from_env_boxed),
                factory_val: Box::new(deepseek::Client::from_val_boxed),
                env_variable: "DEEPSEEK_API_KEY",
                completion_model: Some(deepseek::DEEPSEEK_CHAT),
                ..Default::default()
            },
            ClientConfig {
                name: "Galadriel",
                factory_env: Box::new(galadriel::Client::from_env_boxed),
                factory_val: Box::new(galadriel::Client::from_val_boxed),
                env_variable: "GALADRIEL_API_KEY",
                completion_model: Some(galadriel::GPT_4O),
                ..Default::default()
            },
            ClientConfig {
                name: "Groq",
                factory_env: Box::new(groq::Client::from_env_boxed),
                factory_val: Box::new(groq::Client::from_val_boxed),
                env_variable: "GROQ_API_KEY",
                completion_model: Some(groq::MIXTRAL_8X7B_32768),
                transcription_model: Some(groq::DISTIL_WHISPER_LARGE_V3),
                ..Default::default()
            },
            ClientConfig {
                name: "Hyperbolic",
                factory_env: Box::new(hyperbolic::Client::from_env_boxed),
                factory_val: Box::new(hyperbolic::Client::from_val_boxed),
                env_variable: "HYPERBOLIC_API_KEY",
                completion_model: Some(hyperbolic::LLAMA_3_1_8B),
                image_generation_model: Some(hyperbolic::SD1_5),
                audio_generation_model: Some(("EN", "EN-US")),
                ..Default::default()
            },
            ClientConfig {
                name: "Mira",
                factory_env: Box::new(mira::Client::from_env_boxed),
                factory_val: Box::new(mira::Client::from_val_boxed),
                env_variable: "MIRA_API_KEY",
                completion_model: Some("gpt-4o"),
                ..Default::default()
            },
            ClientConfig {
                name: "Moonshot",
                factory_env: Box::new(moonshot::Client::from_env_boxed),
                factory_val: Box::new(moonshot::Client::from_val_boxed),
                env_variable: "MOONSHOT_API_KEY",
                completion_model: Some(moonshot::MOONSHOT_CHAT),
                ..Default::default()
            },
            ClientConfig {
                name: "Ollama",
                factory_env: Box::new(ollama::Client::from_env_boxed),
                factory_val: Box::new(ollama::Client::from_val_boxed),
                env_variable: "OLLAMA_ENABLED",
                completion_model: Some("llama3.1:8b"),
                embeddings_model: Some(ollama::NOMIC_EMBED_TEXT),
                ..Default::default()
            },
            ClientConfig {
                name: "Perplexity",
                factory_env: Box::new(perplexity::Client::from_env_boxed),
                factory_val: Box::new(perplexity::Client::from_val_boxed),
                env_variable: "PERPLEXITY_API_KEY",
                completion_model: Some(perplexity::SONAR),
                ..Default::default()
            },
        ]
    }

    async fn test_completions_client(config: &ClientConfig) {
        let client = config.factory_env();

        let Some(client) = client.as_completion() else {
            return;
        };

        let model = config
            .completion_model
            .unwrap_or_else(|| panic!("{} does not have completion_model set", config.name));

        let model = client.completion_model(model);

        let resp = model
            .completion_request(Message::user("Whats the capital of France?"))
            .send()
            .await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when prompting, {}",
            config.name,
            resp.err().unwrap()
        );

        let resp = resp.unwrap();

        match resp.choice.first() {
            AssistantContent::Text(text) => {
                assert!(text.text.to_lowercase().contains("paris"));
            }
            _ => {
                unreachable!(
                    "[{}]: First choice wasn't a Text message, {:?}",
                    config.name,
                    resp.choice.first()
                );
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_completions() {
        for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_completions_client(&p).await;
        }
    }

    async fn test_tools_client(config: &ClientConfig) {
        let client = config.factory_env();
        let model = config
            .completion_model
            .unwrap_or_else(|| panic!("{} does not have the model set.", config.name));

        let Some(client) = client.as_completion() else {
            return;
        };

        let model = client.agent(model)
            .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.")
            .max_tokens(1024)
            .tool(Adder)
            .tool(Subtract)
            .build();

        let request = model.completion("Calculate 2 - 5", vec![]).await;

        assert!(
            request.is_ok(),
            "[{}]: Error occurred when building prompt, {}",
            config.name,
            request.err().unwrap()
        );

        let resp = request.unwrap().send().await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when prompting, {}",
            config.name,
            resp.err().unwrap()
        );

        let resp = resp.unwrap();

        assert!(
            resp.choice.iter().any(|content| match content {
                AssistantContent::ToolCall(tc) => {
                    if tc.function.name != Subtract::NAME {
                        return false;
                    }

                    let arguments =
                        serde_json::from_value::<OperationArgs>((tc.function.arguments).clone())
                            .expect("Error parsing arguments");

                    arguments.x == 2.0 && arguments.y == 5.0
                }
                _ => false,
            }),
            "[{}]: Model did not use the Subtract tool.",
            config.name
        )
    }

    #[tokio::test]
    #[ignore]
    async fn test_tools() {
        for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_tools_client(&p).await;
        }
    }

    async fn test_streaming_client(config: &ClientConfig) {
        let client = config.factory_env();

        let Some(client) = client.as_completion() else {
            return;
        };

        let model = config
            .completion_model
            .unwrap_or_else(|| panic!("{} does not have the model set.", config.name));

        let model = client.completion_model(model);

        let resp = model.stream(CompletionRequest {
            preamble: None,
            tools: vec![],
            documents: vec![],
            temperature: None,
            max_tokens: None,
            additional_params: None,
            tool_choice: None,
            chat_history: OneOrMany::one(Message::user("What is the capital of France?")),
        });

        let mut resp = resp.await.unwrap();

        let mut received_chunk = false;

        while let Some(chunk) = resp.next().await {
            received_chunk = true;
            assert!(chunk.is_ok());
        }

        assert!(
            received_chunk,
            "[{}]: Failed to receive a chunk from stream",
            config.name
        );

        for choice in resp.choice {
            match choice {
                AssistantContent::Text(text) => {
                    assert!(
                        text.text.to_lowercase().contains("paris"),
                        "[{}]: Did not answer with Paris",
                        config.name
                    );
                }
                AssistantContent::ToolCall(_) => {}
                AssistantContent::Reasoning(_) => {}
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_streaming() {
        for provider in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_streaming_client(&provider).await;
        }
    }

    async fn test_streaming_tools_client(config: &ClientConfig) {
        let client = config.factory_env();
        let model = config
            .completion_model
            .unwrap_or_else(|| panic!("{} does not have the model set.", config.name));

        let Some(client) = client.as_completion() else {
            return;
        };

        let model = client.agent(model)
            .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.")
            .max_tokens(1024)
            .tool(Adder)
            .tool(Subtract)
            .build();

        let request = model.stream_completion("Calculate 2 - 5", vec![]).await;

        assert!(
            request.is_ok(),
            "[{}]: Error occurred when building prompt, {}",
            config.name,
            request.err().unwrap()
        );

        let resp = request.unwrap().stream().await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when prompting, {}",
            config.name,
            resp.err().unwrap()
        );

        let mut resp = resp.unwrap();

        let mut received_chunk = false;

        while let Some(chunk) = resp.next().await {
            received_chunk = true;
            assert!(chunk.is_ok());
        }

        assert!(
            received_chunk,
            "[{}]: Failed to receive a chunk from stream",
            config.name
        );

        assert!(
            resp.choice.iter().any(|content| match content {
                AssistantContent::ToolCall(tc) => {
                    if tc.function.name != Subtract::NAME {
                        return false;
                    }

                    let arguments =
                        serde_json::from_value::<OperationArgs>((tc.function.arguments).clone())
                            .expect("Error parsing arguments");

                    arguments.x == 2.0 && arguments.y == 5.0
                }
                _ => false,
            }),
            "[{}]: Model did not use the Subtract tool.",
            config.name
        )
    }

    #[tokio::test]
    #[ignore]
    async fn test_streaming_tools() {
        for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_streaming_tools_client(&p).await;
        }
    }

    async fn test_audio_generation_client(config: &ClientConfig) {
        let client = config.factory_env();

        let Some(client) = client.as_audio_generation() else {
            return;
        };

        let (model, voice) = config
            .audio_generation_model
            .unwrap_or_else(|| panic!("{} doesn't have the model set", config.name));

        let model = client.audio_generation_model(model);

        let request = model
            .audio_generation_request()
            .text("Hello world!")
            .voice(voice);

        let resp = request.send().await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when sending request, {}",
            config.name,
            resp.err().unwrap()
        );

        let resp = resp.unwrap();

        assert!(
            !resp.audio.is_empty(),
            "[{}]: Returned audio was empty",
            config.name
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_audio_generation() {
        for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_audio_generation_client(&p).await;
        }
    }

    fn assert_feature<F, M>(
        name: &str,
        feature_name: &str,
        model_name: &str,
        feature: Option<F>,
        model: Option<M>,
    ) {
        assert_eq!(
            feature.is_some(),
            model.is_some(),
            "{} has{} implemented {} but config.{} is {}.",
            name,
            if feature.is_some() { "" } else { "n't" },
            feature_name,
            model_name,
            if model.is_some() { "some" } else { "none" }
        );
    }

    #[test]
    #[ignore]
    pub fn test_polymorphism() {
        for config in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            let client = config.factory_env();
            assert_feature(
                config.name,
                "AsCompletion",
                "completion_model",
                client.as_completion(),
                config.completion_model,
            );

            assert_feature(
                config.name,
                "AsEmbeddings",
                "embeddings_model",
                client.as_embeddings(),
                config.embeddings_model,
            );

            assert_feature(
                config.name,
                "AsTranscription",
                "transcription_model",
                client.as_transcription(),
                config.transcription_model,
            );

            assert_feature(
                config.name,
                "AsImageGeneration",
                "image_generation_model",
                client.as_image_generation(),
                config.image_generation_model,
            );

            assert_feature(
                config.name,
                "AsAudioGeneration",
                "audio_generation_model",
                client.as_audio_generation(),
                config.audio_generation_model,
            )
        }
    }

    async fn test_embed_client(config: &ClientConfig) {
        const TEST: &str = "Hello world.";

        let client = config.factory_env();

        let Some(client) = client.as_embeddings() else {
            return;
        };

        let model = config.embeddings_model.unwrap();

        let model = client.embedding_model(model);

        let resp = model.embed_text(TEST).await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when sending request, {}",
            config.name,
            resp.err().unwrap()
        );

        let resp = resp.unwrap();

        assert_eq!(resp.document, TEST);

        assert!(
            !resp.vec.is_empty(),
            "[{}]: Returned embed was empty",
            config.name
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_embed() {
        for config in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_embed_client(&config).await;
        }
    }

    async fn test_image_generation_client(config: &ClientConfig) {
        let client = config.factory_env();
        let Some(client) = client.as_image_generation() else {
            return;
        };

        let model = config.image_generation_model.unwrap();

        let model = client.image_generation_model(model);

        let resp = model
            .image_generation(ImageGenerationRequest {
                prompt: "A castle sitting on a large hill.".to_string(),
                width: 256,
                height: 256,
                additional_params: None,
            })
            .await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when sending request, {}",
            config.name,
            resp.err().unwrap()
        );

        let resp = resp.unwrap();

        assert!(
            !resp.image.is_empty(),
            "[{}]: Generated image was empty",
            config.name
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_image_generation() {
        for config in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_image_generation_client(&config).await;
        }
    }

    async fn test_transcription_client(config: &ClientConfig, data: Vec<u8>) {
        let client = config.factory_env();
        let Some(client) = client.as_transcription() else {
            return;
        };

        let model = config.image_generation_model.unwrap();

        let model = client.transcription_model(model);

        let resp = model
            .transcription(TranscriptionRequest {
                data,
                filename: "audio.mp3".to_string(),
                language: "en".to_string(),
                prompt: None,
                temperature: None,
                additional_params: None,
            })
            .await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when sending request, {}",
            config.name,
            resp.err().unwrap()
        );

        let resp = resp.unwrap();

        assert!(
            !resp.text.is_empty(),
            "[{}]: Returned transcription was empty",
            config.name
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_transcription() {
        let mut file = File::open("examples/audio/en-us-natural-speech.mp3").unwrap();

        let mut data = Vec::new();
        let _ = file.read(&mut data);

        for config in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_transcription_client(&config, data.clone()).await;
        }
    }

    // 派生反序列化 trait 用于操作参数结构体
    #[derive(Deserialize)]
    // 定义操作参数结构体
    struct OperationArgs {
        // 第一个操作数
        x: f32,
        // 第二个操作数
        y: f32,
    }

    // 派生 Debug 和 Error trait 用于数学错误
    #[derive(Debug, thiserror::Error)]
    // 设置错误消息
    #[error("Math error")]
    // 定义数学错误结构体
    struct MathError;

    // 派生反序列化和序列化 trait 用于加法器工具
    #[derive(Deserialize, Serialize)]
    // 定义加法器工具结构体
    struct Adder;
    // 为加法器实现 Tool trait
    impl Tool for Adder {
        // 设置工具名称为 "add"
        const NAME: &'static str = "add";

        // 设置错误类型为数学错误
        type Error = MathError;
        // 设置参数类型为操作参数
        type Args = OperationArgs;
        // 设置输出类型为 f32
        type Output = f32;

        // 异步获取工具定义
        async fn definition(&self, _prompt: String) -> ToolDefinition {
            // 返回工具定义
            ToolDefinition {
                // 设置工具名称
                name: "add".to_string(),
                // 设置工具描述
                description: "Add x and y together".to_string(),
                // 设置工具参数 JSON Schema
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "x": {
                            "type": "number",
                            "description": "The first number to add"
                        },
                        "y": {
                            "type": "number",
                            "description": "The second number to add"
                        }
                    }
                }),
            }
        }

        // 异步调用工具
        async fn call(&self, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
            // 打印工具调用信息
            println!("[tool-call] Adding {} and {}", args.x, args.y);
            // 计算加法结果
            let result = args.x + args.y;
            // 返回结果
            Ok(result)
        }
    }

    // 派生反序列化和序列化 trait 用于减法器工具
    #[derive(Deserialize, Serialize)]
    // 定义减法器工具结构体
    struct Subtract;
    // 为减法器实现 Tool trait
    impl Tool for Subtract {
        // 设置工具名称为 "subtract"
        const NAME: &'static str = "subtract";

        // 设置错误类型为数学错误
        type Error = MathError;
        // 设置参数类型为操作参数
        type Args = OperationArgs;
        // 设置输出类型为 f32
        type Output = f32;

        // 异步获取工具定义
        async fn definition(&self, _prompt: String) -> ToolDefinition {
            // 从 JSON 值反序列化工具定义
            serde_json::from_value(json!({
                "name": "subtract",
                "description": "Subtract y from x (i.e.: x - y)",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "x": {
                            "type": "number",
                            "description": "The number to subtract from"
                        },
                        "y": {
                            "type": "number",
                            "description": "The number to subtract"
                        }
                    }
                }
            }))
            // 期望反序列化成功
            .expect("Tool Definition")
        }

        // 异步调用工具
        async fn call(&self, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
            // 打印工具调用信息
            println!("[tool-call] Subtracting {} from {}", args.y, args.x);
            // 计算减法结果
            let result = args.x - args.y;
            // 返回结果
            Ok(result)
        }
    }
}
