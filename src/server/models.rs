use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use utoipa::ToSchema;

/// 聊天请求结构
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ChatRequest {
    /// 用户输入的消息
    pub message: String,
    /// 是否使用流式响应
    #[serde(default)]
    pub stream: bool,
    /// 使用的模型名称
    #[serde(default = "default_model")]
    pub model: String,
    /// 系统提示词
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// 温度参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 最大token数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 会话ID（用于多轮对话，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// 额外的元数据（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

fn default_model() -> String {
    "qwen3:4b".to_string()
}

/// 聊天响应结构（非流式）
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ChatResponse {
    /// 响应内容
    pub content: String,
    /// 使用的模型
    pub model: String,
    /// Token使用情况
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    /// 会话ID
    pub conversation_id: String,
    /// 响应时间戳
    pub timestamp: chrono::DateTime<chrono::Local>,
    /// 额外的元数据
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Token使用情况
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TokenUsage {
    /// 提示词token数
    pub prompt_tokens: u32,
    /// 补全token数
    pub completion_tokens: u32,
    /// 总token数
    pub total_tokens: u32,
}

/// 流式响应的数据块
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamChunk {
    /// 文本内容块
    #[serde(rename = "content")]
    Text {
        /// 文本内容
        text: String,
        /// 是否为最后一个块
        #[serde(default)]
        finished: bool,
    },
    /// 推理过程
    #[serde(rename = "reasoning")]
    Reasoning {
        /// 推理内容
        reasoning: String,
    },
    /// 工具调用
    #[serde(rename = "tool_call")]
    ToolCall {
        /// 工具名称
        name: String,
        /// 工具参数
        arguments: serde_json::Value,
    },
    /// 工具响应
    #[serde(rename = "tool_result")]
    ToolResult {
        /// 工具调用ID
        id: String,
        /// 工具执行结果
        result: String,
    },
    /// 错误信息
    #[serde(rename = "error")]
    Error {
        /// 错误消息
        message: String,
    },
    /// 最终响应信息
    #[serde(rename = "final")]
    Final {
        /// 完整的响应信息
        response: ChatResponse,
    },
}

/// API错误响应
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// 错误类型
    pub error: String,
    /// 错误消息
    pub message: String,
    /// 错误详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Local>,
}

// ============== 鉴权相关数据结构 ==============

/// 用户信息结构
#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct UserInfo {
    /// 用户ID
    pub id: u32,
    /// 用户名
    pub username: String,
    /// 邮箱
    pub email: String,
    /// 订阅级别
    pub subscription_level: String,
    /// 订阅过期时间
    pub subscription_expires_at: String,
}

/// API Key信息响应结构
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiKeyInfo {
    /// API Key
    pub api_key: String,
    /// 创建时间
    pub created_at: String,
    /// 过期时间
    pub expires_at: String,
    /// ID
    pub id: u32,
    /// 是否激活
    pub is_active: bool,
    /// Key名称
    pub key_name: String,
    /// 最后使用时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
    /// 用户信息
    pub user: UserInfo,
}

/// 鉴权结果
#[derive(Debug, Clone)]
pub struct AuthResult {
    /// API Key信息
    pub api_key_info: ApiKeyInfo,
    /// 用户信息
    pub user_info: UserInfo,
}

/// 鉴权客户端错误类型
#[derive(Debug)]
pub enum AuthError {
    /// 网络请求失败
    RequestError(String),
    /// HTTP状态码错误
    HttpError(u16),
    /// JSON解析失败
    JsonError(String),
    /// API Key无效
    InvalidApiKey,
    /// API Key已过期
    ExpiredApiKey,
    /// API Key未激活
    InactiveApiKey,
    /// 订阅已过期
    SubscriptionExpired,
    /// 缓存已过期
    CacheExpired,
    /// 数据库错误
    DatabaseError(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::RequestError(msg) => write!(f, "请求失败: {}", msg),
            AuthError::HttpError(status) => write!(f, "HTTP错误: {}", status),
            AuthError::JsonError(msg) => write!(f, "JSON解析失败: {}", msg),
            AuthError::InvalidApiKey => write!(f, "无效的API Key"),
            AuthError::ExpiredApiKey => write!(f, "API Key已过期"),
            AuthError::InactiveApiKey => write!(f, "API Key未激活"),
            AuthError::SubscriptionExpired => write!(f, "用户订阅已过期"),
            AuthError::CacheExpired => write!(f, "缓存已过期"),
            AuthError::DatabaseError(msg) => write!(f, "数据库错误: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

// ============== 对话历史管理相关 ==============

/// 对话信息
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Conversation {
    /// 对话ID
    pub conversation_id: Option<String>,
    /// 用户ID
    pub user_id: i32,
        /// 对话标题
    pub title: Option<String>,
    /// 使用的AI模型
    pub model: String,
    /// 消息数量
    pub message_count: Option<i32>,
    /// 聊天总结
    pub summary: Option<String>,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Local>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Local>,
}

/// 消息信息
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Message {
    /// 消息ID
    pub id: Option<i32>,
    /// 对话ID
    pub conversation_id: String,
    /// 角色 (user/assistant/system)
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 使用的AI模型 (仅assistant角色时)
    pub model: Option<String>,
    /// Token使用情况
    pub usage: Option<TokenUsage>,
    /// 元数据
    pub metadata: Option<serde_json::Value>,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Local>,
}

/// 创建对话请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateConversationRequest {
    /// 会话ID
    pub conversation_id: Option<String>,
    /// 对话标题 (可选)
    pub title: Option<String>,
    /// 系统提示词 (可选)
    pub system_prompt: Option<String>,
    /// 初始消息 (可选)
    pub initial_message: Option<String>,
    /// 使用的AI模型 (可选，默认为 qwen-plus)
    pub model: Option<String>,
}

/// 对话列表查询参数
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListConversationsQuery {
    /// 分页大小，默认20，最大100
    #[serde(default = "default_page_size")]
    pub limit: i64,
    /// 偏移量，默认0
    #[serde(default = "default_offset")]
    pub offset: i64,
    /// 按会话ID筛选（可选）
    pub conversation_id: Option<String>,
    /// 搜索关键词
    pub search: Option<String>,
}

fn default_page_size() -> i64 {
    20
}

fn default_offset() -> i64 {
    0
}

/// 对话列表响应
#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationListResponse {
    /// 对话列表
    pub conversations: Vec<Conversation>,
    /// 总数量
    pub total: i64,
    /// 当前页码
    pub page: i64,
    /// 每页大小
    pub page_size: i64,
    /// 总页数
    pub total_pages: i64,
}

/// 消息列表查询参数
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListMessagesQuery {
    /// 分页大小，默认50，最大100
    #[serde(default = "default_message_page_size")]
    pub limit: i64,
    /// 偏移量，默认0
    #[serde(default = "default_message_offset")]
    pub offset: i64,
    /// 获取指定消息ID之前的消息
    pub before: Option<i32>,
}

fn default_message_page_size() -> i64 {
    50
}

fn default_message_offset() -> i64 {
    0
}

/// 消息列表响应
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageListResponse {
    /// 消息列表
    pub messages: Vec<Message>,
    /// 对话ID
    pub conversation_id: String,
    /// 总数量
    pub total: i64,
    /// 当前页码
    pub page: i64,
    /// 每页大小
    pub page_size: i64,
    /// 总页数
    pub total_pages: i64,
    /// 是否还有更多消息
    pub has_more: bool,
}

/// 创建对话响应
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateConversationResponse {
    /// 对话信息
    pub conversation: Conversation,
    /// 第一个消息（如果有初始消息）
    pub first_message: Option<Message>,
}

/// 更新对话标题请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateConversationTitleRequest {
    /// 新标题
    pub title: String,
}

// ============== 鉴权相关数据结构 ==============

/// 身份认证请求
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AuthRequest {
    /// API 密钥
    pub api_key: String,
}

/// 身份认证响应
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AuthResponse {
    /// 认证是否成功
    pub success: bool,
    /// 用户信息
    pub user: Option<UserInfo>,
    /// 访问令牌
    pub token: Option<String>,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

// ============== 模型相关数据结构 ==============

/// 模型信息
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ModelInfo {
    /// 模型名称
    pub name: String,
    /// 提供商
    pub provider: String,
    /// 描述
    pub description: String,
    /// 是否支持流式
    pub supports_streaming: bool,
    /// 最大token数
    pub max_tokens: u32,
    /// 每1k token费用
    pub cost_per_1k_tokens: f64,
}

/// 模型列表响应
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ModelsResponse {
    /// 模型列表
    pub models: Vec<ModelInfo>,
    /// 总数量
    pub total: i32,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Local>,
}

// ============== 使用统计相关数据结构 ==============

/// 使用统计查询参数
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UsageStatsQuery {
    /// 开始日期 (ISO 8601格式)
    pub from_date: Option<String>,
    /// 结束日期 (ISO 8601格式)
    pub to_date: Option<String>,
    /// 统计周期 (day/week/month)
    pub period: Option<String>,
}

/// 使用统计数据
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UsageStats {
    /// 模型名称
    pub model: String,
    /// 请求次数
    pub requests: u64,
    /// Token使用量
    pub tokens: u64,
    /// 成本（美元）
    pub cost: f64,
}

/// 详细使用统计响应
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UsageStatsResponse {
    /// 统计周期
    pub period: String,
    /// 开始时间
    pub from_date: String,
    /// 结束时间
    pub to_date: String,
    /// 统计数据
    pub stats: DetailedUsageStats,
}

/// 详细使用统计
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct DetailedUsageStats {
    /// 总请求数
    pub total_requests: u64,
    /// 总Token数
    pub total_tokens: u64,
    /// 总成本
    pub total_cost: f64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// 各模型使用情况
    pub model_usage: std::collections::HashMap<String, UsageStats>,
}

// ============== 健康检查相关数据结构 ==============

/// 服务健康状态
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    /// 健康
    Healthy,
    /// 不健康
    Unhealthy,
    /// 未知
    Unknown,
}

/// 健康检查响应
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct HealthCheckResponse {
    /// 整体状态
    pub status: ServiceStatus,
    /// 检查时间
    pub timestamp: chrono::DateTime<chrono::Local>,
    /// 版本号
    pub version: String,
    /// 各服务状态
    pub services: ServicesStatus,
}

/// 服务状态详情
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ServicesStatus {
    /// 数据库状态
    pub database: ServiceStatus,
    /// 缓存状态
    pub cache: ServiceStatus,
    /// AI模型状态
    pub ai_models: std::collections::HashMap<String, ServiceStatus>,
}

/// MCP统计查询参数
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct McpStatsQuery {
    /// 开始日期 (ISO 8601格式)
    pub from_date: Option<String>,
    /// 结束日期 (ISO 8601格式)
    pub to_date: Option<String>,
    /// 分页页码 (从1开始)
    pub page: Option<i32>,
    /// 每页数量
    pub limit: Option<i32>,
    /// 传输类型过滤 (http, sse)
    pub transport_type: Option<String>,
    /// 工具名称过滤
    pub tool_name: Option<String>,
}

/// MCP使用统计汇总
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct McpUsageStats {
    pub total_sessions: i64,
    pub total_tool_calls: i64,
    pub unique_tools_used: i64,
    pub success_rate: f64,
    pub transport_type_counts: serde_json::Value,
}

/// MCP会话信息
#[derive(Debug, Serialize, ToSchema)]
pub struct McpSessionInfo {
    pub session_id: String,
    pub transport_type: String,
    pub tool_calls_count: i64,
    pub created_at: chrono::DateTime<chrono::Local>,
    pub last_activity_at: chrono::DateTime<chrono::Local>,
}

/// MCP工具调用信息
#[derive(Debug, Serialize, ToSchema)]
pub struct McpToolCallInfo {
    pub session_id: Option<String>,
    pub tool_name: String,
    pub status: String,
    pub transport_type: String,
    pub endpoint: String,
    pub execution_time_ms: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Local>,
}

/// 综合使用统计
#[derive(Debug, Serialize, ToSchema)]
pub struct ComprehensiveStats {
    pub mcp: McpUsageStats,
    pub chat: serde_json::Value,
    pub summary: serde_json::Value,
}

/// 模型健康信息
#[derive(Debug, Serialize)]
pub struct ModelHealth {
    /// 模型名称
    pub name: String,
    /// 状态
    pub status: ServiceStatus,
    /// 最后检查时间
    pub last_checked: chrono::DateTime<chrono::Local>,
    /// 响应时间（毫秒）
    pub response_time_ms: Option<u64>,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

// ============== UUID 生成函数 ==============

/// 生成新的会话ID（UUID v4）
pub fn generate_conversation_id() -> String {
    Uuid::new_v4().to_string()
}

/// 验证会话ID格式
pub fn is_valid_conversation_id(id: &str) -> bool {
    id.parse::<Uuid>().is_ok()
}

// ============== MCP 相关数据结构 ==============

/// MCP 服务器信息
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct McpServerInfo {
    /// 服务器名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 协议版本
    pub protocol_version: String,
    /// 服务器标题
    pub title: Option<String>,
    /// 网站地址
    pub website_url: Option<String>,
}

/// MCP 工具信息
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct McpToolInfo {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 工具输入模式 (JSON Schema)
    pub input_schema: serde_json::Value,
    /// 工具类别
    pub category: Option<String>,
}

/// MCP 工具调用请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct McpToolCallRequest {
    /// 工具名称
    pub name: String,
    /// 工具参数
    pub arguments: serde_json::Value,
}

/// MCP 工具调用响应
#[derive(Debug, Serialize, ToSchema)]
pub struct McpToolCallResponse {
    /// 调用结果
    pub content: Vec<McpContent>,
    /// 是否工具出错
    pub isError: Option<bool>,
}

/// MCP 内容块
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct McpContent {
    /// 内容类型
    #[serde(rename = "type")]
    pub content_type: String,
    /// 文本内容
    pub text: Option<String>,
    /// 数据内容
    pub data: Option<serde_json::Value>,
}

/// MCP 初始化请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct McpInitializeRequest {
    /// 协议版本
    pub protocolVersion: String,
    /// 能力信息
    pub capabilities: serde_json::Value,
    /// 客户端信息
    pub clientInfo: McpClientInfo,
}

/// MCP 客户端信息
#[derive(Debug, Deserialize, ToSchema)]
pub struct McpClientInfo {
    /// 客户端名称
    pub name: String,
    /// 客户端版本
    pub version: String,
}

/// MCP 初始化响应
#[derive(Debug, Serialize, ToSchema)]
pub struct McpInitializeResponse {
    /// 协议版本
    pub protocolVersion: String,
    /// 能力信息
    pub capabilities: serde_json::Value,
    /// 服务器信息
    pub serverInfo: McpServerInfo,
}