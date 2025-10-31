use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

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