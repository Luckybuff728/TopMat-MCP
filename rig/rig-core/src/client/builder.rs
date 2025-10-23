// 导入代理相关类型
use crate::agent::Agent;
// 导入提供商客户端 trait
use crate::client::ProviderClient;
// 导入完成请求和消息类型
use crate::completion::{CompletionRequest, Message};
// 导入嵌入模型动态类型
use crate::embeddings::embedding::EmbeddingModelDyn;
// 导入所有支持的 LLM 提供商模块
use crate::providers::{
    anthropic, azure, cohere, deepseek, galadriel, gemini, groq, huggingface, hyperbolic, mira,
    moonshot, ollama, openai, openrouter, perplexity, together, xai,
};
// 导入流式完成响应类型
use crate::streaming::StreamingCompletionResponse;
// 导入转录模型动态类型
use crate::transcription::TranscriptionModelDyn;
// 导入完成模型动态类型
use rig::completion::CompletionModelDyn;
// 导入 HashMap 用于存储提供商映射
use std::collections::HashMap;
// 导入 panic 安全相关 trait
use std::panic::{RefUnwindSafe, UnwindSafe};
// 导入错误处理宏
use thiserror::Error;

// 派生 Debug 和 Error trait 用于错误处理
#[derive(Debug, Error)]
// 定义客户端构建错误枚举
pub enum ClientBuildError {
    // 工厂错误，包含错误消息字符串
    #[error("factory error: {}", .0)]
    FactoryError(String),
    // 无效的 ID 字符串错误
    #[error("invalid id string: {}", .0)]
    InvalidIdString(String),
    // 不支持的功能错误，包含提供商和功能信息
    #[error("unsupported feature: {} for {}", .1, .0)]
    UnsupportedFeature(String, String),
    // 未知提供商错误
    #[error("unknown provider")]
    UnknownProvider,
}

// 定义完成模型的装箱类型别名，包含生命周期参数
pub type BoxCompletionModel<'a> = Box<dyn CompletionModelDyn + 'a>;
// 定义代理构建器的装箱类型别名
pub type BoxAgentBuilder<'a> = AgentBuilder<CompletionModelHandle<'a>>;
// 定义代理的装箱类型别名
pub type BoxAgent<'a> = Agent<CompletionModelHandle<'a>>;
// 定义嵌入模型的装箱类型别名
pub type BoxEmbeddingModel<'a> = Box<dyn EmbeddingModelDyn + 'a>;
// 定义转录模型的装箱类型别名
pub type BoxTranscriptionModel<'a> = Box<dyn TranscriptionModelDyn + 'a>;

/// 动态客户端构建器。
/// 当您需要支持从一系列 LLM 提供商（Rig 支持的）创建任何类型的客户端时使用此构建器。
/// 用法：
/// ```rust
/// use rig::{
///     client::builder::DynClientBuilder, completion::Prompt, providers::anthropic::CLAUDE_3_7_SONNET,
/// };
/// #[tokio::main]
/// async fn main() {
///     let multi_client = DynClientBuilder::new();
///     // 设置 OpenAI 客户端
///     let completion_openai = multi_client.agent("openai", "gpt-4o").unwrap();
///     let agent_openai = completion_openai
///         .preamble("You are a helpful assistant")
///         .build();
///     // 设置 Anthropic 客户端
///     let completion_anthropic = multi_client.agent("anthropic", CLAUDE_3_7_SONNET).unwrap();
///     let agent_anthropic = completion_anthropic
///         .preamble("You are a helpful assistant")
///         .max_tokens(1024)
///         .build();
///     println!("Sending prompt: 'Hello world!'");
///     let res_openai = agent_openai.prompt("Hello world!").await.unwrap();
///     println!("Response from OpenAI (using gpt-4o): {res_openai}");
///     let res_anthropic = agent_anthropic.prompt("Hello world!").await.unwrap();
///     println!("Response from Anthropic (using Claude 3.7 Sonnet): {res_anthropic}");
/// }
/// ```
// 定义动态客户端构建器结构体
pub struct DynClientBuilder {
    // 注册表，存储提供商名称到客户端工厂的映射
    registry: HashMap<String, ClientFactory>,
}

// 为 DynClientBuilder 实现 Default trait
impl Default for DynClientBuilder {
    // 提供默认实现，调用 new() 方法
    fn default() -> Self {
        Self::new()
    }
}

// 为 DynClientBuilder 实现方法
impl<'a> DynClientBuilder {
    /// 生成 `DynClientBuilder` 的新实例。
    /// 默认情况下，每个可以注册的客户端
    /// 都会被注册到客户端构建器中。
    // 创建新的动态客户端构建器实例
    pub fn new() -> Self {
        Self {
            // 创建空的注册表 HashMap
            registry: HashMap::new(),
        }
        // 注册所有支持的提供商
        .register_all(vec![
            // 注册 Anthropic 提供商
            ClientFactory::new(
                DefaultProviders::ANTHROPIC,
                anthropic::Client::from_env_boxed,
                anthropic::Client::from_val_boxed,
            ),
            // 注册 Cohere 提供商
            ClientFactory::new(
                DefaultProviders::COHERE,
                cohere::Client::from_env_boxed,
                cohere::Client::from_val_boxed,
            ),
            // 注册 Gemini 提供商
            ClientFactory::new(
                DefaultProviders::GEMINI,
                gemini::Client::from_env_boxed,
                gemini::Client::from_val_boxed,
            ),
            // 注册 Huggingface 提供商
            ClientFactory::new(
                DefaultProviders::HUGGINGFACE,
                huggingface::Client::from_env_boxed,
                huggingface::Client::from_val_boxed,
            ),
            // 注册 OpenAI 提供商
            ClientFactory::new(
                DefaultProviders::OPENAI,
                openai::Client::from_env_boxed,
                openai::Client::from_val_boxed,
            ),
            // 注册 OpenRouter 提供商
            ClientFactory::new(
                DefaultProviders::OPENROUTER,
                openrouter::Client::from_env_boxed,
                openrouter::Client::from_val_boxed,
            ),
            // 注册 Together 提供商
            ClientFactory::new(
                DefaultProviders::TOGETHER,
                together::Client::from_env_boxed,
                together::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::XAI,
                xai::Client::from_env_boxed,
                xai::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::AZURE,
                azure::Client::from_env_boxed,
                azure::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::DEEPSEEK,
                deepseek::Client::from_env_boxed,
                deepseek::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::GALADRIEL,
                galadriel::Client::from_env_boxed,
                galadriel::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::GROQ,
                groq::Client::from_env_boxed,
                groq::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::HYPERBOLIC,
                hyperbolic::Client::from_env_boxed,
                hyperbolic::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::MOONSHOT,
                moonshot::Client::from_env_boxed,
                moonshot::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::MIRA,
                mira::Client::from_env_boxed,
                mira::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::MISTRAL,
                mistral::Client::from_env_boxed,
                mistral::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::OLLAMA,
                ollama::Client::from_env_boxed,
                ollama::Client::from_val_boxed,
            ),
            ClientFactory::new(
                DefaultProviders::PERPLEXITY,
                perplexity::Client::from_env_boxed,
                perplexity::Client::from_val_boxed,
            ),
        ])
    }

    /// 生成一个没有注册任何客户端工厂的 `DynClientBuilder` 新实例。
    // 创建空的动态客户端构建器
    pub fn empty() -> Self {
        Self {
            // 创建空的注册表
            registry: HashMap::new(),
        }
    }

    /// 注册一个新的 ClientFactory
    // 注册单个客户端工厂，返回修改后的构建器
    pub fn register(mut self, client_factory: ClientFactory) -> Self {
        // 将工厂插入到注册表中
        self.registry
            .insert(client_factory.name.clone(), client_factory);
        // 返回修改后的构建器
        self
    }

    /// 注册多个 ClientFactory
    // 批量注册客户端工厂
    pub fn register_all(mut self, factories: impl IntoIterator<Item = ClientFactory>) -> Self {
        // 遍历所有工厂并插入到注册表
        for factory in factories {
            self.registry.insert(factory.name.clone(), factory);
        }

        // 返回修改后的构建器
        self
    }

    /// 根据给定的提供商返回一个（装箱的）特定提供商。
    // 构建指定提供商的客户端
    pub fn build(&self, provider: &str) -> Result<Box<dyn ProviderClient>, ClientBuildError> {
        // 获取对应的工厂
        let factory = self.get_factory(provider)?;
        // 使用工厂构建客户端
        factory.build()
    }

    /// 根据给定的提供商返回一个（装箱的）特定提供商。
    // 使用提供商值构建客户端
    pub fn build_val(
        &self,
        provider: &str,
        provider_value: ProviderValue,
    ) -> Result<Box<dyn ProviderClient>, ClientBuildError> {
        // 获取对应的工厂
        let factory = self.get_factory(provider)?;
        // 使用工厂和提供商值构建客户端
        factory.build_from_val(provider_value)
    }

    /// 解析 provider:model 字符串，分别返回提供商和模型。
    /// 例如，`openai:gpt-4o` 将返回 ("openai", "gpt-4o")。
    // 解析提供商和模型字符串
    pub fn parse(&self, id: &'a str) -> Result<(&'a str, &'a str), ClientBuildError> {
        // 按冒号分割字符串
        let (provider, model) = id
            .split_once(":")
            .ok_or(ClientBuildError::InvalidIdString(id.to_string()))?;

        // 返回解析后的提供商和模型
        Ok((provider, model))
    }

    /// 返回一个特定的客户端工厂（存在于注册表中）。
    // 获取指定提供商的工厂
    fn get_factory(&self, provider: &str) -> Result<&ClientFactory, ClientBuildError> {
        // 从注册表中查找工厂，如果不存在则返回未知提供商错误
        self.registry
            .get(provider)
            .ok_or(ClientBuildError::UnknownProvider)
    }

    /// 根据提供商和模型获取装箱的完成模型。
    // 创建完成模型
    pub fn completion(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<BoxCompletionModel<'a>, ClientBuildError> {
        // 构建客户端
        let client = self.build(provider)?;

        // 获取完成功能，如果不支持则返回错误
        let completion = client
            .as_completion()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "completion".to_owned(),
            ))?;

        Ok(completion.completion_model(model))
    }

    /// Get a boxed agent based on the provider and model..
    // 根据提供商和模型获取装箱的代理构建器
    pub fn agent(
        // 接收自身引用
        &self,
        // 提供商名称
        provider: &str,
        // 模型名称
        model: &str,
    ) -> Result<BoxAgentBuilder<'a>, ClientBuildError> {
        // 构建客户端
        let client = self.build(provider)?;

        // 获取完成功能，如果不支持则返回错误
        let client = client
            .as_completion()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "completion".to_string(),
            ))?;

        // 返回代理构建器
        Ok(client.agent(model))
    }

    /// Get a boxed agent based on the provider and model, as well as an API key.
    // 根据提供商、模型和 API 密钥获取装箱的代理构建器
    pub fn agent_with_api_key_val<P>(
        // 接收自身引用
        &self,
        // 提供商名称
        provider: &str,
        // 模型名称
        model: &str,
        // 提供商值（API 密钥等）
        provider_value: P,
    ) -> Result<BoxAgentBuilder<'a>, ClientBuildError>
    where
        // P 必须能够转换为 ProviderValue
        P: Into<ProviderValue>,
    {
        // 使用提供商值构建客户端
        let client = self.build_val(provider, provider_value.into())?;

        // 获取完成功能，如果不支持则返回错误
        let client = client
            .as_completion()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "completion".to_string(),
            ))?;

        // 返回代理构建器
        Ok(client.agent(model))
    }

    /// Get a boxed embedding model based on the provider and model.
    // 根据提供商和模型获取装箱的嵌入模型
    pub fn embeddings(
        // 接收自身引用
        &self,
        // 提供商名称
        provider: &str,
        // 模型名称
        model: &str,
    ) -> Result<Box<dyn EmbeddingModelDyn + 'a>, ClientBuildError> {
        // 构建客户端
        let client = self.build(provider)?;

        // 获取嵌入功能，如果不支持则返回错误
        let embeddings = client
            .as_embeddings()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "embeddings".to_owned(),
            ))?;

        // 返回嵌入模型
        Ok(embeddings.embedding_model(model))
    }

    /// Get a boxed embedding model based on the provider and model.
    // 根据提供商、模型和 API 密钥获取装箱的嵌入模型
    pub fn embeddings_with_api_key_val<P>(
        // 接收自身引用
        &self,
        // 提供商名称
        provider: &str,
        // 模型名称
        model: &str,
        // 提供商值（API 密钥等）
        provider_value: P,
    ) -> Result<Box<dyn EmbeddingModelDyn + 'a>, ClientBuildError>
    where
        // P 必须能够转换为 ProviderValue
        P: Into<ProviderValue>,
    {
        // 使用提供商值构建客户端
        let client = self.build_val(provider, provider_value.into())?;

        // 获取嵌入功能，如果不支持则返回错误
        let embeddings = client
            .as_embeddings()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "embeddings".to_owned(),
            ))?;

        // 返回嵌入模型
        Ok(embeddings.embedding_model(model))
    }

    /// Get a boxed transcription model based on the provider and model.
    // 根据提供商和模型获取装箱的转录模型
    pub fn transcription(
        // 接收自身引用
        &self,
        // 提供商名称
        provider: &str,
        // 模型名称
        model: &str,
    ) -> Result<Box<dyn TranscriptionModelDyn + 'a>, ClientBuildError> {
        // 构建客户端
        let client = self.build(provider)?;
        // 获取转录功能，如果不支持则返回错误
        let transcription =
            client
                .as_transcription()
                .ok_or(ClientBuildError::UnsupportedFeature(
                    provider.to_string(),
                    "transcription".to_owned(),
                ))?;

        // 返回转录模型
        Ok(transcription.transcription_model(model))
    }

    /// Get a boxed transcription model based on the provider and model.
    // 根据提供商、模型和 API 密钥获取装箱的转录模型
    pub fn transcription_with_api_key_val<P>(
        // 接收自身引用
        &self,
        // 提供商名称
        provider: &str,
        // 模型名称
        model: &str,
        // 提供商值（API 密钥等）
        provider_value: P,
    ) -> Result<Box<dyn TranscriptionModelDyn + 'a>, ClientBuildError>
    where
        // P 必须能够转换为 ProviderValue
        P: Into<ProviderValue>,
    {
        // 使用提供商值构建客户端
        let client = self.build_val(provider, provider_value.into())?;
        // 获取转录功能，如果不支持则返回错误
        let transcription =
            client
                .as_transcription()
                .ok_or(ClientBuildError::UnsupportedFeature(
                    provider.to_string(),
                    "transcription".to_owned(),
                ))?;

        // 返回转录模型
        Ok(transcription.transcription_model(model))
    }

    /// Get the ID of a provider model based on a `provider:model` ID.
    // 根据 `provider:model` ID 获取提供商模型 ID
    pub fn id<'id>(&'a self, id: &'id str) -> Result<ProviderModelId<'a, 'id>, ClientBuildError> {
        // 解析提供商和模型
        let (provider, model) = self.parse(id)?;

        // 返回提供商模型 ID
        Ok(ProviderModelId {
            // 设置构建器引用
            builder: self,
            // 设置提供商
            provider,
            // 设置模型
            model,
        })
    }

    /// Stream a completion request to the specified provider and model.
    ///
    /// # Arguments
    /// * `provider` - The name of the provider (e.g., "openai", "anthropic")
    /// * `model` - The name of the model (e.g., "gpt-4o", "claude-3-sonnet")
    /// * `request` - The completion request containing prompt, parameters, etc.
    ///
    /// # Returns
    /// A future that resolves to a streaming completion response
    // 将完成请求流式传输到指定的提供商和模型
    //
    // # 参数
    // * `provider` - 提供商名称（例如："openai", "anthropic"）
    // * `model` - 模型名称（例如："gpt-4o", "claude-3-sonnet"）
    // * `request` - 包含提示、参数等的完成请求
    //
    // # 返回值
    // 解析为流式完成响应的 Future
    pub async fn stream_completion(
        // 接收自身引用
        &self,
        // 提供商名称
        provider: &str,
        // 模型名称
        model: &str,
        // 完成请求
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<()>, ClientBuildError> {
        // 构建客户端
        let client = self.build(provider)?;
        // 获取完成功能，如果不支持则返回错误
        let completion = client
            .as_completion()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "completion".to_string(),
            ))?;

        // 获取完成模型
        let model = completion.completion_model(model);
        // 流式处理请求
        model
            .stream(request)
            .await
            // 将错误映射为工厂错误
            .map_err(|e| ClientBuildError::FactoryError(e.to_string()))
    }

    /// Stream a simple prompt to the specified provider and model.
    ///
    /// # Arguments
    /// * `provider` - The name of the provider (e.g., "openai", "anthropic")
    /// * `model` - The name of the model (e.g., "gpt-4o", "claude-3-sonnet")
    /// * `prompt` - The prompt to send to the model
    ///
    /// # Returns
    /// A future that resolves to a streaming completion response
    // 将简单提示流式传输到指定的提供商和模型
    //
    // # 参数
    // * `provider` - 提供商名称（例如："openai", "anthropic"）
    // * `model` - 模型名称（例如："gpt-4o", "claude-3-sonnet"）
    // * `prompt` - 发送到模型的提示
    //
    // # 返回值
    // 解析为流式完成响应的 Future
    pub async fn stream_prompt(
        // 接收自身引用
        &self,
        // 提供商名称
        provider: &str,
        // 模型名称
        model: &str,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
    ) -> Result<StreamingCompletionResponse<()>, ClientBuildError> {
        // 构建客户端
        let client = self.build(provider)?;
        // 获取完成功能，如果不支持则返回错误
        let completion = client
            .as_completion()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "completion".to_string(),
            ))?;

        // 获取完成模型
        let model = completion.completion_model(model);
        // 创建完成请求
        let request = CompletionRequest {
            // 设置前言为 None
            preamble: None,
            // 设置工具为空向量
            tools: vec![],
            // 设置文档为空向量
            documents: vec![],
            // 设置温度为 None
            temperature: None,
            // 设置最大 token 数为 None
            max_tokens: None,
            // 设置附加参数为 None
            additional_params: None,
            // 设置工具选择为 None
            tool_choice: None,
            // 设置聊天历史为单个提示
            chat_history: crate::OneOrMany::one(prompt.into()),
        };

        // 流式处理请求
        model
            .stream(request)
            .await
            // 将错误映射为工厂错误
            .map_err(|e| ClientBuildError::FactoryError(e.to_string()))
    }

    /// Stream a chat with history to the specified provider and model.
    ///
    /// # Arguments
    /// * `provider` - The name of the provider (e.g., "openai", "anthropic")
    /// * `model` - The name of the model (e.g., "gpt-4o", "claude-3-sonnet")
    /// * `prompt` - The new prompt to send to the model
    /// * `chat_history` - The chat history to include with the request
    ///
    /// # Returns
    /// A future that resolves to a streaming completion response
    // 将带历史的聊天流式传输到指定的提供商和模型
    //
    // # 参数
    // * `provider` - 提供商名称（例如："openai", "anthropic"）
    // * `model` - 模型名称（例如："gpt-4o", "claude-3-sonnet"）
    // * `prompt` - 发送到模型的新提示
    // * `chat_history` - 包含在请求中的聊天历史
    //
    // # 返回值
    // 解析为流式完成响应的 Future
    pub async fn stream_chat(
        // 接收自身引用
        &self,
        // 提供商名称
        provider: &str,
        // 模型名称
        model: &str,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
        // 聊天历史
        chat_history: Vec<Message>,
    ) -> Result<StreamingCompletionResponse<()>, ClientBuildError> {
        // 构建客户端
        let client = self.build(provider)?;
        // 获取完成功能，如果不支持则返回错误
        let completion = client
            .as_completion()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "completion".to_string(),
            ))?;

        // 获取完成模型
        let model = completion.completion_model(model);
        // 创建可变聊天历史
        let mut history = chat_history;
        // 将新提示添加到历史中
        history.push(prompt.into());

        // 创建完成请求
        let request = CompletionRequest {
            // 设置前言为 None
            preamble: None,
            // 设置工具为空向量
            tools: vec![],
            // 设置文档为空向量
            documents: vec![],
            // 设置温度为 None
            temperature: None,
            // 设置最大 token 数为 None
            max_tokens: None,
            // 设置附加参数为 None
            additional_params: None,
            // 设置工具选择为 None
            tool_choice: None,
            // 设置聊天历史为多个消息，如果失败则使用空用户消息
            chat_history: crate::OneOrMany::many(history)
                .unwrap_or_else(|_| crate::OneOrMany::one(Message::user(""))),
        };

        // 流式处理请求
        model
            .stream(request)
            .await
            // 将错误映射为工厂错误
            .map_err(|e| ClientBuildError::FactoryError(e.to_string()))
    }
}

// 定义提供商模型 ID 结构体，包含构建器引用和提供商、模型信息
pub struct ProviderModelId<'builder, 'id> {
    // 构建器引用
    builder: &'builder DynClientBuilder,
    // 提供商名称
    provider: &'id str,
    // 模型名称
    model: &'id str,
}

// 为 ProviderModelId 实现方法
impl<'builder> ProviderModelId<'builder, '_> {
    // 获取完成模型
    pub fn completion(self) -> Result<BoxCompletionModel<'builder>, ClientBuildError> {
        // 调用构建器的完成方法
        self.builder.completion(self.provider, self.model)
    }

    // 获取代理构建器
    pub fn agent(self) -> Result<BoxAgentBuilder<'builder>, ClientBuildError> {
        // 调用构建器的代理方法
        self.builder.agent(self.provider, self.model)
    }

    // 获取嵌入模型
    pub fn embedding(self) -> Result<BoxEmbeddingModel<'builder>, ClientBuildError> {
        // 调用构建器的嵌入方法
        self.builder.embeddings(self.provider, self.model)
    }

    // 获取转录模型
    pub fn transcription(self) -> Result<BoxTranscriptionModel<'builder>, ClientBuildError> {
        // 调用构建器的转录方法
        self.builder.transcription(self.provider, self.model)
    }

    /// Stream a completion request using this provider and model.
    ///
    /// # Arguments
    /// * `request` - The completion request containing prompt, parameters, etc.
    ///
    /// # Returns
    /// A future that resolves to a streaming completion response
    // 使用此提供商和模型流式传输完成请求
    //
    // # 参数
    // * `request` - 包含提示、参数等的完成请求
    //
    // # 返回值
    // 解析为流式完成响应的 Future
    pub async fn stream_completion(
        // 消费自身
        self,
        // 完成请求
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<()>, ClientBuildError> {
        // 调用构建器的流式完成方法
        self.builder
            .stream_completion(self.provider, self.model, request)
            .await
    }

    /// Stream a simple prompt using this provider and model.
    ///
    /// # Arguments
    /// * `prompt` - The prompt to send to the model
    ///
    /// # Returns
    /// A future that resolves to a streaming completion response
    // 使用此提供商和模型流式传输简单提示
    //
    // # 参数
    // * `prompt` - 发送到模型的提示
    //
    // # 返回值
    // 解析为流式完成响应的 Future
    pub async fn stream_prompt(
        // 消费自身
        self,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
    ) -> Result<StreamingCompletionResponse<()>, ClientBuildError> {
        // 调用构建器的流式提示方法
        self.builder
            .stream_prompt(self.provider, self.model, prompt)
            .await
    }

    /// Stream a chat with history using this provider and model.
    ///
    /// # Arguments
    /// * `prompt` - The new prompt to send to the model
    /// * `chat_history` - The chat history to include with the request
    ///
    /// # Returns
    /// A future that resolves to a streaming completion response
    // 使用此提供商和模型流式传输带历史的聊天
    //
    // # 参数
    // * `prompt` - 发送到模型的新提示
    // * `chat_history` - 包含在请求中的聊天历史
    //
    // # 返回值
    // 解析为流式完成响应的 Future
    pub async fn stream_chat(
        // 消费自身
        self,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
        // 聊天历史
        chat_history: Vec<Message>,
    ) -> Result<StreamingCompletionResponse<()>, ClientBuildError> {
        // 调用构建器的流式聊天方法
        self.builder
            .stream_chat(self.provider, self.model, prompt, chat_history)
            .await
    }
}

// 当启用 "image" 功能时
#[cfg(feature = "image")]
// 图像生成模块
mod image {
    // 导入客户端构建错误类型
    use crate::client::builder::ClientBuildError;
    // 导入图像生成模型动态类型
    use crate::image_generation::ImageGenerationModelDyn;
    // 导入动态客户端构建器和提供商模型 ID
    use rig::client::builder::{DynClientBuilder, ProviderModelId};

    // 定义图像生成模型的装箱类型别名
    pub type BoxImageGenerationModel<'a> = Box<dyn ImageGenerationModelDyn + 'a>;

    // 为动态客户端构建器实现图像生成方法
    impl DynClientBuilder {
        // 获取图像生成模型
        pub fn image_generation<'a>(
            // 接收自身引用
            &self,
            // 提供商名称
            provider: &str,
            // 模型名称
            model: &str,
        ) -> Result<BoxImageGenerationModel<'a>, ClientBuildError> {
            // 构建客户端
            let client = self.build(provider)?;
            // 获取图像生成功能，如果不支持则返回错误
            let image =
                client
                    .as_image_generation()
                    .ok_or(ClientBuildError::UnsupportedFeature(
                        provider.to_string(),
                        "image_generation".to_string(),
                    ))?;

            // 返回图像生成模型
            Ok(image.image_generation_model(model))
        }
    }

    // 为提供商模型 ID 实现图像生成方法
    impl<'builder> ProviderModelId<'builder, '_> {
        // 获取图像生成模型
        pub fn image_generation(
            // 消费自身
            self,
        ) -> Result<Box<dyn ImageGenerationModelDyn + 'builder>, ClientBuildError> {
            // 调用构建器的图像生成方法
            self.builder.image_generation(self.provider, self.model)
        }
    }
}
// 当启用 "image" 功能时，重新导出图像模块的所有公共项
#[cfg(feature = "image")]
pub use image::*;

// 当启用 "audio" 功能时
#[cfg(feature = "audio")]
// 音频生成模块
mod audio {
    // 导入音频生成模型动态类型
    use crate::audio_generation::AudioGenerationModelDyn;
    // 导入动态客户端构建器
    use crate::client::builder::DynClientBuilder;
    // 导入客户端构建错误和提供商模型 ID
    use crate::client::builder::{ClientBuildError, ProviderModelId};

    // 定义音频生成模型的装箱类型别名
    pub type BoxAudioGenerationModel<'a> = Box<dyn AudioGenerationModelDyn + 'a>;

    // 为动态客户端构建器实现音频生成方法
    impl DynClientBuilder {
        // 获取音频生成模型
        pub fn audio_generation<'a>(
            // 接收自身引用
            &self,
            // 提供商名称
            provider: &str,
            // 模型名称
            model: &str,
        ) -> Result<BoxAudioGenerationModel<'a>, ClientBuildError> {
            // 构建客户端
            let client = self.build(provider)?;
            // 获取音频生成功能，如果不支持则返回错误
            let audio =
                client
                    .as_audio_generation()
                    .ok_or(ClientBuildError::UnsupportedFeature(
                        provider.to_string(),
                        "audio_generation".to_owned(),
                    ))?;

            // 返回音频生成模型
            Ok(audio.audio_generation_model(model))
        }
    }

    // 为提供商模型 ID 实现音频生成方法
    impl<'builder> ProviderModelId<'builder, '_> {
        // 获取音频生成模型
        pub fn audio_generation(
            // 消费自身
            self,
        ) -> Result<Box<dyn AudioGenerationModelDyn + 'builder>, ClientBuildError> {
            // 调用构建器的音频生成方法
            self.builder.audio_generation(self.provider, self.model)
        }
    }
}
// 导入代理构建器类型
use crate::agent::AgentBuilder;
// 导入完成模型句柄类型
use crate::client::completion::CompletionModelHandle;
// 当启用 "audio" 功能时，重新导出音频模块的所有公共项
#[cfg(feature = "audio")]
pub use audio::*;
// 导入 Mistral 提供商
use rig::providers::mistral;

// 导入提供商值类型
use super::ProviderValue;

// 定义客户端工厂结构体
pub struct ClientFactory {
    // 工厂名称（提供商名称）
    pub name: String,
    // 从环境变量创建客户端的工厂函数
    pub factory_env: Box<dyn Fn() -> Box<dyn ProviderClient>>,
    // 从提供商值创建客户端的工厂函数
    pub factory_val: Box<dyn Fn(ProviderValue) -> Box<dyn ProviderClient>>,
}

// 实现 UnwindSafe trait，表示可以安全地跨 panic 边界传递
impl UnwindSafe for ClientFactory {}
// 实现 RefUnwindSafe trait，表示引用可以安全地跨 panic 边界传递
impl RefUnwindSafe for ClientFactory {}

// 为 ClientFactory 实现方法
impl ClientFactory {
    // 创建新的客户端工厂实例
    pub fn new<F1, F2>(name: &str, func_env: F1, func_val: F2) -> Self
    where
        // F1 是从环境变量创建客户端的函数类型
        F1: 'static + Fn() -> Box<dyn ProviderClient>,
        // F2 是从提供商值创建客户端的函数类型
        F2: 'static + Fn(ProviderValue) -> Box<dyn ProviderClient>,
    {
        Self {
            // 将名称转换为字符串
            name: name.to_string(),
            // 装箱环境工厂函数
            factory_env: Box::new(func_env),
            // 装箱值工厂函数
            factory_val: Box::new(func_val),
        }
    }

    // 从环境变量构建客户端
    pub fn build(&self) -> Result<Box<dyn ProviderClient>, ClientBuildError> {
        // 捕获 panic 并调用环境工厂函数
        std::panic::catch_unwind(|| (self.factory_env)())
            // 将 panic 错误映射为工厂错误
            .map_err(|e| ClientBuildError::FactoryError(format!("{e:?}")))
    }

    // 从提供商值构建客户端
    pub fn build_from_val(
        // 接收自身引用
        &self,
        // 提供商值
        val: ProviderValue,
    ) -> Result<Box<dyn ProviderClient>, ClientBuildError> {
        // 捕获 panic 并调用值工厂函数
        std::panic::catch_unwind(|| (self.factory_val)(val))
            // 将 panic 错误映射为工厂错误
            .map_err(|e| ClientBuildError::FactoryError(format!("{e:?}")))
    }
}

// 定义默认提供商常量结构体
pub struct DefaultProviders;
// 为 DefaultProviders 实现方法
impl DefaultProviders {
    // Anthropic 提供商常量
    pub const ANTHROPIC: &'static str = "anthropic";
    // Cohere 提供商常量
    pub const COHERE: &'static str = "cohere";
    // Gemini 提供商常量
    pub const GEMINI: &'static str = "gemini";
    // Hugging Face 提供商常量
    pub const HUGGINGFACE: &'static str = "huggingface";
    // OpenAI 提供商常量
    pub const OPENAI: &'static str = "openai";
    // OpenRouter 提供商常量
    pub const OPENROUTER: &'static str = "openrouter";
    // Together 提供商常量
    pub const TOGETHER: &'static str = "together";
    // XAI 提供商常量
    pub const XAI: &'static str = "xai";
    // Azure 提供商常量
    pub const AZURE: &'static str = "azure";
    // DeepSeek 提供商常量
    pub const DEEPSEEK: &'static str = "deepseek";
    // Galadriel 提供商常量
    pub const GALADRIEL: &'static str = "galadriel";
    // Groq 提供商常量
    pub const GROQ: &'static str = "groq";
    // Hyperbolic 提供商常量
    pub const HYPERBOLIC: &'static str = "hyperbolic";
    // Moonshot 提供商常量
    pub const MOONSHOT: &'static str = "moonshot";
    // Mira 提供商常量
    pub const MIRA: &'static str = "mira";
    // Mistral 提供商常量
    pub const MISTRAL: &'static str = "mistral";
    // Ollama 提供商常量
    pub const OLLAMA: &'static str = "ollama";
    // Perplexity 提供商常量
    pub const PERPLEXITY: &'static str = "perplexity";
}
