use axum::{Router, middleware, http::{HeaderValue, Method, HeaderMap}, response::IntoResponse, extract::Request};
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;
use regex::Regex;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, 
    session::local::LocalSessionManager,
};
use rmcp::transport::sse_server::SseServer;

// 重新导出模块，保持向后兼容性
// pub use crate::server::handlers::{auth_handler, chat_handler};

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

    // 创建聊天路由，添加消息存储中间件（在认证之后）
    let chat_route = Router::new()
        .route("/v1/chat", axum::routing::post(crate::server::handlers::chat_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::message_storage::MessageStorage::store_messages,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::auth::AuthMiddleware::authenticate_request,
        ));
    
    let mcp_service = StreamableHttpService::new(
        || Ok(crate::server::mcp::create_mcp_server()),
        LocalSessionManager::default().into(),
        Default::default(),
    );
    
    // MCP 路由 - 先应用 MCP 服务，再应用 MCP 数据存储中间件，最后应用认证中间件
    let mcp_route = Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::mcp_storage::McpStorage::store_mcp_data,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::mcp_auth::McpAuthMiddleware::authenticate_request,
        ));

    // SSE MCP 路由 - 使用 SSE 协议提供 MCP 服务，集成到现有路由中
    use rmcp::transport::sse_server::SseServerConfig;
    use tokio_util::sync::CancellationToken;

    // Create SSE server config
    // Note: bind address is required but won't be used when integrating router
    let sse_config = SseServerConfig {
        bind: "127.0.0.1:0".parse().unwrap(), // Use port 0 to let OS choose unused port
        sse_path: "/".to_string(), // Root path for the SSE service
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };
    let (mut sse_server, sse_router) = SseServer::new(sse_config.clone());

    // 启动任务来处理 SSE 传输，正确连接到 MCP 服务器
    let ct = sse_config.ct.clone();
    let _transport_handle = tokio::spawn(async move {
        use futures::StreamExt;
        use rmcp::service::serve_directly_with_ct;

        let mut transport_count = 0;
        while let Some(transport) = sse_server.next().await {
            transport_count += 1;
            tracing::info!("处理 SSE 传输 #{}", transport_count);

            // 创建 MCP 服务器实例
            let mcp_server = crate::server::mcp::create_mcp_server();

            // 将传输连接到 MCP 服务器
            let server = serve_directly_with_ct(mcp_server, transport, None, ct.clone());
            tracing::info!("MCP 服务器启动成功，等待连接...");

            // 等待服务器结束
            if let Err(e) = server.waiting().await {
                tracing::error!("MCP 服务器运行错误: {}", e);
            }
        }
        tracing::info!("SSE 传输处理结束");
    });

    // SSE MCP 路由 - 集成 SSE 路由器，添加数据存储和认证中间件
    let sse_mcp_route = Router::new()
        .nest_service("/sse", sse_router)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::mcp_storage::McpStorage::store_mcp_data,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::mcp_auth::McpAuthMiddleware::authenticate_request,
        ));

    // 创建其他受保护的路由（只需要认证）
    let other_protected_routes = Router::new()
        // 原有的API端点
        .route("/usage/stats", axum::routing::get(crate::server::handlers::get_usage_stats_handler))
        .route("/v1/conversations", axum::routing::get(crate::server::handlers::list_conversations_handler))
        .route("/v1/conversations", axum::routing::post(crate::server::handlers::create_conversation_handler))
        .route("/v1/conversations/{id}", axum::routing::get(crate::server::handlers::get_conversation_handler))
        .route("/v1/conversations/{id}/title", axum::routing::put(crate::server::handlers::update_conversation_title_handler))
        .route("/v1/conversations/{id}", axum::routing::delete(crate::server::handlers::delete_conversation_handler))
        .route("/v1/conversations/{id}/messages", axum::routing::get(crate::server::handlers::list_messages_handler))
        .route("/v1/conversations/{id}/messages/{message_id}", axum::routing::get(crate::server::handlers::get_message_handler))
        .route("/v1/conversations/{id}/messages/{message_id}", axum::routing::delete(crate::server::handlers::delete_message_handler))
        // MCP统计API端点
        .route("/usage/mcp/stats", axum::routing::get(crate::server::handlers::mcp_stats::get_mcp_usage_stats_handler))
        .route("/usage/mcp/sessions", axum::routing::get(crate::server::handlers::mcp_stats::get_mcp_sessions_handler))
        .route("/usage/mcp/tool-calls", axum::routing::get(crate::server::handlers::mcp_stats::get_mcp_tool_calls_handler))
        .route("/usage/comprehensive", axum::routing::get(crate::server::handlers::mcp_stats::get_comprehensive_stats_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::auth::AuthMiddleware::authenticate_request,
        ));

    // 创建公开路由
    let public_routes = Router::new()
        // 认证本身
        .route("/v1/auth", axum::routing::post(crate::server::handlers::auth_handler))
        // 模型列表（公开信息）
        .route("/v1/models", axum::routing::get(crate::server::handlers::list_models_handler))
        // 健康检查
        .route("/health", axum::routing::get(crate::server::handlers::health_check_handler));


    Router::new()
        .merge(public_routes)
        .merge(chat_route)
        .merge(mcp_route)
        .merge(sse_mcp_route)
        .merge(other_protected_routes)
        .layer(create_cors_layer())
        .with_state(state)
}