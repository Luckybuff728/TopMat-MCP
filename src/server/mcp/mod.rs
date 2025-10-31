//! MCP (Model Context Protocol) 相关功能模块
//! 
//! 这个模块包含与 MCP 服务器交互相关的所有功能，包括：
//! - McpAgent: 持有 MCP 客户端生命周期的 Agent 包装器

pub mod mcp_agent;

pub use mcp_agent::McpAgent;

