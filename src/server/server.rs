use axum::{Router, middleware, http::{HeaderValue, Method, HeaderMap}, response::IntoResponse, extract::Request};
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;
use regex::Regex;

// 重新导出模块，保持向后兼容性
pub use crate::server::handlers::{auth_handler, chat_handler};

/// 使用正则表达式检查是否允许的域名
fn is_allowed_origin(origin: &str) -> bool {
    // 127.0.0.1 和 localhost 的所有端口
    let localhost_regex = Regex::new(r"^https?://(127\.0\.0\.1|localhost)(:\d+)?$").unwrap();
    if localhost_regex.is_match(origin) {
        return true;
    }

    // 192.168 开头的所有域名和端口
    let lan_regex = Regex::new(r"^https?://192\.168\.").unwrap();
    if lan_regex.is_match(origin) {
        return true;
    }

    false
}

/// 创建 CORS 层 - 使用 tower-http 的标准 CORS 支持
fn create_cors_layer() -> CorsLayer {
    // 创建允许所有域名的宽松 CORS 策略
    // 注意：在生产环境中应该更严格地配置允许的域名
    CorsLayer::very_permissive()
        // 允许凭证
        .allow_credentials(true)
}

/// 创建服务器实例
pub async fn create_server(database: crate::server::database::DatabaseConnection) -> Router {
    let state = crate::server::handlers::chat::ServerState::new(database).await;

    // 创建聊天路由，添加消息记录中间件（在认证之后）
    let chat_route = Router::new()
        .route("/v1/chat", axum::routing::post(crate::server::handlers::chat_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::message_logger::MessageLogger::log_messages,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::auth::AuthMiddleware::authenticate_request,
        ));
    
    // 创建 MCP 服务（需要认证，但对 GET 请求可能需要特殊处理）
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, 
        session::local::LocalSessionManager,
    };
    
    let mcp_service = StreamableHttpService::new(
        || Ok(crate::server::mcp::create_mcp_server()),
        LocalSessionManager::default().into(),
        Default::default(),
    );
    
    // MCP 路由 - 先应用 MCP 服务，再应用 MCP 专用认证中间件（只对 POST 请求认证）
    let mcp_route = Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::mcp_auth::McpAuthMiddleware::authenticate_request,
        ));
    
    // 创建其他受保护的路由（只需要认证）
    let other_protected_routes = Router::new()
        .route("/usage/stats", axum::routing::get(crate::server::handlers::get_usage_stats_handler))
        .route("/v1/conversations", axum::routing::get(crate::server::handlers::list_conversations_handler))
        .route("/v1/conversations", axum::routing::post(crate::server::handlers::create_conversation_handler))
        .route("/v1/conversations/:id", axum::routing::get(crate::server::handlers::get_conversation_handler))
        .route("/v1/conversations/:id/title", axum::routing::put(crate::server::handlers::update_conversation_title_handler))
        .route("/v1/conversations/:id", axum::routing::delete(crate::server::handlers::delete_conversation_handler))
        .route("/v1/conversations/:id/messages", axum::routing::get(crate::server::handlers::list_messages_handler))
        .route("/v1/conversations/:id/messages/:message_id", axum::routing::get(crate::server::handlers::get_message_handler))
        .route("/v1/conversations/:id/messages/:message_id", axum::routing::delete(crate::server::handlers::delete_message_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::auth::AuthMiddleware::authenticate_request,
        ));

    // 创建公开路由
    let public_routes = crate::server::routing::create_public_routes();

    Router::new()
        .merge(public_routes)
        .merge(chat_route)
        .merge(mcp_route)
        .merge(other_protected_routes)
        .layer(create_cors_layer())
        .with_state(state)
}