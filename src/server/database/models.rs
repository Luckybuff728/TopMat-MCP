use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============== 用户相关 ==============

/// 用户模型
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub subscription_level: String,
    pub subscription_expires_at: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API密钥模型
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct ApiKey {
    pub id: i64,
    pub user_id: i64,
    pub api_key: String,
    pub key_name: String,
    pub is_active: bool,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============== 对话相关 ==============

/// 对话模型
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub conversation_id: String,
    pub user_id: i64,
    pub title: Option<String>,
    pub model: String,
    pub message_count: i32,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 消息模型
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct Message {
    pub message_id: i64,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============== 使用统计相关 ==============

/// 使用统计模型
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct UsageStats {
    pub id: i64,
    pub user_id: i64,
    pub model: String,
    pub request_date: String,
    pub request_count: i32,
    pub token_count: i32,
    pub cost_usd: f64,
    pub avg_response_time_ms: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============== 创建请求模型 ==============

/// 创建用户请求
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub subscription_level: Option<String>,
}

/// 创建API密钥请求
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub user_id: i64,
    pub key_name: String,
    pub expires_at: Option<String>,
}

/// 创建对话请求
#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub user_id: i64,
    pub conversation_id: String,
    pub title: Option<String>,
    pub model: String,
}

/// 创建消息请求
#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub metadata: Option<String>,
}

// ============== MCP 相关模型 ==============

/// MCP会话记录
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct McpSession {
    pub id: i64,
    pub session_id: String,
    pub user_id: i64,
    pub transport_type: String,
    pub client_info: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
}

/// MCP工具调用记录
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct McpToolCall {
    pub id: i64,
    pub user_id: i64,
    pub session_id: Option<String>,
    pub tool_name: String,
    pub request_arguments: Option<String>,
    pub response_result: Option<String>,
    pub execution_time_ms: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub transport_type: String,
    pub endpoint: String,
    pub created_at: DateTime<Utc>,
}

/// 创建MCP会话请求
#[derive(Debug, Deserialize)]
pub struct CreateMcpSessionRequest {
    pub session_id: String,
    pub user_id: i64,
    pub transport_type: String,
    pub client_info: Option<String>,
}

/// 创建MCP工具调用请求
#[derive(Debug, Deserialize)]
pub struct CreateMcpToolCallRequest {
    pub user_id: i64,
    pub session_id: Option<String>,
    pub tool_name: String,
    pub request_arguments: Option<String>,
    pub response_result: Option<String>,
    pub execution_time_ms: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub transport_type: String,
    pub endpoint: String,
}

/// MCP工具调用统计
#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolCallStats {
    pub tool_name: String,
    pub total_calls: i64,
    pub success_calls: i64,
    pub error_calls: i64,
    pub avg_execution_time_ms: f64,
    pub last_called_at: Option<DateTime<Utc>>,
}

/// MCP使用统计汇总
#[derive(Debug, Serialize, Deserialize)]
pub struct McpUsageStats {
    pub total_sessions: i64,
    pub total_tool_calls: i64,
    pub unique_tools_used: i64,
    pub success_rate: f64,
    pub avg_session_duration_minutes: f64,
    pub transport_type_counts: std::collections::HashMap<String, i64>,
    pub most_used_tools: Vec<McpToolCallStats>,
}
