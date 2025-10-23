// 导入图像生成模块
use crate::image_generation;
// 导入图像生成错误和请求类型
use crate::image_generation::{ImageGenerationError, ImageGenerationRequest};
// 导入 JSON 工具的原地合并函数
use crate::json_utils::merge_inplace;
// 导入 OpenAI 提供商的 API 响应和客户端
use crate::providers::openai::{ApiResponse, Client};
// 导入 base64 引擎 trait
use base64::Engine;
// 导入标准 base64 编解码器
use base64::prelude::BASE64_STANDARD;
// 导入反序列化宏
use serde::Deserialize;
// 导入 JSON 宏
use serde_json::json;

// ================================================================
// OpenAI 图像生成 API
// ================================================================
// DALL-E-2 模型常量
pub const DALL_E_2: &str = "dall-e-2";
// DALL-E-3 模型常量
pub const DALL_E_3: &str = "dall-e-3";

// GPT-IMAGE-1 模型常量
pub const GPT_IMAGE_1: &str = "gpt-image-1";

// 派生 Debug 和 Deserialize trait
#[derive(Debug, Deserialize)]
// 图像生成数据结构体
pub struct ImageGenerationData {
    // base64 编码的 JSON 数据
    pub b64_json: String,
}

// 派生 Debug 和 Deserialize trait
#[derive(Debug, Deserialize)]
// 图像生成响应结构体
pub struct ImageGenerationResponse {
    // 创建时间戳
    pub created: i32,
    // 图像生成数据向量
    pub data: Vec<ImageGenerationData>,
}

// 为 ImageGenerationResponse 实现到 image_generation::ImageGenerationResponse 的转换
impl TryFrom<ImageGenerationResponse>
    for image_generation::ImageGenerationResponse<ImageGenerationResponse>
{
    // 错误类型
    type Error = ImageGenerationError;

    // 尝试转换方法
    fn try_from(value: ImageGenerationResponse) -> Result<Self, Self::Error> {
        // 克隆第一个数据的 base64 JSON
        let b64_json = value.data[0].b64_json.clone();

        // 解码 base64 数据为字节
        let bytes = BASE64_STANDARD
            .decode(&b64_json)
            .expect("Failed to decode b64");

        // 返回图像生成响应
        Ok(image_generation::ImageGenerationResponse {
            // 图像字节数据
            image: bytes,
            // 原始响应
            response: value,
        })
    }
}

// 派生 Clone trait
#[derive(Clone)]
// 图像生成模型结构体
pub struct ImageGenerationModel {
    // 客户端
    client: Client,
    /// Name of the model (e.g.: dall-e-2)
    // 模型名称（例如：dall-e-2）
    pub model: String,
}

// ImageGenerationModel 的实现
impl ImageGenerationModel {
    // 创建新的图像生成模型实例（包可见）
    pub(crate) fn new(client: Client, model: &str) -> Self {
        Self {
            // 设置客户端
            client,
            // 设置模型名称
            model: model.to_string(),
        }
    }
}

// 为 ImageGenerationModel 实现 image_generation::ImageGenerationModel trait
impl image_generation::ImageGenerationModel for ImageGenerationModel {
    // 响应类型
    type Response = ImageGenerationResponse;

    // 图像生成方法（支持 worker 特性）
    #[cfg_attr(feature = "worker", worker::send)]
    async fn image_generation(
        &self,
        generation_request: ImageGenerationRequest,
    ) -> Result<image_generation::ImageGenerationResponse<Self::Response>, ImageGenerationError>
    {
        // 构建 JSON 请求体
        let mut request = json!({
            "model": self.model,
            "prompt": generation_request.prompt,
            "size": format!("{}x{}", generation_request.width, generation_request.height),
        });

        // 如果模型不是 gpt-image-1，添加响应格式
        if self.model != *"gpt-image-1" {
            // 原地合并 JSON，设置响应格式为 base64 JSON
            merge_inplace(
                &mut request,
                json!({
                    "response_format": "b64_json"
                }),
            );
        }

        // 发送 POST 请求到图像生成端点
        let response = self
            .client
            .post("/images/generations")
            .json(&request)
            .send()
            .await?;

        // 检查响应状态
        if !response.status().is_success() {
            // 返回提供商错误
            return Err(ImageGenerationError::ProviderError(format!(
                "{}: {}",
                response.status(),
                response.text().await?
            )));
        }

        // 获取响应文本
        let t = response.text().await?;

        // 解析 API 响应
        match serde_json::from_str::<ApiResponse<ImageGenerationResponse>>(&t)? {
            // 成功响应
            ApiResponse::Ok(response) => response.try_into(),
            // 错误响应
            ApiResponse::Err(err) => Err(ImageGenerationError::ProviderError(err.message)),
        }
    }
}
