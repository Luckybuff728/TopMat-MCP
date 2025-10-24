use axum::{Router, middleware};

// 重新导出模块，保持向后兼容性
pub use crate::server::handlers::{auth_handler, chat_handler};

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
        .with_state(state)
}