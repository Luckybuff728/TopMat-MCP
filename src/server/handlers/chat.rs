use axum::{extract::{Json, State, Extension}};
use tracing::info;
use utoipa::path;
use serde_json::json;

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
///
/// 发送消息到 AI 模型并获取回复。支持流式和非流式响应。
/// 如果未提供 conversation_id，系统会自动创建新的对话。
///
/// **认证方式**: Bearer Token
/// ```
/// Authorization: Bearer <your_api_key>
/// ```
#[utoipa::path(
    post,
    path = "/v1/chat",
    tag = "chat",
    summary = "聊天对话",
    description = "发送消息到 AI 模型并获取回复。支持流式和非流式响应。",
    request_body(
        content = ChatRequest,
        description = "聊天请求参数",
        example = json!({
            "message": "你好，请介绍一下你自己",
            "stream": false,
            "model": "qwen-plus",
            "system_prompt": "你是一个有用的AI助手",
            "temperature": 0.7,
            "max_tokens": 1000,
            "conversation_id": "123e4567-e89b-12d3-a456-426614174000"
        })
    ),
    responses(
        (status = 200, description = "请求成功", body = ChatResponse,
         example = json!({
             "content": "你好！我是一个AI助手，很高兴为您服务。",
             "model": "qwen-plus",
             "usage": {
                 "prompt_tokens": 20,
                 "completion_tokens": 15,
                 "total_tokens": 35
             },
             "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
             "timestamp": "2024-01-01T12:00:00Z",
             "metadata": {
                 "response_time_ms": 1500
             }
         })
        ),
        (status = 400, description = "请求参数错误", body = ErrorResponse),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 429, description = "请求过于频繁", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse),
        (status = 503, description = "AI 服务暂时不可用", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn chat_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Json(request): Json<ChatRequest>,
) -> Result<axum::response::Response, ErrorResponse> {
    let conversation_id = request.conversation_id.as_ref()
        .expect("conversation_id should be set by MessageStorage middleware");

    info!(
        "收到聊天请求: model={}, stream={}, message={}, user_id={}, conversation_id={}",
        request.model, request.stream, request.message, auth_user.user_id, conversation_id
    );

    // 处理聊天请求
    // conversation_id 已由 MessageStorage 中间件确保存在
    // 消息保存也由 MessageStorage 中间件自动处理
    let (response, _chat_response) = crate::server::model_router::get_model_router()
        .handle_chat_request_with_response(request.clone(), auth_user)
        .await?;

    Ok(response)
}