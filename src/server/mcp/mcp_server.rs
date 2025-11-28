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
        match ToolRegistry::call_tool(&name, args_value).await {
            Ok(result) => {
                info!("工具 {} 执行成功", name);
                Ok(CallToolResult::success(vec![Content::text(result)]))
            }
            Err(e) => {
                tracing::error!("工具 {} 执行失败: {}", name, e);
                Err(McpError::internal_error(
                    format!("Tool execution failed: {}", e),
                    None,
                ))
            }
        }
    }
}

/// 创建 MCP 服务器实例
pub fn create_mcp_server() -> TopMatMcpServer {
    TopMatMcpServer::new()
}
