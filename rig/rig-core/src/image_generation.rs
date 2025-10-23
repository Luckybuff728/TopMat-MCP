//! 与 Rig 中核心图像生成抽象相关的所有内容。
//! Rig 允许使用 [ImageGenerationModel] trait 调用许多不同的提供商（支持图像生成的）。
// 导入图像生成模型句柄类型
use crate::client::image_generation::ImageGenerationModelHandle;
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
// 定义图像生成错误枚举
pub enum ImageGenerationError {
    /// HTTP 错误（例如：连接错误、超时等）
    // HTTP 错误，从 reqwest::Error 自动转换
    #[error("HttpError: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON 错误（例如：序列化、反序列化）
    // JSON 错误，从 serde_json::Error 自动转换
    #[error("JsonError: {0}")]
    JsonError(#[from] serde_json::Error),

    /// 构建图像生成请求时的错误
    // 请求构建错误，从通用错误类型自动转换
    #[error("RequestError: {0}")]
    RequestError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// 解析图像生成响应时的错误
    // 响应解析错误，包含错误消息字符串
    #[error("ResponseError: {0}")]
    ResponseError(String),

    /// 图像生成模型提供商返回的错误
    // 提供商错误，包含错误消息字符串
    #[error("ProviderError: {0}")]
    ProviderError(String),
}
// 图像生成 trait
pub trait ImageGeneration<M>
where
    // M 必须实现 ImageGenerationModel trait
    M: ImageGenerationModel,
{
    /// Generates a transcription request builder for the given `file`.
    /// This function is meant to be called by the user to further customize the
    /// request at transcription time before sending it.
    ///
    /// ❗IMPORTANT: The type that implements this trait might have already
    /// populated fields in the builder (the exact fields depend on the type).
    /// For fields that have already been set by the model, calling the corresponding
    /// method on the builder will overwrite the value set by the model.
    // 为给定的提示和尺寸生成图像生成请求构建器
    // 此函数旨在由用户在发送前进一步自定义请求时调用
    // 注意：实现此 trait 的类型可能已经在构建器中填充了字段
    // 对于已由模型设置的字段，调用构建器上的相应方法将覆盖模型设置的值
    fn image_generation(
        &self,
        // 图像生成的提示文本
        prompt: &str,
        // 生成图像的尺寸 (宽度, 高度)
        size: &(u32, u32),
    ) -> impl std::future::Future<
        Output = Result<ImageGenerationRequestBuilder<M>, ImageGenerationError>,
    > + Send;
}

/// A unified response for a model image generation, returning both the image and the raw response.
// 模型图像生成的统一响应，返回图像和原始响应
// 派生 Debug trait
#[derive(Debug)]
pub struct ImageGenerationResponse<T> {
    // 生成的图像数据（字节数组）
    pub image: Vec<u8>,
    // 原始响应数据
    pub response: T,
}

// 图像生成模型 trait
pub trait ImageGenerationModel: Clone + Send + Sync {
    // 响应类型，必须实现 Send 和 Sync trait
    type Response: Send + Sync;

    // 执行图像生成
    fn image_generation(
        &self,
        // 图像生成请求
        request: ImageGenerationRequest,
    ) -> impl std::future::Future<
        Output = Result<ImageGenerationResponse<Self::Response>, ImageGenerationError>,
    > + Send;

    // 获取图像生成请求构建器
    fn image_generation_request(&self) -> ImageGenerationRequestBuilder<Self> {
        // 创建新的构建器实例
        ImageGenerationRequestBuilder::new(self.clone())
    }
}

// 动态图像生成模型 trait
pub trait ImageGenerationModelDyn: Send + Sync {
    // 执行图像生成，返回装箱的 Future
    fn image_generation(
        &self,
        // 图像生成请求
        request: ImageGenerationRequest,
    ) -> BoxFuture<'_, Result<ImageGenerationResponse<()>, ImageGenerationError>>;

    // 获取图像生成请求构建器
    fn image_generation_request(
        &self,
    ) -> ImageGenerationRequestBuilder<ImageGenerationModelHandle<'_>>;
}

// 为实现了 ImageGenerationModel 的类型实现 ImageGenerationModelDyn
impl<T> ImageGenerationModelDyn for T
where
    // T 必须实现 ImageGenerationModel trait
    T: ImageGenerationModel,
{
    // 执行图像生成，返回装箱的 Future
    fn image_generation(
        &self,
        request: ImageGenerationRequest,
    ) -> BoxFuture<'_, Result<ImageGenerationResponse<()>, ImageGenerationError>> {
        // 装箱异步块
        Box::pin(async {
            // 调用图像生成方法
            let resp = self.image_generation(request).await;
            // 映射响应，将原始响应替换为空元组
            resp.map(|r| ImageGenerationResponse {
                image: r.image,
                response: (),
            })
        })
    }

    // 获取图像生成请求构建器
    fn image_generation_request(
        &self,
    ) -> ImageGenerationRequestBuilder<ImageGenerationModelHandle<'_>> {
        // 创建图像生成请求构建器，使用模型句柄包装
        ImageGenerationRequestBuilder::new(ImageGenerationModelHandle {
            inner: Arc::new(self.clone()),
        })
    }
}

/// An image generation request.
// 图像生成请求
// 标记为非穷尽结构体，允许未来添加字段
#[non_exhaustive]
pub struct ImageGenerationRequest {
    // 图像生成的提示文本
    pub prompt: String,
    // 生成图像的宽度
    pub width: u32,
    // 生成图像的高度
    pub height: u32,
    // 附加参数（可选）
    pub additional_params: Option<Value>,
}

/// A builder for `ImageGenerationRequest`.
/// Can be sent to a model provider.
// ImageGenerationRequest 的构建器
// 可以发送到模型提供商
// 标记为非穷尽结构体，允许未来添加字段
#[non_exhaustive]
pub struct ImageGenerationRequestBuilder<M>
where
    // M 必须实现 ImageGenerationModel trait
    M: ImageGenerationModel,
{
    // 图像生成模型
    model: M,
    // 图像生成的提示文本
    prompt: String,
    // 生成图像的宽度
    width: u32,
    // 生成图像的高度
    height: u32,
    // 附加参数（可选）
    additional_params: Option<Value>,
}

// 为 ImageGenerationRequestBuilder 实现方法
impl<M> ImageGenerationRequestBuilder<M>
where
    // M 必须实现 ImageGenerationModel trait
    M: ImageGenerationModel,
{
    // 创建新的图像生成请求构建器
    pub fn new(model: M) -> Self {
        Self {
            // 设置模型
            model,
            // 初始化空提示文本
            prompt: "".to_string(),
            // 设置默认高度为 256
            height: 256,
            // 设置默认宽度为 256
            width: 256,
            // 初始化附加参数为 None
            additional_params: None,
        }
    }

    /// Sets the prompt for the image generation request
    // 设置图像生成请求的提示文本
    pub fn prompt(mut self, prompt: &str) -> Self {
        // 设置提示文本
        self.prompt = prompt.to_string();
        // 返回 self 以支持链式调用
        self
    }

    /// The width of the generated image
    // 设置生成图像的宽度
    pub fn width(mut self, width: u32) -> Self {
        // 设置宽度
        self.width = width;
        // 返回 self 以支持链式调用
        self
    }

    /// The height of the generated image
    // 设置生成图像的高度
    pub fn height(mut self, height: u32) -> Self {
        // 设置高度
        self.height = height;
        // 返回 self 以支持链式调用
        self
    }

    /// Adds additional parameters to the image generation request.
    // 为图像生成请求添加附加参数
    pub fn additional_params(mut self, params: Value) -> Self {
        // 设置附加参数
        self.additional_params = Some(params);
        // 返回 self 以支持链式调用
        self
    }

    // 构建图像生成请求
    pub fn build(self) -> ImageGenerationRequest {
        ImageGenerationRequest {
            // 设置提示文本
            prompt: self.prompt,
            // 设置宽度
            width: self.width,
            // 设置高度
            height: self.height,
            // 设置附加参数
            additional_params: self.additional_params,
        }
    }

    // 发送图像生成请求
    pub async fn send(self) -> Result<ImageGenerationResponse<M::Response>, ImageGenerationError> {
        // 克隆模型
        let model = self.model.clone();

        // 调用模型的图像生成方法
        model.image_generation(self.build()).await
    }
}
