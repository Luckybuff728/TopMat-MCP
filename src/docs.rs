//! OpenAPI 文档配置模块
//!
//! 此模块定义了 TopMat-LLM 服务的 OpenAPI 3.0 规范，
//! 用于生成交互式 API 文档。

use utoipa::{OpenApi, ToSchema, Modify};
use utoipa::openapi::{security::{SecurityScheme, HttpAuthScheme, SecurityRequirement, HttpBuilder}, path::PathItem};

// 重新导出 OpenAPI 模型
pub use crate::server::models::*;

/// TopMat-LLM API 的 OpenAPI 文档配置
///
/// 此结构定义了 API 的元数据、路径、组件和标签，
/// 用于生成完整的 OpenAPI 3.0 规范文档。
///
/// 第二阶段：已添加主要端点的路径注解和安全配置。
#[derive(OpenApi)]
#[openapi(
    info(
        title = "TopMat-LLM API",
        description = "统一 LLM 聊天服务器 API，提供多模型提供商支持、对话管理、使用统计和 MCP 工具集成",
        version = "0.1.0",
        contact(
            name = "TopMat-LLM Support",
            email = "fengmengqi@topmaterial-tech.com"
        )
    ),
    // servers(
    //     (url = "http://127.0.0.1:8081", description = "开发环境"),
    //     (url = "http://localhost:10007", description = "Docker 环境"),
    //     (url = "https://api.topmaterial-tech.com", description = "生产环境")
    // ),
    paths(
        crate::server::handlers::usage::health_check_handler,
        crate::server::handlers::usage::get_usage_stats_handler,
        crate::server::handlers::auth::auth_handler,
        crate::server::handlers::models::list_models_handler,
        crate::server::handlers::chat::chat_handler,
        // Conversation management endpoints
        crate::server::handlers::conversations::list_conversations_handler,
        crate::server::handlers::conversations::create_conversation_handler,
        crate::server::handlers::conversations::get_conversation_handler,
        crate::server::handlers::conversations::update_conversation_title_handler,
        crate::server::handlers::conversations::delete_conversation_handler,
        // Message management endpoints
        crate::server::handlers::messages::list_messages_handler,
        crate::server::handlers::messages::get_message_handler,
        crate::server::handlers::messages::delete_message_handler,
        // Usage statistics endpoints
        crate::server::handlers::mcp_stats::get_mcp_usage_stats_handler,
        crate::server::handlers::mcp_stats::get_mcp_sessions_handler,
        crate::server::handlers::mcp_stats::get_mcp_tool_calls_handler,
        crate::server::handlers::mcp_stats::get_comprehensive_stats_handler,
        // MCP endpoints (documentation-only handlers)
        crate::server::handlers::mcp_docs::mcp_info_handler,
        crate::server::handlers::mcp_docs::mcp_tools_list_handler,
        crate::server::handlers::mcp_docs::mcp_tool_call_handler,
        crate::server::handlers::mcp_docs::sse_info_handler,
        crate::server::handlers::mcp_docs::sse_message_handler,
    ),
    components(
        schemas(
            AuthRequest,
            AuthResponse,
            ModelInfo,
            HealthCheckResponse,
            ErrorResponse,
            ChatRequest,
            ChatResponse,
            TokenUsage,
            // Conversation management models
            Conversation,
            CreateConversationRequest,
            ListConversationsQuery,
            ConversationListResponse,
            CreateConversationResponse,
            ListMessagesQuery,
            MessageListResponse,
            UpdateConversationTitleRequest,
            // Usage statistics models
            McpStatsQuery,
            McpUsageStats,
            McpSessionInfo,
            McpToolCallInfo,
            ComprehensiveStats,
            // MCP protocol models
            McpServerInfo,
            McpToolInfo,
            McpToolCallRequest,
            McpToolCallResponse,
            McpContent,
            McpInitializeRequest,
            McpClientInfo,
            McpInitializeResponse,
        )
    ),
    tags(
        (name = "health", description = "健康检查相关接口（无需认证）"),
        (name = "models", description = "模型信息相关接口（无需认证）"),
        (name = "auth", description = "身份认证相关接口"),
        (name = "chat", description = "聊天对话相关接口"),
        (name = "conversations", description = "对话管理相关接口"),
        (name = "mcp", description = "MCP 工具相关接口"),
        (name = "usage", description = "使用统计相关接口")
        
    ),
    modifiers(&BearerTokenSecurityAddon)
)]
pub struct ApiDoc;

/// Bearer Token 安全方案修饰符
struct BearerTokenSecurityAddon;

impl Modify for BearerTokenSecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // 添加安全方案到 components
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("API Key")
                    .build(),
            ),
        );

        // 定义不需要认证的路径
        let public_paths = [
            "/health",
            "/v1/models",
        ];

        
        let bearer_security_req = SecurityRequirement::new("bearerAuth", Vec::<String>::new());

        // 为所有路径应用安全要求，除了公开路径外都需要认证
        for (path, path_item) in openapi.paths.paths.iter_mut() {
            let path_str = path.as_str();

            // 检查路径是否为公开路径（不需要认证）
            let is_public = public_paths.iter().any(|&public_path| path_str.contains(public_path));

            // 如果不是公开路径，则需要认证
            if !is_public {
                // 为该路径的所有操作添加安全要求
                if let Some(ref mut operation) = path_item.get {
                    operation.security = Some(vec![bearer_security_req.clone()]);
                }
                if let Some(ref mut operation) = path_item.post {
                    operation.security = Some(vec![bearer_security_req.clone()]);
                }
                if let Some(ref mut operation) = path_item.put {
                    operation.security = Some(vec![bearer_security_req.clone()]);
                }
                if let Some(ref mut operation) = path_item.delete {
                    operation.security = Some(vec![bearer_security_req.clone()]);
                }
                if let Some(ref mut operation) = path_item.patch {
                    operation.security = Some(vec![bearer_security_req.clone()]);
                }
                if let Some(ref mut operation) = path_item.options {
                    operation.security = Some(vec![bearer_security_req.clone()]);
                }
                if let Some(ref mut operation) = path_item.head {
                    operation.security = Some(vec![bearer_security_req.clone()]);
                }
                if let Some(ref mut operation) = path_item.trace {
                    operation.security = Some(vec![bearer_security_req.clone()]);
                }
            }
        }
    }
}

