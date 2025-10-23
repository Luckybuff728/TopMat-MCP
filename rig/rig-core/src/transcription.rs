//! 此模块提供与音频转录模型协作的功能。
//! 它提供了用于生成音频转录请求、
//! 处理转录响应和定义转录模型的 trait、结构体和枚举。
// 此模块提供与音频转录模型协作的功能
// 它提供了用于生成音频转录请求、处理转录响应和定义转录模型的 trait、结构体和枚举

// 导入转录模型句柄类型
use crate::client::transcription::TranscriptionModelHandle;
// 导入 JSON 工具函数
use crate::json_utils;
// 导入 Future 装箱类型用于异步操作
use futures::future::BoxFuture;
// 导入 Arc 用于原子引用计数
use std::sync::Arc;
// 导入文件系统和路径相关类型
use std::{fs, path::Path};
// 导入错误处理宏
use thiserror::Error;

// 错误类型定义
// 派生 Debug 和 Error trait 用于错误处理
#[derive(Debug, Error)]
// 标记为非穷尽枚举，允许未来添加新的错误变体
#[non_exhaustive]
// 定义转录错误枚举
pub enum TranscriptionError {
    /// HTTP 错误（例如：连接错误、超时等）
    // HTTP 错误，从 reqwest::Error 自动转换
    #[error("HttpError: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON 错误（例如：序列化、反序列化）
    // JSON 错误，从 serde_json::Error 自动转换
    #[error("JsonError: {0}")]
    JsonError(#[from] serde_json::Error),

    /// 构建转录请求时的错误
    // 请求构建错误，从通用错误类型自动转换
    #[error("RequestError: {0}")]
    RequestError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// 解析转录响应时的错误
    // 响应解析错误，包含错误消息字符串
    #[error("ResponseError: {0}")]
    ResponseError(String),

    /// 转录模型提供商返回的错误
    // 提供商错误，包含错误消息字符串
    #[error("ProviderError: {0}")]
    ProviderError(String),
}

/// 定义低级 LLM 转录接口的 trait
// 定义低级 LLM 转录接口的 trait
pub trait Transcription<M>
where
    // M 必须实现 TranscriptionModel trait
    M: TranscriptionModel,
{
    /// 为给定的 `file` 生成转录请求构建器。
    /// 此函数旨在由用户调用，以在发送之前在转录时进一步自定义请求。
    ///
    /// ❗重要：实现此 trait 的类型可能已经在构建器中填充了字段（具体字段取决于类型）。
    /// 对于已由模型设置的字段，在构建器上调用相应方法将覆盖模型设置的值。
    // 为给定的文件生成转录请求构建器
    // 此函数旨在由用户调用，以在发送之前在转录时进一步自定义请求
    // 重要：实现此 trait 的类型可能已经在构建器中填充了字段（具体字段取决于类型）
    // 对于已由模型设置的字段，在构建器上调用相应方法将覆盖模型设置的值
    fn transcription(
        &self,
        filename: &str,
        data: &[u8],
    ) -> impl std::future::Future<
        Output = Result<TranscriptionRequestBuilder<M>, TranscriptionError>,
    > + Send;
}

/// 包含转录文本和原始响应的通用转录响应结构体。
// 包含转录文本和原始响应的通用转录响应结构体
pub struct TranscriptionResponse<T> {
    // 转录的文本内容
    pub text: String,
    // 原始响应对象
    pub response: T,
}

/// 定义可用于生成转录请求的转录模型的 trait。
/// 此 trait 旨在由用户实现，以定义自定义转录模型，
/// 无论是来自第三方提供商（例如：OpenAI）还是本地模型。
// 定义可用于生成转录请求的转录模型的 trait
// 此 trait 旨在由用户实现，以定义自定义转录模型
// 无论是来自第三方提供商（例如：OpenAI）还是本地模型
pub trait TranscriptionModel: Clone + Send + Sync {
    /// 底层模型返回的原始响应类型。
    // 底层模型返回的原始响应类型，必须支持同步和发送
    type Response: Sync + Send;

    /// 为给定的转录模型生成完成响应
    // 为给定的转录模型生成完成响应
    fn transcription(
        &self,
        request: TranscriptionRequest,
    ) -> impl std::future::Future<
        Output = Result<TranscriptionResponse<Self::Response>, TranscriptionError>,
    > + Send;

    /// 为给定的 `file` 生成转录请求构建器
    // 为给定的文件生成转录请求构建器
    fn transcription_request(&self) -> TranscriptionRequestBuilder<Self> {
        TranscriptionRequestBuilder::new(self.clone())
    }
}

// 动态转录模型 trait，用于动态分发
pub trait TranscriptionModelDyn: Send + Sync {
    // 执行转录请求
    fn transcription(
        &self,
        request: TranscriptionRequest,
    ) -> BoxFuture<'_, Result<TranscriptionResponse<()>, TranscriptionError>>;

    // 创建转录请求构建器
    fn transcription_request(&self) -> TranscriptionRequestBuilder<TranscriptionModelHandle<'_>>;
}

// 为所有实现 TranscriptionModel trait 的类型实现 TranscriptionModelDyn trait
impl<T> TranscriptionModelDyn for T
where
    // T 必须实现 TranscriptionModel trait
    T: TranscriptionModel,
{
    // 执行转录请求
    fn transcription(
        &self,
        request: TranscriptionRequest,
    ) -> BoxFuture<'_, Result<TranscriptionResponse<()>, TranscriptionError>> {
        // 装箱异步闭包
        Box::pin(async move {
            // 调用转录模型
            let resp = self.transcription(request).await?;

            // 返回转录响应，丢弃原始响应
            Ok(TranscriptionResponse {
                text: resp.text,
                response: (),
            })
        })
    }

    // 创建转录请求构建器
    fn transcription_request(&self) -> TranscriptionRequestBuilder<TranscriptionModelHandle<'_>> {
        // 创建转录模型句柄并构建请求构建器
        TranscriptionRequestBuilder::new(TranscriptionModelHandle {
            inner: Arc::new(self.clone()),
        })
    }
}

/// Struct representing a general transcription request that can be sent to a transcription model provider.
// 表示可以发送给转录模型提供商的通用转录请求的结构体
pub struct TranscriptionRequest {
    /// The file data to be sent to the transcription model provider
    // 要发送给转录模型提供商的文件数据
    pub data: Vec<u8>,
    /// The file name to be used in the request
    // 请求中使用的文件名
    pub filename: String,
    /// The language used in the response from the transcription model provider
    // 转录模型提供商响应中使用的语言
    pub language: String,
    /// The prompt to be sent to the transcription model provider
    // 要发送给转录模型提供商的提示
    pub prompt: Option<String>,
    /// The temperature sent to the transcription model provider
    // 发送给转录模型提供商的温度参数
    pub temperature: Option<f64>,
    /// Additional parameters to be sent to the transcription model provider
    // 要发送给转录模型提供商的额外参数
    pub additional_params: Option<serde_json::Value>,
}

/// Builder struct for a transcription request
///
/// Example usage:
/// ```rust
/// use rig::{
///     providers::openai::{Client, self},
///     transcription::TranscriptionRequestBuilder,
/// };
///
/// let openai = Client::new("your-openai-api-key");
/// let model = openai.transcription_model(openai::WHISPER_1).build();
///
/// // Create the completion request and execute it separately
/// let request = TranscriptionRequestBuilder::new(model, "~/audio.mp3".to_string())
///     .temperature(0.5)
///     .build();
///
/// let response = model.transcription(request)
///     .await
///     .expect("Failed to get transcription response");
/// ```
///
/// Alternatively, you can execute the transcription request directly from the builder:
/// ```rust
/// use rig::{
///     providers::openai::{Client, self},
///     transcription::TranscriptionRequestBuilder,
/// };
///
/// let openai = Client::new("your-openai-api-key");
/// let model = openai.transcription_model(openai::WHISPER_1).build();
///
/// // Create the completion request and execute it directly
/// let response = TranscriptionRequestBuilder::new(model, "~/audio.mp3".to_string())
///     .temperature(0.5)
///     .send()
///     .await
///     .expect("Failed to get transcription response");
/// ```
///
/// Note: It is usually unnecessary to create a completion request builder directly.
/// Instead, use the [TranscriptionModel::transcription_request] method.
// 转录请求的构建器结构体
// 示例用法：
// ```rust
// use rig::{
//     providers::openai::{Client, self},
//     transcription::TranscriptionRequestBuilder,
// };
//
// let openai = Client::new("your-openai-api-key");
// let model = openai.transcription_model(openai::WHISPER_1).build();
//
// // 创建完成请求并分别执行
// let request = TranscriptionRequestBuilder::new(model, "~/audio.mp3".to_string())
//     .temperature(0.5)
//     .build();
//
// let response = model.transcription(request)
//     .await
//     .expect("Failed to get transcription response");
// ```
//
// 或者，您可以直接从构建器执行转录请求：
// ```rust
// use rig::{
//     providers::openai::{Client, self},
//     transcription::TranscriptionRequestBuilder,
// };
//
// let openai = Client::new("your-openai-api-key");
// let model = openai.transcription_model(openai::WHISPER_1).build();
//
// // 创建完成请求并直接执行
// let response = TranscriptionRequestBuilder::new(model, "~/audio.mp3".to_string())
//     .temperature(0.5)
//     .send()
//     .await
//     .expect("Failed to get transcription response");
// ```
//
// 注意：通常不需要直接创建完成请求构建器。
// 相反，使用 TranscriptionModel::transcription_request 方法。
pub struct TranscriptionRequestBuilder<M>
where
    // M 必须实现 TranscriptionModel trait
    M: TranscriptionModel,
{
    // 转录模型
    model: M,
    // 文件数据
    data: Vec<u8>,
    // 文件名（可选）
    filename: Option<String>,
    // 语言
    language: String,
    // 提示（可选）
    prompt: Option<String>,
    // 温度参数（可选）
    temperature: Option<f64>,
    // 额外参数（可选）
    additional_params: Option<serde_json::Value>,
}

// 转录请求构建器实现
impl<M> TranscriptionRequestBuilder<M>
where
    // M 必须实现 TranscriptionModel trait
    M: TranscriptionModel,
{
    // 创建新的转录请求构建器
    pub fn new(model: M) -> Self {
        TranscriptionRequestBuilder {
            model,
            data: vec![],
            filename: None,
            language: "en".to_string(),
            prompt: None,
            temperature: None,
            additional_params: None,
        }
    }

    // 设置文件名
    pub fn filename(mut self, filename: Option<String>) -> Self {
        self.filename = filename;
        self
    }

    /// Sets the data for the request
    // 为请求设置数据
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Load the specified file into data
    // 将指定文件加载到数据中
    pub fn load_file<P>(self, path: P) -> Self
    where
        // P 必须实现 AsRef<Path> trait
        P: AsRef<Path>,
    {
        // 获取路径引用
        let path = path.as_ref();
        // 读取文件数据
        let data = fs::read(path).expect("Failed to load audio file, file did not exist");

        // 设置文件名和数据
        self.filename(Some(
            path.file_name()
                .expect("Path was not a file")
                .to_str()
                .expect("Failed to convert filename to ascii")
                .to_string(),
        ))
        .data(data)
    }

    /// Sets the output language for the transcription request
    // 为转录请求设置输出语言
    pub fn language(mut self, language: String) -> Self {
        self.language = language;
        self
    }

    /// Sets the prompt to be sent in the transcription request
    // 设置要发送在转录请求中的提示
    pub fn prompt(mut self, prompt: String) -> Self {
        self.prompt = Some(prompt);
        self
    }

    /// Set the temperature to be sent in the transcription request
    // 设置在转录请求中发送的温度参数
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Adds additional parameters to the transcription request.
    // 向转录请求添加额外参数
    pub fn additional_params(mut self, additional_params: serde_json::Value) -> Self {
        // 匹配现有参数
        match self.additional_params {
            // 如果已有参数，则合并
            Some(params) => {
                self.additional_params = Some(json_utils::merge(params, additional_params));
            }
            // 如果没有参数，则设置新参数
            None => {
                self.additional_params = Some(additional_params);
            }
        }
        self
    }

    /// Sets the additional parameters for the transcription request.
    // 为转录请求设置额外参数
    pub fn additional_params_opt(mut self, additional_params: Option<serde_json::Value>) -> Self {
        self.additional_params = additional_params;
        self
    }

    /// Builds the transcription request
    /// Panics if data is empty.
    // 构建转录请求
    // 如果数据为空则 panic
    pub fn build(self) -> TranscriptionRequest {
        // 检查数据是否为空
        if self.data.is_empty() {
            panic!("Data cannot be empty!")
        }

        // 创建转录请求
        TranscriptionRequest {
            data: self.data,
            filename: self.filename.unwrap_or("file".to_string()),
            language: self.language,
            prompt: self.prompt,
            temperature: self.temperature,
            additional_params: self.additional_params,
        }
    }

    /// Sends the transcription request to the transcription model provider and returns the transcription response
    // 将转录请求发送给转录模型提供商并返回转录响应
    pub async fn send(self) -> Result<TranscriptionResponse<M::Response>, TranscriptionError> {
        // 克隆模型
        let model = self.model.clone();

        // 构建请求并发送
        model.transcription(self.build()).await
    }
}
