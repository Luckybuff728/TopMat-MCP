// 导入音频生成模块的相关类型
use crate::audio_generation::{
    self, AudioGenerationError, AudioGenerationRequest, AudioGenerationResponse,
};
// 导入 OpenAI 客户端
use crate::providers::openai::Client;
// 导入字节类型
use bytes::Bytes;
// 导入 JSON 宏
use serde_json::json;

// TTS-1 模型常量
pub const TTS_1: &str = "tts-1";
// TTS-1-HD 模型常量
pub const TTS_1_HD: &str = "tts-1-hd";

// 派生 Clone trait
#[derive(Clone)]
// 音频生成模型结构体
pub struct AudioGenerationModel {
    // OpenAI 客户端
    client: Client,
    // 模型名称
    pub model: String,
}

// AudioGenerationModel 的实现
impl AudioGenerationModel {
    // 创建新的音频生成模型实例
    pub fn new(client: Client, model: &str) -> Self {
        Self {
            // 设置客户端
            client,
            // 设置模型名称
            model: model.to_string(),
        }
    }
}

// 为 AudioGenerationModel 实现 audio_generation::AudioGenerationModel trait
impl audio_generation::AudioGenerationModel for AudioGenerationModel {
    // 响应类型为字节
    type Response = Bytes;

    // 音频生成方法（支持 worker 特性）
    #[cfg_attr(feature = "worker", worker::send)]
    async fn audio_generation(
        &self,
        request: AudioGenerationRequest,
    ) -> Result<AudioGenerationResponse<Self::Response>, AudioGenerationError> {
        // 构建 JSON 请求体
        let request = json!({
            "model": self.model,
            "input": request.text,
            "voice": request.voice,
            "speed": request.speed,
        });

        // 发送 POST 请求到音频生成端点
        let response = self
            .client
            .post("/audio/speech")
            .json(&request)
            .send()
            .await?;

        // 检查响应状态
        if !response.status().is_success() {
            // 返回提供商错误
            return Err(AudioGenerationError::ProviderError(format!(
                "{}: {}",
                response.status(),
                response.text().await?
            )));
        }

        // 获取响应字节
        let bytes = response.bytes().await?;

        // 返回音频生成响应
        Ok(AudioGenerationResponse {
            // 音频字节向量
            audio: bytes.to_vec(),
            // 原始响应字节
            response: bytes,
        })
    }
}
