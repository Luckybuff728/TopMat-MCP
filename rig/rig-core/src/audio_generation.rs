//! 与音频生成相关的所有内容（即文本转语音）。
//! Rig 使用 [AudioGenerationModel] trait 抽象了许多不同的提供商。
// 导入音频生成模型句柄类型
use crate::client::audio_generation::AudioGenerationModelHandle;
// 导入 Future 装箱类型用于异步操作
use futures::future::BoxFuture;
// 导入 JSON 值类型
use serde_json::Value;
// 导入 Arc 用于原子引用计数
use std::sync::Arc;
// 导入错误处理宏
use thiserror::Error;

// 派生 Debug 和 Error trait 用于错误处理
#[derive(Debug, Error)]
// 定义音频生成错误枚举
pub enum AudioGenerationError {
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
// 定义音频生成 trait，支持泛型模型类型
pub trait AudioGeneration<M>
where
    // M 必须实现 AudioGenerationModel
    M: AudioGenerationModel,
{
    /// 为给定的 `text` 和 `voice` 生成音频生成请求构建器。
    /// 此函数旨在由用户调用，以便在发送前进一步自定义请求。
    ///
    /// ❗重要：实现此 trait 的类型可能已经
    /// 在构建器中填充了字段（具体字段取决于类型）。
    /// 对于已由模型设置的字段，在构建器上调用相应的方法将覆盖模型设置的值。
    // 定义音频生成方法，返回异步 Future
    fn audio_generation(
        &self,
        text: &str,
        voice: &str,
    ) -> impl std::future::Future<
        Output = Result<AudioGenerationRequestBuilder<M>, AudioGenerationError>,
    > + Send;
}

// 定义音频生成响应结构体，支持泛型响应类型
pub struct AudioGenerationResponse<T> {
    // 生成的音频数据（字节数组）
    pub audio: Vec<u8>,
    // 模型特定的响应数据
    pub response: T,
}

// 定义音频生成模型 trait，要求实现 Clone、Send、Sync
pub trait AudioGenerationModel: Clone + Send + Sync {
    // 定义响应类型，必须是 Send + Sync
    type Response: Send + Sync;

    // 定义音频生成方法，接受请求并返回响应
    fn audio_generation(
        &self,
        request: AudioGenerationRequest,
    ) -> impl std::future::Future<
        Output = Result<AudioGenerationResponse<Self::Response>, AudioGenerationError>,
    > + Send;

    // 定义音频生成请求构建器创建方法
    fn audio_generation_request(&self) -> AudioGenerationRequestBuilder<Self> {
        // 使用自身克隆创建请求构建器
        AudioGenerationRequestBuilder::new(self.clone())
    }
}

// 定义动态音频生成模型 trait，用于运行时多态
pub trait AudioGenerationModelDyn: Send + Sync {
    // 定义动态音频生成方法，返回装箱的 Future
    fn audio_generation(
        &self,
        request: AudioGenerationRequest,
    ) -> BoxFuture<'_, Result<AudioGenerationResponse<()>, AudioGenerationError>>;

    // 定义动态音频生成请求构建器创建方法
    fn audio_generation_request(
        &self,
    ) -> AudioGenerationRequestBuilder<AudioGenerationModelHandle<'_>>;
}

// 为实现了 AudioGenerationModel 的类型自动实现 AudioGenerationModelDyn
impl<T> AudioGenerationModelDyn for T
where
    // T 必须实现 AudioGenerationModel
    T: AudioGenerationModel,
{
    // 实现动态音频生成方法
    fn audio_generation(
        &self,
        request: AudioGenerationRequest,
    ) -> BoxFuture<'_, Result<AudioGenerationResponse<()>, AudioGenerationError>> {
        // 装箱异步操作
        Box::pin(async move {
            // 调用具体的音频生成方法
            let resp = self.audio_generation(request).await;

            // 将响应映射为空响应类型
            resp.map(|r| AudioGenerationResponse {
                audio: r.audio,
                response: (),
            })
        })
    }

    // 实现动态音频生成请求构建器创建方法
    fn audio_generation_request(
        &self,
    ) -> AudioGenerationRequestBuilder<AudioGenerationModelHandle<'_>> {
        // 使用 AudioGenerationModelHandle 包装动态模型创建请求构建器
        AudioGenerationRequestBuilder::new(AudioGenerationModelHandle {
            // 将模型用 Arc 包装后存储在句柄中
            inner: Arc::new(self.clone()),
        })
    }
}

// 标记为非穷尽结构体，允许未来添加字段
#[non_exhaustive]
// 定义音频生成请求结构体
pub struct AudioGenerationRequest {
    // 要转换为音频的文本内容
    pub text: String,
    // 音频的声音/语音类型
    pub voice: String,
    // 音频播放速度
    pub speed: f32,
    // 额外的参数（可选）
    pub additional_params: Option<Value>,
}

// 标记为非穷尽结构体，允许未来添加字段
#[non_exhaustive]
// 定义音频生成请求构建器，支持泛型模型类型
pub struct AudioGenerationRequestBuilder<M>
where
    // M 必须实现 AudioGenerationModel
    M: AudioGenerationModel,
{
    // 音频生成模型
    model: M,
    // 要转换为音频的文本内容
    text: String,
    // 音频的声音/语音类型
    voice: String,
    // 音频播放速度
    speed: f32,
    // 额外的参数（可选）
    additional_params: Option<Value>,
}

// 为 AudioGenerationRequestBuilder 实现方法
impl<M> AudioGenerationRequestBuilder<M>
where
    // M 必须实现 AudioGenerationModel
    M: AudioGenerationModel,
{
    // 创建新的音频生成请求构建器
    pub fn new(model: M) -> Self {
        Self {
            // 设置模型
            model,
            // 初始化空文本
            text: "".to_string(),
            // 初始化空声音
            voice: "".to_string(),
            // 设置默认速度为 1.0
            speed: 1.0,
            // 初始化无额外参数
            additional_params: None,
        }
    }

    /// 为音频生成请求设置文本
    // 设置文本内容
    pub fn text(mut self, text: &str) -> Self {
        // 将文本转换为字符串并设置
        self.text = text.to_string();
        // 返回修改后的构建器
        self
    }

    /// 生成音频的声音类型
    // 设置声音类型
    pub fn voice(mut self, voice: &str) -> Self {
        // 将声音转换为字符串并设置
        self.voice = voice.to_string();
        // 返回修改后的构建器
        self
    }

    /// 生成音频的速度
    // 设置播放速度
    pub fn speed(mut self, speed: f32) -> Self {
        // 设置速度值
        self.speed = speed;
        // 返回修改后的构建器
        self
    }

    /// 为音频生成请求添加额外参数。
    // 添加额外参数
    pub fn additional_params(mut self, params: Value) -> Self {
        // 设置额外参数
        self.additional_params = Some(params);
        // 返回修改后的构建器
        self
    }

    // 构建音频生成请求
    pub fn build(self) -> AudioGenerationRequest {
        AudioGenerationRequest {
            // 设置文本内容
            text: self.text,
            // 设置声音类型
            voice: self.voice,
            // 设置播放速度
            speed: self.speed,
            // 设置额外参数
            additional_params: self.additional_params,
        }
    }

    // 发送音频生成请求
    pub async fn send(self) -> Result<AudioGenerationResponse<M::Response>, AudioGenerationError> {
        // 克隆模型
        let model = self.model.clone();

        // 使用模型执行音频生成
        model.audio_generation(self.build()).await
    }
}
