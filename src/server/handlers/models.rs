use axum::{extract::State, response::Json};
use serde_json::Value;

use super::chat::ServerState;
use crate::server::model_router::get_model_router;
use crate::server::models::{ErrorResponse, ModelsResponse};

/// 获取可用模型列表
#[utoipa::path(
    get,
    path = "/v1/models",
    tag = "models",
    summary = "获取模型列表",
    description = "获取当前可用的AI模型列表，包括模型详细信息、性能参数和费用信息",
    responses(
        (status = 200, description = "获取成功", body = ModelsResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
)]
pub async fn list_models_handler(State(_state): State<ServerState>) -> Json<Value> {
    let mut models = get_model_router().get_available_models();

    // 按特定顺序排序
    let order = [
        "qwen-plus",
        "qwen-turbo",
        "qwen-max",
        "qwen-flash",
        "qwq-plus",
        "calphamesh",
        "phase-field",
        "ml-server",
        "ollama-qwen3-4b",
        "ollama-llama3",
        "coating",
    ];

    models.sort_by_key(|name| order.iter().position(|&x| x == name).unwrap_or(usize::MAX));

    let response = serde_json::json!({
        "models": models.iter().map(|model| {
            match model.as_str() {
                "qwen-plus" => serde_json::json!({
                    "name": "qwen-plus",
                    "provider": "qwen",
                    "description": "通义千问Plus，适合一般对话，质量较高",
                    "supports_streaming": true,
                    "max_tokens": 4000,
                    "cost_per_1k_tokens": 0.0020
                }),
                "qwen-turbo" => serde_json::json!({
                    "name": "qwen-turbo",
                    "provider": "qwen",
                    "description": "通义千问Turbo，响应速度快，适合实时对话",
                    "supports_streaming": true,
                    "max_tokens": 4000,
                    "cost_per_1k_tokens": 0.0015
                }),
                "qwen-max" => serde_json::json!({
                    "name": "qwen-max",
                    "provider": "qwen",
                    "description": "通义千问Max，最高质量，适合复杂任务",
                    "supports_streaming": true,
                    "max_tokens": 8000,
                    "cost_per_1k_tokens": 0.0080
                }),
                "qwen-flash" => serde_json::json!({
                    "name": "qwen-flash",
                    "provider": "qwen",
                    "description": "通义千问Flash，极速响应，适合简单问答",
                    "supports_streaming": true,
                    "max_tokens": 2000,
                    "cost_per_1k_tokens": 0.0005
                }),
                "qwq-plus" => serde_json::json!({
                    "name": "qwq-plus",
                    "provider": "qwen",
                    "description": "通义千问增强版，逻辑推理能力强",
                    "supports_streaming": true,
                    "max_tokens": 4000,
                    "cost_per_1k_tokens": 0.0030
                }),
                "ollama-qwen3-4b" => serde_json::json!({
                    "name": "ollama-qwen3-4b",
                    "provider": "ollama",
                    "description": "Ollama本地Qwen3 4B参数版本",
                    "supports_streaming": true,
                    "max_tokens": 4096,
                    "cost_per_1k_tokens": 0.0001
                }),
                "ollama-llama3" => serde_json::json!({
                    "name": "ollama-llama3",
                    "provider": "ollama",
                    "description": "Ollama本地Llama3模型",
                    "supports_streaming": true,
                    "max_tokens": 4096,
                    "cost_per_1k_tokens": 0.00001
                }),
                "calphamesh" => serde_json::json!({
                    "name": "calphamesh",
                    "provider": "qwen",
                    "description": "Calphamesh智能体，可以调用Calphamesh工具",
                    "supports_streaming": true,
                    "max_tokens": 4096,
                    "cost_per_1k_tokens": 0.0010
                }),
                "phase-field" => serde_json::json!({
                    "name": "phase-field",
                    "provider": "qwen",
                    "description": "Phase-field智能体，可以调用Phase-field工具",
                    "supports_streaming": true,
                    "max_tokens": 4096,
                    "cost_per_1k_tokens": 0.0010
                }),
                "ml-server" => serde_json::json!({
                    "name": "ml-server",
                    "provider": "qwen",
                    "description": "ML-Server智能体，可以调用ONNX-Server工具",
                    "supports_streaming": true,
                    "max_tokens": 4096,
                    "cost_per_1k_tokens": 0.0010
                }),
                "coating" => serde_json::json!({
                    "name": "coating",
                    "provider": "qwen",
                    "description": "涂层优化智能体，可以调用涂层优化工具(测试)",
                    "supports_streaming": true,
                    "max_tokens": 4096,
                    "cost_per_1k_tokens": 0.0010
                }),
                _ => serde_json::json!({
                    "name": model,
                    "provider": "unknown",
                    "description": "未知模型",
                    "supports_streaming": false,
                    "max_tokens": 2000,
                    "cost_per_1k_tokens": 0.0
                })
            }
        }).collect::<Vec<_>>(),
        "total": models.len(),
        "timestamp": chrono::Local::now()
    });

    Json(response)
}
