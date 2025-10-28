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

    // 创建受保护的路由并添加认证中间件
    let protected_routes = crate::server::routing::create_protected_routes()
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::auth::AuthMiddleware::authenticate_request,
        ));

    // 创建公开路由
    let public_routes = crate::server::routing::create_public_routes();

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(create_cors_layer())
        .with_state(state)
}