//! MCP 服务器实现
//! 
//! 使用 rmcp 的 StreamableHttp 对外提供 MCP 服务
//! 自动从 tools 目录加载所有工具

use rmcp::{
    ErrorData as McpError, ServerHandler,
    model::*,
    service::{RequestContext, RoleServer},
};
use serde_json::json;
use tracing::{info, warn};

use super::tool_registry::ToolRegistry;

// 内联实现从HTTP headers提取API key的函数


/// TopMat MCP 服务器
#[derive(Clone)]
pub struct TopMatMcpServer;

impl TopMatMcpServer {
    pub fn new() -> Self {
        Self
    }

    pub fn new_with_db(_database: Option<crate::server::database::DatabaseConnection>) -> Self {
        // 注意：这里暂时不存储数据库实例，因为我们通过RequestContext传递
        // 在实际部署时，可能需要重构为使用全局状态或依赖注入
        Self
    }
}

impl Default for TopMatMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for TopMatMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            server_info: Implementation {
                name: "TopMat-LLM MCP Server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("TopMat-LLM".to_string()),
                website_url: Some("https://lab.topmaterial-tech.com/".to_string()),
                icons: None,
            },
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            instructions: Some(
                "这是 TopMat-LLM 的 MCP 服务器，提供材料模拟和计算相关的工具。\n\
                \n\
                所有工具都从 src/server/mcp/tools 目录自动加载，包括：\n\
                - 思考和推理工具\n\
                - Calpha Mesh 相图计算工具\n\
                - ONNX Service 模型推理工具\n\
                - RAG 知识库检索工具（钢铁、硬质合金、AL_IDME）\n\
                - Phase Field 相场模拟工具\n\
                \n\
                这些工具可以协同工作，完成从材料模拟、预测到数据分析的完整流程。\n\
                \n\
                优化特性：使用单例模式避免重复工具注册，提高内存效率。".to_string()
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        info!("列出所有可用工具（单例模式）");

        let tools = ToolRegistry::get_tool_definitions().await;

        info!("共有 {} 个工具可用（全局共享实例）", tools.len());
        
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        CallToolRequestParam { name, arguments }: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        info!("调用工具: {}（单例模式）", name);

        // 打印请求体（工具参数）
        if let Some(ref args) = arguments {
            info!("📥 请求体 (arguments): {}", serde_json::to_string_pretty(&serde_json::Value::Object(args.clone())).unwrap_or_default());
        } else {
            info!("📥 请求体 (arguments): {{}}");
        }

        // 打印请求头
        if let Some(http_parts) = context.extensions.get::<axum::http::request::Parts>() {
            let headers_str: String = http_parts.headers.iter()
                .map(|(k, v)| format!("  {}: {}", k, v.to_str().unwrap_or("<binary>")))
                .collect::<Vec<_>>()
                .join("\n");
            info!("📋 请求头:\n{}", headers_str);
        }

        // 记录开始时间用于执行时间统计
        let start_time = std::time::Instant::now();

        // 提取用户信息和会话上下文
        let (user_id, session_id, transport_type, endpoint) =
            Self::extract_context_info(&context);

        // 为 calphamesh 工具自动注入 API key
        let mut modified_arguments = arguments;
        if name.starts_with("calphamesh_") {
            // 尝试获取 axum::http::request::Parts 类型的扩展
            if let Some(http_parts) = context.extensions.get::<axum::http::request::Parts>() {
                info!("http_parts: {:#?}", http_parts);
                // 尝试从中提取 API key
                if let Some(api_key) = crate::server::auth::extract_api_key_from_headers(&http_parts.headers) {
                    info!("从 HTTP headers 成功提取 API key: {}...", &api_key[..std::cmp::min(4, api_key.len())]);

                    let mut args_map: serde_json::Map<String, serde_json::Value> =
                        modified_arguments.unwrap_or_default().into_iter().collect();
                    args_map.insert("api_key".to_string(), serde_json::Value::String(api_key));
                    modified_arguments = Some(args_map);

                    info!("为工具 {} 从 HTTP headers 注入 API key", name);
                } else {
                    warn!("HTTP headers 中也未找到有效的 API key");
                }
            } else {
                warn!("RequestContext 中也没有 axum::http::request::Parts 扩展");
                warn!("这表明 rmcp 框架没有正确传递 Axum 的 extensions");
            }

        }

        // 将 arguments 转换为 JsonValue
        let args_value = if let Some(args) = modified_arguments {
            serde_json::Value::Object(args)
        } else {
            json!({})
        };

        // 调用工具（使用单例方法）
        let execution_result = match ToolRegistry::call_tool(&name, args_value.clone()).await {
            Ok(result) => {
                info!("工具 {} 执行成功", name);
                let execution_time = start_time.elapsed().as_millis() as i32;

                // 记录成功的工具调用
                if let (Some(user_id), Some(transport_type), Some(endpoint)) =
                    (user_id, transport_type.as_ref(), endpoint.as_ref()) {
                    if let Err(e) = Self::record_tool_call_to_db(
                        user_id,
                        session_id.as_deref(),
                        &name,
                        &args_value,
                        &serde_json::json!(result),
                        execution_time,
                        "success",
                        None,
                        transport_type,
                        endpoint,
                    ).await {
                        warn!("记录工具调用成功结果失败: {}", e);
                    }
                }

                Ok(CallToolResult::success(vec![Content::text(result)]))
            }
            Err(e) => {
                tracing::error!("工具 {} 执行失败: {}", name, e);
                let execution_time = start_time.elapsed().as_millis() as i32;

                // 记录失败的工具调用
                if let (Some(user_id), Some(transport_type), Some(endpoint)) =
                    (user_id, transport_type.as_ref(), endpoint.as_ref()) {
                    if let Err(e) = Self::record_tool_call_to_db(
                        user_id,
                        session_id.as_deref(),
                        &name,
                        &args_value,
                        &serde_json::Value::Null,
                        execution_time,
                        "error",
                        Some(&e.to_string()),
                        transport_type,
                        endpoint,
                    ).await {
                        warn!("记录工具调用错误结果失败: {}", e);
                    }
                }

                Err(McpError::internal_error(
                    format!("Tool execution failed: {}", e),
                    None,
                ))
            }
        };

        execution_result
    }
}

impl TopMatMcpServer {
    /// 从RequestContext中提取用户信息和会话上下文
    fn extract_context_info(
        context: &RequestContext<RoleServer>,
    ) -> (Option<i64>, Option<String>, Option<String>, Option<String>) {
        let mut user_id = None;
        let mut session_id = None;
        let mut transport_type = None;
        let mut endpoint = None;

        // 尝试从MCP会话上下文中获取信息
        if let Some(mcp_context) = context.extensions.get::<crate::server::middleware::mcp_storage::McpSessionContext>() {
            user_id = Some(mcp_context.user_id);
            session_id = Some(mcp_context.session_id.clone());
            transport_type = Some(mcp_context.transport_type.clone());
            endpoint = Some(mcp_context.endpoint.clone());
        }

        // 如果没有找到MCP上下文，尝试从AuthUser中获取用户信息
        if user_id.is_none() {
            if let Some(auth_user) = context.extensions.get::<crate::server::middleware::auth::AuthUser>() {
                user_id = Some(auth_user.user_id as i64);
            }
        }

        // 默认传输类型和端点（如果没有从中间件获取）
        if transport_type.is_none() {
            transport_type = Some("http".to_string());
            endpoint = Some("/mcp".to_string());
        }

        (user_id, session_id, transport_type, endpoint)
    }

    /// 记录工具调用到数据库
    async fn record_tool_call_to_db(
        user_id: i64,
        session_id: Option<&str>,
        tool_name: &str,
        arguments: &serde_json::Value,
        result: &serde_json::Value,
        execution_time_ms: i32,
        status: &str,
        error_message: Option<&str>,
        transport_type: &str,
        endpoint: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 注意：这里我们暂时不能直接访问数据库连接
        // 因为MCP服务器运行在独立的环境中
        // 实际的数据记录将在中间件层面完成
        // 这里只是为了保持API一致性

        info!(
            "MCP工具调用记录: user_id={}, session_id={}, tool={}, status={}, time={}ms, transport={}, endpoint={}",
            user_id,
            session_id.unwrap_or("none"),
            tool_name,
            status,
            execution_time_ms,
            transport_type,
            endpoint
        );

        Ok(())
    }
}

/// 创建 MCP 服务器实例
pub fn create_mcp_server() -> TopMatMcpServer {
    TopMatMcpServer::new()
}
