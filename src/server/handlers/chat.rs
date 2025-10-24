use axum::{
    extract::{Json, State, Request},
    response::{IntoResponse, Response},
    body::Bytes,
};
use tracing::{info, error};
use sqlx::Row;
use http_body_util::BodyExt;

use crate::server::models::*;
use crate::server::auth::{AuthClient, extract_api_key};
use crate::server::database::DatabaseConnection;
use crate::server::database::models::{Conversation, Message};

/// 服务器状态
#[derive(Clone)]
pub struct ServerState {
    /// 标记是否配置了通义千问
    pub qwen_available: bool,
    /// 鉴权客户端
    pub auth_client: AuthClient,
    /// 数据库连接
    pub database: DatabaseConnection,
}

impl ServerState {
    pub async fn new(database: DatabaseConnection) -> Self {
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

        // 创建鉴权客户端
        let auth_api_url = std::env::var("AUTH_API_URL").ok();
        let auth_client = AuthClient::new(auth_api_url, database.clone());

        tracing::info!("鉴权客户端已初始化");

        Self {
            qwen_available,
            auth_client,
            database
        }
    }
}

/// 处理聊天请求
pub async fn chat_handler(
    State(state): State<ServerState>,
    Json(request): Json<ChatRequest>,
) -> Result<axum::response::Response, ErrorResponse> {
    info!("收到聊天请求: model={}, stream={}, message={}", request.model, request.stream, request.message);

    // 处理聊天请求
    let response = crate::server::model_router::get_model_router().handle_chat_request(request).await?;

    Ok(response)
}
