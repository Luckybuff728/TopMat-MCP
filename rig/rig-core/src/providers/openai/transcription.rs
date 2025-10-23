// 导入 OpenAI 提供商的 API 响应和客户端
use crate::providers::openai::{ApiResponse, Client};
// 导入转录模块
use crate::transcription;
// 导入转录错误类型
use crate::transcription::TranscriptionError;
// 导入 reqwest 的 multipart Part 类型
use reqwest::multipart::Part;
// 导入反序列化宏
use serde::Deserialize;

// ================================================================
// OpenAI 转录 API
// ================================================================
// Whisper-1 模型常量
pub const WHISPER_1: &str = "whisper-1";

// 派生 Debug 和 Deserialize trait
#[derive(Debug, Deserialize)]
// 转录响应结构体
pub struct TranscriptionResponse {
    // 转录文本
    pub text: String,
}

// 为 TranscriptionResponse 实现到 transcription::TranscriptionResponse 的转换
impl TryFrom<TranscriptionResponse>
    for transcription::TranscriptionResponse<TranscriptionResponse>
{
    // 错误类型
    type Error = TranscriptionError;

    // 尝试转换方法
    fn try_from(value: TranscriptionResponse) -> Result<Self, Self::Error> {
        // 返回转录响应
        Ok(transcription::TranscriptionResponse {
            // 克隆转录文本
            text: value.text.clone(),
            // 原始响应
            response: value,
        })
    }
}

// 派生 Clone trait
#[derive(Clone)]
// 转录模型结构体
pub struct TranscriptionModel {
    // 客户端
    client: Client,
    /// Name of the model (e.g.: gpt-3.5-turbo-1106)
    // 模型名称（例如：whisper-1）
    pub model: String,
}

// TranscriptionModel 的实现
impl TranscriptionModel {
    // 创建新的转录模型实例
    pub fn new(client: Client, model: &str) -> Self {
        Self {
            // 设置客户端
            client,
            // 设置模型名称
            model: model.to_string(),
        }
    }
}

// 为 TranscriptionModel 实现 transcription::TranscriptionModel trait
impl transcription::TranscriptionModel for TranscriptionModel {
    // 响应类型
    type Response = TranscriptionResponse;

    // 转录方法（支持 worker 特性）
    #[cfg_attr(feature = "worker", worker::send)]
    async fn transcription(
        &self,
        request: transcription::TranscriptionRequest,
    ) -> Result<
        transcription::TranscriptionResponse<Self::Response>,
        transcription::TranscriptionError,
    > {
        // 获取音频数据
        let data = request.data;

        // 构建 multipart 表单
        let mut body = reqwest::multipart::Form::new()
            // 添加模型字段
            .text("model", self.model.clone())
            // 添加语言字段
            .text("language", request.language)
            // 添加文件字段
            .part(
                "file",
                Part::bytes(data).file_name(request.filename.clone()),
            );

        // 如果有提示词，添加到表单
        if let Some(prompt) = request.prompt {
            body = body.text("prompt", prompt.clone());
        }

        // 如果有温度参数，添加到表单
        if let Some(ref temperature) = request.temperature {
            body = body.text("temperature", temperature.to_string());
        }

        // 如果有额外参数，添加到表单
        if let Some(ref additional_params) = request.additional_params {
            // 遍历额外参数的键值对
            for (key, value) in additional_params
                .as_object()
                .expect("Additional Parameters to OpenAI Transcription should be a map")
            {
                // 添加每个参数到表单
                body = body.text(key.to_owned(), value.to_string());
            }
        }

        // 发送 POST 请求到转录端点
        let response = self
            .client
            .post("audio/transcriptions")
            .multipart(body)
            .send()
            .await?;

        // 检查响应状态
        if response.status().is_success() {
            // 解析响应为 JSON
            match response
                .json::<ApiResponse<TranscriptionResponse>>()
                .await?
            {
                // 成功响应
                ApiResponse::Ok(response) => response.try_into(),
                // 错误响应
                ApiResponse::Err(api_error_response) => Err(TranscriptionError::ProviderError(
                    api_error_response.message,
                )),
            }
        } else {
            // 返回提供商错误
            Err(TranscriptionError::ProviderError(response.text().await?))
        }
    }
}
