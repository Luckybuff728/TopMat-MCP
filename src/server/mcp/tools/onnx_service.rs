//! ONNX Service MCP 工具
//!
//! 提供 ONNX Service API 交互的工具，用于模型管理、推理计算等功能

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error as StdError;
use rig::{
    completion::ToolDefinition,
    tool::Tool,
};

use reqwest;

// ONNX Service API 基础 URL
const API_BASE_URL: &str = "http://111.22.21.99:10002";


// ==================== 错误类型 ====================

#[derive(Debug)]
pub enum OnnxServiceError {
    HttpError(String),
    ApiError { status: u16, message: String },
    JsonError(String),
    InvalidRequest(String),
    MissingParameter(String),
}

impl std::fmt::Display for OnnxServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnnxServiceError::HttpError(msg) => write!(f, "HTTP请求失败: {}", msg),
            OnnxServiceError::ApiError { status, message } => {
                write!(f, "API错误 (状态码 {}): {}", status, message)
            }
            OnnxServiceError::JsonError(msg) => {
                write!(f, "JSON序列化/反序列化错误: {}", msg)
            }
            OnnxServiceError::InvalidRequest(msg) => write!(f, "无效请求: {}", msg),
            OnnxServiceError::MissingParameter(param) => {
                write!(f, "缺少必需参数: {}", param)
            }
        }
    }
}

impl StdError for OnnxServiceError {}

// ==================== 请求/响应结构体 ====================

/// 空参数结构体，用于不需要参数的工具
#[derive(Debug, Deserialize)]
pub struct EmptyParams {}

/// 健康检查响应
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub model_status: String,
}

/// 模型加载请求
#[derive(Debug, Deserialize)]
pub struct LoadModelRequest {
    pub folder_name: String,
}

/// 模型卸载请求
#[derive(Debug, Deserialize)]
pub struct UnloadModelRequest {
    pub uuid: Option<String>,
    pub model_name: Option<String>,
}

/// 推理请求
#[derive(Debug, Deserialize)]
pub struct InferenceRequest {
    pub uuid: String,
    pub inputs: HashMap<String, f64>,
}

/// UUID参数
#[derive(Debug, Deserialize)]
pub struct UuidParams {
    pub uuid: String,
}

/// 模型信息响应
#[derive(Debug, Deserialize)]
pub struct AllModelsInfoResponse {
    pub dynamic_models: Vec<DynamicModelInfo>,
    pub total_count: usize,
}

/// 动态模型信息
#[derive(Debug, Deserialize)]
pub struct DynamicModelInfo {
    pub uuid: String,
    pub config: ModelConfig,
    pub onnx_path: String,
    pub status: String,
    pub is_loaded: bool,
}

/// 模型配置
#[derive(Debug, Deserialize)]
pub struct ModelConfig {
    pub model: ModelMetadata,
    pub inputs: Vec<InputSpec>,
    pub outputs: Vec<OutputSpec>,
}

/// 模型元数据
#[derive(Debug, Deserialize)]
pub struct ModelMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// 输入规格
#[derive(Debug, Deserialize)]
pub struct InputSpec {
    pub feature: String,
    pub min: f64,
    pub max: f64,
    pub description: String,
}

/// 输出规格
#[derive(Debug, Deserialize)]
pub struct OutputSpec {
    pub target: String,
    pub min: f64,
    pub max: f64,
    pub description: String,
}

/// 推理响应
#[derive(Debug, Deserialize)]
pub struct InferenceResponse {
    pub outputs: HashMap<String, OutputValue>,
    pub inference_time_ms: f64,
    pub request_id: String,
    pub model_uuid: String,
}

/// 输出值结构
#[derive(Debug, Deserialize)]
pub struct OutputValue {
    pub value: f64,
    pub description: String,
    pub min: f64,
    pub max: f64,
}

/// 模型配置响应
#[derive(Debug, Deserialize)]
pub struct ModelConfigResponse {
    pub uuid: String,
    pub config: ModelConfig,
    pub status: String,
    pub is_loaded: bool,
}

// ==================== 工具实现 ====================

/// ONNX Service 健康检查工具
#[derive(Deserialize, Serialize)]
pub struct OnnxHealthCheck;

impl Default for OnnxHealthCheck {
    fn default() -> Self {
        Self
    }
}

impl Tool for OnnxHealthCheck {
    const NAME: &'static str = "onnx_health_check";
    type Error = OnnxServiceError;
    type Args = EmptyParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "onnx_health_check".to_string(),
            description: "检查 ONNX Service 服务状态和模型状态".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        let health_url = format!("{}/health", API_BASE_URL);

        let response = client
            .get(&health_url)
            .send()
            .await
            .map_err(|e| OnnxServiceError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OnnxServiceError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let health_response: HealthResponse = response
            .json()
            .await
            .map_err(|e| OnnxServiceError::JsonError(e.to_string()))?;

        Ok(format!(
            "🟢 ONNX Service 健康状态:\n📊 服务状态: {}\n🤖 模型状态: {}\n⏰ 检查时间: {}",
            health_response.status, health_response.model_status, health_response.timestamp
        ))
    }
}

/// ONNX Service 获取模型信息工具
#[derive(Deserialize, Serialize)]
pub struct OnnxGetModelsInfo;

impl Default for OnnxGetModelsInfo {
    fn default() -> Self {
        Self
    }
}

impl Tool for OnnxGetModelsInfo {
    const NAME: &'static str = "onnx_get_models_info";
    type Error = OnnxServiceError;
    type Args = EmptyParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "onnx_get_models_info".to_string(),
            description: "获取所有已加载的 ONNX 模型信息".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        let models_url = format!("{}/model/info", API_BASE_URL);

        let response = client
            .get(&models_url)
            .send()
            .await
            .map_err(|e| OnnxServiceError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OnnxServiceError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let models_response: AllModelsInfoResponse = response
            .json()
            .await
            .map_err(|e| OnnxServiceError::JsonError(e.to_string()))?;

        let mut info_text = format!(
            "📋 模型信息总览 (共 {} 个模型)\n\n",
            models_response.total_count
        );

        // 动态模型信息
        if !models_response.dynamic_models.is_empty() {
            info_text.push_str("🚀 动态模型列表:\n");
            for model in models_response.dynamic_models {
                info_text.push_str(&format!(
                    "• UUID: {}\n  名称: {}\n  版本: {}\n  描述: {}\n  状态: {}\n  路径: {}\n\n",
                    model.uuid,
                    model.config.model.name,
                    model.config.model.version,
                    model.config.model.description,
                    model.status,
                    model.onnx_path
                ));
            }
        } else {
            info_text.push_str("暂无动态模型\n");
        }

        Ok(info_text)
    }
}

/// ONNX Service 加载模型工具
#[derive(Deserialize, Serialize)]
pub struct OnnxLoadModel;

impl Default for OnnxLoadModel {
    fn default() -> Self {
        Self
    }
}

impl Tool for OnnxLoadModel {
    const NAME: &'static str = "onnx_load_model";
    type Error = OnnxServiceError;
    type Args = LoadModelRequest;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "onnx_load_model".to_string(),
            description: "加载指定文件夹中的 ONNX 模型".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "folder_name": {
                        "type": "string",
                        "description": "模型文件夹名称"
                    }
                },
                "required": ["folder_name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        let load_url = format!("{}/model/load", API_BASE_URL);

        let load_payload = json!({
            "folder_name": args.folder_name
        });

        let response = client
            .post(&load_url)
            .json(&load_payload)
            .send()
            .await
            .map_err(|e| OnnxServiceError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OnnxServiceError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let load_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OnnxServiceError::JsonError(e.to_string()))?;

        Ok(format!(
            "✅ 模型加载成功！\n📁 文件夹: {}\n📝 响应: {}",
            args.folder_name,
            serde_json::to_string_pretty(&load_response).unwrap_or_default()
        ))
    }
}

/// ONNX Service 卸载模型工具
#[derive(Deserialize, Serialize)]
pub struct OnnxUnloadModel;

impl Default for OnnxUnloadModel {
    fn default() -> Self {
        Self
    }
}

impl Tool for OnnxUnloadModel {
    const NAME: &'static str = "onnx_unload_model";
    type Error = OnnxServiceError;
    type Args = UnloadModelRequest;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "onnx_unload_model".to_string(),
            description: "卸载指定的 ONNX 模型".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "uuid": {
                        "type": "string",
                        "description": "模型UUID（可选）"
                    },
                    "model_name": {
                        "type": "string",
                        "description": "模型名称（可选）"
                    }
                },
                "anyOf": [
                    {"required": ["uuid"]},
                    {"required": ["model_name"]}
                ]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.uuid.is_none() && args.model_name.is_none() {
            return Err(OnnxServiceError::MissingParameter(
                "必须提供 uuid 或 model_name 中的一个".to_string(),
            ));
        }

        let client = reqwest::Client::new();
        let unload_url = format!("{}/model/unload", API_BASE_URL);

        let unload_payload = json!({
            "uuid": args.uuid,
            "model_name": args.model_name
        });

        let response = client
            .post(&unload_url)
            .json(&unload_payload)
            .send()
            .await
            .map_err(|e| OnnxServiceError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OnnxServiceError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let unload_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OnnxServiceError::JsonError(e.to_string()))?;

        let identifier = args.uuid.unwrap_or_else(|| args.model_name.unwrap_or_default());
        Ok(format!(
            "✅ 模型卸载成功！\n🔍 标识符: {}\n📝 响应: {}",
            identifier,
            serde_json::to_string_pretty(&unload_response).unwrap_or_default()
        ))
    }
}

/// ONNX Service 模型推理工具
#[derive(Deserialize, Serialize)]
pub struct OnnxModelInference;

impl Default for OnnxModelInference {
    fn default() -> Self {
        Self
    }
}

impl Tool for OnnxModelInference {
    const NAME: &'static str = "onnx_model_inference";
    type Error = OnnxServiceError;
    type Args = InferenceRequest;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "onnx_model_inference".to_string(),
            description: "对指定 ONNX 模型执行推理计算".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "uuid": {
                        "type": "string",
                        "description": "模型UUID"
                    },
                    "inputs": {
                        "type": "object",
                        "additionalProperties": {"type": "number"},
                        "description": "输入参数字典，格式为 {\"feature_name\": value}"
                    }
                },
                "required": ["uuid", "inputs"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        let inference_url = format!("{}/models/{}/inference", API_BASE_URL, args.uuid);

        let inference_payload = json!({
            "inputs": args.inputs
        });

        let response = client
            .post(&inference_url)
            .json(&inference_payload)
            .send()
            .await
            .map_err(|e| OnnxServiceError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OnnxServiceError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let inference_response: InferenceResponse = response
            .json()
            .await
            .map_err(|e| OnnxServiceError::JsonError(e.to_string()))?;

        let mut result_text = format!(
            "🎯 推理完成！\n🆔 模型UUID: {}\n⏱️ 推理时间: {:.2}ms\n📋 请求ID: {}\n\n🔢 输出结果:\n",
            inference_response.model_uuid,
            inference_response.inference_time_ms,
            inference_response.request_id
        );

        for (key, output_value) in inference_response.outputs {
            result_text.push_str(&format!(
                "• {}: {:.6}\n  📝 描述: {}\n  📊 范围: [{:.2}, {:.2}]\n",
                key,
                output_value.value,
                output_value.description,
                output_value.min,
                output_value.max
            ));
        }

        Ok(result_text)
    }
}

/// ONNX Service 获取模型配置工具
#[derive(Deserialize, Serialize)]
pub struct OnnxGetModelConfig;

impl Default for OnnxGetModelConfig {
    fn default() -> Self {
        Self
    }
}

impl Tool for OnnxGetModelConfig {
    const NAME: &'static str = "onnx_get_model_config";
    type Error = OnnxServiceError;
    type Args = UuidParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "onnx_get_model_config".to_string(),
            description: "获取指定 ONNX 模型的详细配置信息".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "uuid": {
                        "type": "string",
                        "description": "模型UUID"
                    }
                },
                "required": ["uuid"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        let config_url = format!("{}/models/{}/info", API_BASE_URL, args.uuid);

        let response = client
            .get(&config_url)
            .send()
            .await
            .map_err(|e| OnnxServiceError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OnnxServiceError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let config_response: ModelConfigResponse = response
            .json()
            .await
            .map_err(|e| OnnxServiceError::JsonError(e.to_string()))?;

        let mut config_text = format!(
            "⚙️ 模型配置详情\n\n🆔 UUID: {}\n📊 状态: {}\n✅ 已加载: {}\n\n📋 基本信息:\n• 名称: {}\n• 版本: {}\n• 描述: {}\n\n",
            config_response.uuid,
            config_response.status,
            config_response.is_loaded,
            config_response.config.model.name,
            config_response.config.model.version,
            config_response.config.model.description
        );

        // 输入规格
        config_text.push_str("📥 输入规格:\n");
        for input in config_response.config.inputs {
            config_text.push_str(&format!(
                "• {}: [{:.2}, {:.2}] - {}\n",
                input.feature, input.min, input.max, input.description
            ));
        }

        // 输出规格
        config_text.push_str("\n📤 输出规格:\n");
        for output in config_response.config.outputs {
            config_text.push_str(&format!(
                "• {}: [{:.2}, {:.2}] - {}\n",
                output.target, output.min, output.max, output.description
            ));
        }

        Ok(config_text)
    }
}

/// ONNX Service 问候工具
#[derive(Deserialize, Serialize)]
pub struct OnnxSayHello;

impl Default for OnnxSayHello {
    fn default() -> Self {
        Self
    }
}

impl Tool for OnnxSayHello {
    const NAME: &'static str = "onnx_say_hello";
    type Error = OnnxServiceError;
    type Args = EmptyParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "onnx_say_hello".to_string(),
            description: "ONNX Service 服务器问候和使用说明".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok("🚀 欢迎使用 ONNX Service MCP 工具！\n\n可用功能:\n\n🔍 服务管理:\n• onnx_health_check - 检查服务状态\n• onnx_get_models_info - 获取所有模型信息\n\n🤖 模型管理:\n• onnx_load_model - 加载模型\n• onnx_unload_model - 卸载模型\n• onnx_get_model_config - 获取模型配置\n\n🎯 模型推理:\n• onnx_model_inference - 执行模型推理\n\n💡 使用提示:\n1. 首先使用 onnx_health_check 检查服务状态\n2. 使用 onnx_get_models_info 查看已加载的模型\n3. 使用 onnx_load_model 加载新模型\n4. 使用 onnx_model_inference 进行推理计算".to_string())
    }
}