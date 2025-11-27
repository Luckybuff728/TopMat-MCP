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
use tracing::info;

use super::tool_registry::ToolRegistry;

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
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        info!("调用工具: {}（单例模式）", name);

        // 将 arguments 转换为 JsonValue
        let args_value = if let Some(args) = arguments {
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
