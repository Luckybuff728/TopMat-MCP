use axum::{extract::{Json, State, Extension}};
use tracing::info;

use crate::server::models::*;
use crate::server::database::DatabaseConnection;
use crate::server::middleware::auth::AuthUser;

/// 服务器状态
#[derive(Clone)]
pub struct ServerState {
    /// 数据库连接
    pub database: DatabaseConnection,
    /// 鉴权客户端
    pub auth_client: crate::server::auth::AuthClient,
}

impl ServerState {
    pub async fn new(database: DatabaseConnection) -> Self {
        // 创建鉴权客户端
        let auth_api_url = std::env::var("AUTH_API_URL").ok();
        let auth_client = crate::server::auth::AuthClient::new(auth_api_url, database.clone());
        info!("鉴权客户端已初始化");

        Self { database, auth_client }
    }
}

/// 处理聊天请求
/// 注意：conversation_id 由 MessageLogger 中间件确保存在并保持一致
pub async fn chat_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Json(request): Json<ChatRequest>,
) -> Result<axum::response::Response, ErrorResponse> {
    let conversation_id = request.conversation_id.as_ref()
        .expect("conversation_id should be set by MessageLogger middleware");

    info!(
        "收到聊天请求: model={}, stream={}, message={}, user_id={}, conversation_id={}",
        request.model, request.stream, request.message, auth_user.user_id, conversation_id
    );

    // 处理聊天请求
    // conversation_id 已由 MessageLogger 中间件确保存在
    // 消息保存也由 MessageLogger 中间件自动处理
    let (response, _chat_response) = crate::server::model_router::get_model_router()
        .handle_chat_request_with_response(request.clone())
        .await?;

    Ok(response)
}