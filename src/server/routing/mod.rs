use axum::{
    routing::{get, post, put, delete},
    Router,
};

use crate::server::handlers::{
    auth_handler, chat_handler, list_models_handler,
    // 对话管理
    list_conversations_handler,
    create_conversation_handler,
    get_conversation_handler,
    update_conversation_title_handler,
    delete_conversation_handler,
    // 消息管理
    list_messages_handler,
    get_message_handler,
    delete_message_handler,
    // 使用统计和健康检查
    get_usage_stats_handler, health_check_handler,
};
use crate::server::handlers::chat::ServerState;

/// 创建公开路由（无需认证）
pub fn create_public_routes() -> Router<ServerState> {
    Router::new()
        // 认证本身
        .route("/v1/auth", post(auth_handler))
        // 模型列表（公开信息）
        .route("/v1/models", get(list_models_handler))
        // 健康检查
        .route("/health", get(health_check_handler))
}

/// 创建受保护的路由（需要认证）
/// 注意：这个函数当前未使用，所有路由在 server.rs 中配置
#[allow(dead_code)]
pub fn create_protected_routes() -> Router<ServerState> {
    // 创建聊天路由，单独添加消息记录中间件
    let chat_routes = Router::new()
        .route("/v1/chat", post(chat_handler));
    
    // 其他路由
    let other_routes = Router::new()
        // 使用统计（需要认证）
        .route("/usage/stats", get(get_usage_stats_handler))
        // 对话管理（需要认证）
        .route("/v1/conversations", get(list_conversations_handler))
        .route("/v1/conversations", post(create_conversation_handler))
        .route("/v1/conversations/:id", get(get_conversation_handler))
        .route("/v1/conversations/:id/title", put(update_conversation_title_handler))
        .route("/v1/conversations/:id", delete(delete_conversation_handler))
        // 消息管理（需要认证）
        .route("/v1/conversations/:id/messages", get(list_messages_handler))
        // .route("/v1/conversations/:id/messages", post(add_message_handler))
        .route("/v1/conversations/:id/messages/:message_id", get(get_message_handler))
        .route("/v1/conversations/:id/messages/:message_id", delete(delete_message_handler));
    
    // 合并路由
    Router::new()
        .merge(chat_routes)
        .merge(other_routes)
    }


