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
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::info;

use super::tool_registry::ToolRegistry;

// 全局工具注册表（延迟初始化）
static TOOL_REGISTRY: OnceCell<Arc<ToolRegistry>> = OnceCell::const_new();

/// 获取或初始化工具注册表
async fn get_tool_registry() -> Arc<ToolRegistry> {
    TOOL_REGISTRY.get_or_init(|| async {
        info!("初始化工具注册表...");
        Arc::new(ToolRegistry::new().await)
    }).await.clone()
}

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
                "这是 TopMat-LLM 的 MCP 服务器，提供光学薄膜设计和优化相关的工具。\n\
                \n\
                所有工具都从 src/server/mcp/tools 目录自动加载，包括：\n\
                - 思考和推理工具\n\
                - 涂层沉积模拟\n\
                - 机器学习性能预测\n\
                - 历史数据查询\n\
                - 实验数据读取\n\
                \n\
                这些工具可以协同工作，完成从模拟、预测到数据分析的完整流程。".to_string()
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        info!("列出所有可用工具");
        
        let registry = get_tool_registry().await;
        let tools = registry.get_tool_definitions();
        
        info!("共有 {} 个工具可用", tools.len());
        
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
        info!("调用工具: {}", name);
        
        let registry = get_tool_registry().await;
        
        // 将 arguments 转换为 JsonValue
        let args_value = if let Some(args) = arguments {
            serde_json::Value::Object(args)
        } else {
            json!({})
        };
        
        // 调用工具
        match registry.call_tool(&name, args_value).await {
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
