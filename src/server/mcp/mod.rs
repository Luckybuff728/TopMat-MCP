//! MCP (Model Context Protocol) 相关功能模块
//! 
//! 这个模块包含与 MCP 服务器交互相关的所有功能，包括：
//! - McpAgent: 持有 MCP 客户端生命周期的 Agent 包装器
//! - McpServer: 对外提供 MCP 服务的服务器实现
//! - ToolRegistry: 自动注册和管理所有工具

pub mod mcp_agent;
pub mod mcp_server;
pub mod tool_registry;
pub mod tool_macros;

pub use mcp_agent::McpAgent;
pub use mcp_server::{create_mcp_server, TopMatMcpServer};
pub use tool_registry::ToolRegistry;

pub mod tools;
