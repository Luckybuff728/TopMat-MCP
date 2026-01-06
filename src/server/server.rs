use axum::{Router, middleware};
use regex::Regex;
use rmcp::transport::sse_server::SseServer;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// 重新导出模块，保持向后兼容性
// pub use crate::server::handlers::{auth_handler, chat_handler};

/// 使用正则表达式检查是否允许的域名
#[allow(dead_code)]
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
        .route(
            "/v1/chat",
            axum::routing::post(crate::server::handlers::chat_handler),
        )
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

    // MCP 路由 - 执行顺序：认证 → 数据存储 → MCP 服务 (layer 从下到上执行)
    let mcp_route = Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::mcp_storage::McpStorage::store_mcp_data,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::auth::AuthMiddleware::authenticate_request_skip_get,
        ));

    // SSE MCP 路由 - 使用 SSE 协议提供 MCP 服务，集成到现有路由中
    use rmcp::transport::sse_server::SseServerConfig;
    use tokio_util::sync::CancellationToken;

    // Create SSE server config
    // Note: bind address is required but won't be used when integrating router
    // We don't set a global CancellationToken here since each connection gets its own
    let sse_config = SseServerConfig {
        bind: "127.0.0.1:0".parse().unwrap(), // Use port 0 to let OS choose unused port
        sse_path: "/".to_string(),            // Root path for the SSE service
        post_path: "/message".to_string(),
        ct: CancellationToken::new(), // This token won't be used for individual connections
        sse_keep_alive: None,
    };
    let (mut sse_server, sse_router) = SseServer::new(sse_config.clone());

    // 启动任务来处理 SSE 传输，为每个连接创建独立的 CancellationToken
    let _transport_handle = tokio::spawn(async move {
        use futures::StreamExt;
        use rmcp::service::serve_directly_with_ct;

        let mut transport_count = 0;
        while let Some(transport) = sse_server.next().await {
            transport_count += 1;
            tracing::info!("处理 SSE 传输 #{}", transport_count);

            // 为每个连接创建独立的 CancellationToken
            let connection_ct = tokio_util::sync::CancellationToken::new();

            // 创建 MCP 服务器实例
            let mcp_server = crate::server::mcp::create_mcp_server();

            // 将传输连接到 MCP 服务器，使用独立的 token
            let server = serve_directly_with_ct(mcp_server, transport, None, connection_ct);
            tracing::info!("MCP 服务器启动成功，等待连接...");

            // 等待服务器结束
            if let Err(e) = server.waiting().await {
                tracing::error!("MCP 服务器运行错误: {}", e);
            }
        }
        tracing::info!("SSE 传输处理结束");
    });

    // SSE MCP 路由 - 执行顺序：认证 → 数据存储 → SSE 服务 (layer 从下到上执行)
    let sse_mcp_route = Router::new()
        .nest_service("/sse", sse_router)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::mcp_storage::McpStorage::store_mcp_data,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::auth::AuthMiddleware::authenticate_request_skip_get,
        ));

    // 创建其他受保护的路由（只需要认证）
    let other_protected_routes = Router::new()
        // 原有的API端点
        .route(
            "/usage/stats",
            axum::routing::get(crate::server::handlers::get_usage_stats_handler),
        )
        .route(
            "/v1/conversations",
            axum::routing::get(crate::server::handlers::list_conversations_handler),
        )
        .route(
            "/v1/conversations",
            axum::routing::post(crate::server::handlers::create_conversation_handler),
        )
        .route(
            "/v1/conversations/{id}",
            axum::routing::get(crate::server::handlers::get_conversation_handler),
        )
        .route(
            "/v1/conversations/{id}/title",
            axum::routing::put(crate::server::handlers::update_conversation_title_handler),
        )
        .route(
            "/v1/conversations/{id}",
            axum::routing::delete(crate::server::handlers::delete_conversation_handler),
        )
        .route(
            "/v1/conversations/{id}/messages",
            axum::routing::get(crate::server::handlers::list_messages_handler),
        )
        .route(
            "/v1/conversations/{id}/messages/{message_id}",
            axum::routing::get(crate::server::handlers::get_message_handler),
        )
        .route(
            "/v1/conversations/{id}/messages/{message_id}",
            axum::routing::delete(crate::server::handlers::delete_message_handler),
        )
        // MCP统计API端点
        .route(
            "/usage/mcp/stats",
            axum::routing::get(crate::server::handlers::mcp_stats::get_mcp_usage_stats_handler),
        )
        .route(
            "/usage/mcp/sessions",
            axum::routing::get(crate::server::handlers::mcp_stats::get_mcp_sessions_handler),
        )
        .route(
            "/usage/mcp/tool-calls",
            axum::routing::get(crate::server::handlers::mcp_stats::get_mcp_tool_calls_handler),
        )
        .route(
            "/usage/comprehensive",
            axum::routing::get(crate::server::handlers::mcp_stats::get_comprehensive_stats_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::middleware::auth::AuthMiddleware::authenticate_request,
        ));

    // 创建公开路由
    let public_routes = Router::new()
        // 认证本身
        .route(
            "/v1/auth",
            axum::routing::post(crate::server::handlers::auth_handler),
        )
        // 模型列表（公开信息）
        .route(
            "/v1/models",
            axum::routing::get(crate::server::handlers::list_models_handler),
        )
        // 健康检查
        .route(
            "/health",
            axum::routing::get(crate::server::handlers::health_check_handler),
        );

    // 创建 Swagger UI 路由
    let swagger_ui = SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", crate::docs::ApiDoc::openapi())
        .config(
            utoipa_swagger_ui::Config::default()
                .try_it_out_enabled(true) // 启用"Try it out"功能
                .display_request_duration(true) // 显示请求耗时
                .filter(true) // 启用过滤功能
                .deep_linking(true) // 启用深度链接
                .persist_authorization(true) // 持久化认证信息
                .with_credentials(true) // 允许发送认证信息
                .doc_expansion("list".to_string()) // 默认展开为列表视图
                .show_mutated_request(true) // 显示变更后的请求
                .supported_submit_methods(["get", "post", "put", "delete", "patch"]), // 支持的HTTP方法
        );

    Router::new()
        .merge(public_routes)
        .merge(chat_route)
        .merge(mcp_route)
        .merge(sse_mcp_route)
        .merge(other_protected_routes)
        .merge(swagger_ui) // 添加 Swagger UI 路由
        .layer(create_cors_layer())
        .with_state(state)
}
