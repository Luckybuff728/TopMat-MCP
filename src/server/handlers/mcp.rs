//! MCP 端点处理器
//! 
//! 提供 /mcp 端点，使用 rmcp 的 StreamableHttpService

// 注意：MCP 服务不是作为普通的 handler，而是作为 nest_service 添加到路由中
// 参见 src/server/routing/mod.rs 中的实现
