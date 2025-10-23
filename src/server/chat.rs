use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::post,
    Router,
};
use tracing::info;

use super::models::*;
use super::agent::{qwen, ollama};

/// 服务器状态
#[derive(Clone)]
pub struct ServerState {
    /// 标记是否配置了通义千问
    qwen_available: bool,
}

impl ServerState {
    pub fn new() -> Self {
        // 检查是否配置了通义千问API密钥
        let qwen_available = match std::env::var("DASHSCOPE_API_KEY") {
            Ok(_) => {
                tracing::info!("通义千问 API 密钥已配置，通义千问模型可用");
                true
            }
            Err(_) => {
                tracing::warn!("未配置 DASHSCOPE_API_KEY，通义千问模型将不可用。请在 .env 文件中配置 API 密钥。");
                false
            }
        };

        Self { qwen_available }
    }
}

/// 处理聊天请求
pub async fn chat_handler(
    State(state): State<ServerState>,
    Json(request): Json<ChatRequest>,
) -> Result<axum::response::Response, ErrorResponse> {
    info!("收到聊天请求: model={}, stream={}, message={}", request.model, request.stream, request.message);

    // 根据模型选择处理方式
    if request.model == "qwen-plus" {
        qwen::qwen_plus(request).await
    }
    else if request.model == "qwen-turbo" {
        qwen::qwen_turbo(request).await
    }
    else if request.model == "qwen-max" {
        qwen::qwen_max(request).await
    }
    else if request.model == "qwen-flash" {
        qwen::qwen_flash(request).await
    }
    else if request.model == "qwq-plus" {
        qwen::qwq_plus(request).await
    }
    else if request.model == "ollama-qwen3-4b" {
        ollama::ollama_qwen3_4b(request).await
    }
    else if request.model == "ollama-llama3" {
        ollama::ollama_llama3(request).await
    }
    else {
        Err(ErrorResponse {
            error: "model_not_supported".to_string(),
            message: format!("不支持的模型: {}", request.model),
            details: None,
            timestamp: chrono::Utc::now(),
        })
    }
}

/// 创建聊天路由
pub fn create_chat_router() -> Router<ServerState> {
    Router::new().route("/chat", post(chat_handler))
}

/// 创建服务器实例
pub fn create_server() -> Router {
    let state = ServerState::new();

    Router::new()
        .merge(create_chat_router())
        .with_state(state)
}

// 实现ErrorResponse的IntoResponse
impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;

        let status = match self.error.as_str() {
            "model_not_supported" => StatusCode::BAD_REQUEST,
            "qwen_not_configured" => StatusCode::BAD_REQUEST,
            "chat_failed" => StatusCode::INTERNAL_SERVER_ERROR,
            "streaming_chat_failed" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, axum::Json(self)).into_response()
    }
}