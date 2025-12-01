//! MCP使用统计处理器
//!
//! 提供MCP工具调用和会话数据的统计查询API

use axum::{
    extract::{Query, State, Path, Extension},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

use crate::server::{
    database::DatabaseConnection,
    handlers::chat::ServerState,
    middleware::auth::AuthUser,
    models::{ErrorResponse},
};

// 重新定义MCP相关数据结构
#[derive(Debug, Serialize)]
pub struct McpSession {
    pub id: i64,
    pub session_id: String,
    pub user_id: i64,
    pub transport_type: String,
    pub client_info: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub last_activity_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
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
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct McpToolCallStats {
    pub tool_name: String,
    pub total_calls: i64,
    pub success_calls: i64,
    pub error_calls: i64,
    pub avg_execution_time_ms: f64,
    pub last_called_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct McpUsageStats {
    pub total_sessions: i64,
    pub total_tool_calls: i64,
    pub unique_tools_used: i64,
    pub success_rate: f64,
    pub avg_session_duration_minutes: f64,
    pub transport_type_counts: std::collections::HashMap<String, i64>,
    pub most_used_tools: Vec<McpToolCallStats>,
}

/// MCP统计查询参数
#[derive(Debug, Deserialize)]
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

/// 分页响应结构
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

/// 分页信息
#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub page: i32,
    pub limit: i32,
    pub total: i64,
    pub total_pages: i32,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: i32, limit: i32, total: i64) -> Self {
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        Self {
            data,
            pagination: PaginationInfo {
                page,
                limit,
                total,
                total_pages,
            },
        }
    }
}

/// 获取MCP使用统计汇总
pub async fn get_mcp_usage_stats_handler(
    State(state): State<ServerState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<McpStatsQuery>,
) -> Result<Json<McpUsageStats>, ErrorResponse> {
    let user_id = auth_user.user_id;

    // 计算日期范围
    let (from_date, to_date) = calculate_date_range(params.from_date, params.to_date)?;

    // 获取会话统计
    let total_sessions: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM mcp_sessions
        WHERE user_id = ? AND created_at >= ? AND created_at <= ?
        "#
    )
    .bind(user_id)
    .bind(&from_date)
    .bind(&to_date)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询会话统计失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    // 获取工具调用统计
    let total_tool_calls: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM mcp_tool_calls
        WHERE user_id = ? AND created_at >= ? AND created_at <= ?
        "#
    )
    .bind(user_id)
    .bind(&from_date)
    .bind(&to_date)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询工具调用统计失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    // 获取成功调用数量
    let success_calls: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM mcp_tool_calls
        WHERE user_id = ? AND created_at >= ? AND created_at <= ? AND status = 'success'
        "#
    )
    .bind(user_id)
    .bind(&from_date)
    .bind(&to_date)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询成功调用统计失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    // 获取唯一工具数量
    let unique_tools_used: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT tool_name)
        FROM mcp_tool_calls
        WHERE user_id = ? AND created_at >= ? AND created_at <= ? AND status = 'success'
        "#
    )
    .bind(user_id)
    .bind(&from_date)
    .bind(&to_date)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询唯一工具统计失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    // 获取平均会话时长
    let avg_session_duration = sqlx::query_scalar(
        r#"
        SELECT AVG(
            (julianday(last_activity_at) - julianday(created_at)) * 24 * 60
        )
        FROM mcp_sessions
        WHERE user_id = ? AND created_at >= ? AND created_at <= ?
        "#
    )
    .bind(user_id)
    .bind(&from_date)
    .bind(&to_date)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询平均会话时长失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    // 获取传输类型统计
    let transport_type_counts = get_transport_type_counts(&state.database, user_id, &from_date, &to_date).await?;

    // 获取最常用工具
    let most_used_tools = get_most_used_tools(&state.database, user_id, &from_date, &to_date, 10).await?;

    let success_rate = if total_tool_calls > 0 {
        success_calls as f64 / total_tool_calls as f64
    } else {
        0.0
    };

    Ok(Json(McpUsageStats {
        total_sessions,
        total_tool_calls,
        unique_tools_used,
        success_rate,
        avg_session_duration_minutes: avg_session_duration.unwrap_or(0.0),
        transport_type_counts,
        most_used_tools,
    }))
}

/// 获取MCP会话列表
pub async fn get_mcp_sessions_handler(
    State(state): State<ServerState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<McpStatsQuery>,
) -> Result<Json<PaginatedResponse<McpSession>>, ErrorResponse> {
    let user_id = auth_user.user_id;
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * limit;

    let (from_date, to_date) = calculate_date_range(params.from_date, params.to_date)?;

    // 构建查询条件
    let mut where_clause = "WHERE user_id = ? AND created_at >= ? AND created_at <= ?".to_string();
    if let Some(ref transport) = params.transport_type {
        where_clause.push_str(&format!(" AND transport_type = '{}'", transport));
    }

    // 获取总数
    let total_query = format!(
        "SELECT COUNT(*) FROM mcp_sessions {}",
        where_clause
    );
    let total: i64 = sqlx::query_scalar(&total_query)
        .bind(user_id)
        .bind(&from_date)
        .bind(&to_date)
        .fetch_one(state.database.pool())
        .await
        .map_err(|e| ErrorResponse {
            error: "database_error".to_string(),
            message: "查询会话总数失败".to_string(),
            details: Some(serde_json::json!({ "error": e.to_string() })),
            timestamp: chrono::Utc::now(),
        })?;

    // 获取会话列表
    let sessions_query = format!(
        "SELECT * FROM mcp_sessions {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let sessions: Vec<McpSession> = sqlx::query_as(&sessions_query)
        .bind(user_id)
        .bind(&from_date)
        .bind(&to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(state.database.pool())
        .await
        .map_err(|e| ErrorResponse {
            error: "database_error".to_string(),
            message: "查询会话列表失败".to_string(),
            details: Some(serde_json::json!({ "error": e.to_string() })),
            timestamp: chrono::Utc::now(),
        })?;

    Ok(Json(PaginatedResponse::new(sessions, page, limit, total)))
}

/// 获取MCP工具调用记录
pub async fn get_mcp_tool_calls_handler(
    State(state): State<ServerState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<McpStatsQuery>,
) -> Result<Json<PaginatedResponse<McpToolCall>>, ErrorResponse> {
    let user_id = auth_user.user_id;
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * limit;

    let (from_date, to_date) = calculate_date_range(params.from_date, params.to_date)?;

    // 构建查询条件
    let mut where_clause = "WHERE user_id = ? AND created_at >= ? AND created_at <= ?".to_string();
    if let Some(ref transport) = params.transport_type {
        where_clause.push_str(&format!(" AND transport_type = '{}'", transport));
    }
    if let Some(ref tool_name) = params.tool_name {
        where_clause.push_str(&format!(" AND tool_name = '{}'", tool_name));
    }

    // 获取总数
    let total_query = format!(
        "SELECT COUNT(*) FROM mcp_tool_calls {}",
        where_clause
    );
    let total: i64 = sqlx::query_scalar(&total_query)
        .bind(user_id)
        .bind(&from_date)
        .bind(&to_date)
        .fetch_one(state.database.pool())
        .await
        .map_err(|e| ErrorResponse {
            error: "database_error".to_string(),
            message: "查询工具调用总数失败".to_string(),
            details: Some(serde_json::json!({ "error": e.to_string() })),
            timestamp: chrono::Utc::now(),
        })?;

    // 获取工具调用记录
    let calls_query = format!(
        "SELECT * FROM mcp_tool_calls {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let tool_calls: Vec<McpToolCall> = sqlx::query_as(&calls_query)
        .bind(user_id)
        .bind(&from_date)
        .bind(&to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(state.database.pool())
        .await
        .map_err(|e| ErrorResponse {
            error: "database_error".to_string(),
            message: "查询工具调用记录失败".to_string(),
            details: Some(serde_json::json!({ "error": e.to_string() })),
            timestamp: chrono::Utc::now(),
        })?;

    Ok(Json(PaginatedResponse::new(tool_calls, page, limit, total)))
}

/// 获取综合使用统计
pub async fn get_comprehensive_stats_handler(
    State(state): State<ServerState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<McpStatsQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let user_id = auth_user.user_id;
    let (from_date, to_date) = calculate_date_range(params.from_date, params.to_date)?;

    // 获取聊天统计
    let chat_stats = get_chat_statistics(&state.database, user_id, &from_date, &to_date).await?;

    // 获取MCP统计
    let mcp_stats = get_mcp_usage_stats_handler(State(state.clone()), Extension(auth_user.clone()), Query(params)).await?;

    Ok(Json(serde_json::json!({
        "period": {
            "from_date": from_date,
            "to_date": to_date
        },
        "chat": chat_stats,
        "mcp": mcp_stats.0,
        "summary": {
            "total_requests": chat_stats.total_requests + mcp_stats.0.total_tool_calls,
            "active_sessions": chat_stats.active_conversations + mcp_stats.0.total_sessions,
            "data_points": chat_stats.total_messages + mcp_stats.0.total_tool_calls
        }
    })))
}

// 辅助函数

/// 计算日期范围
fn calculate_date_range(
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<(String, String), ErrorResponse> {
    let to_dt = if let Some(to) = to_date {
        DateTime::parse_from_rfc3339(&to)
            .map_err(|_| ErrorResponse {
                error: "invalid_date_format".to_string(),
                message: "结束日期格式无效".to_string(),
                details: None,
                timestamp: chrono::Utc::now(),
            })?
            .with_timezone(&Utc)
    } else {
        chrono::Utc::now()
    };

    let from_dt = if let Some(from) = from_date {
        DateTime::parse_from_rfc3339(&from)
            .map_err(|_| ErrorResponse {
                error: "invalid_date_format".to_string(),
                message: "开始日期格式无效".to_string(),
                details: None,
                timestamp: chrono::Utc::now(),
            })?
            .with_timezone(&Utc)
    } else {
        to_dt - Duration::days(30) // 默认30天
    };

    Ok((
        from_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        to_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
    ))
}

/// 获取传输类型统计
async fn get_transport_type_counts(
    db: &DatabaseConnection,
    user_id: i64,
    from_date: &str,
    to_date: &str,
) -> Result<HashMap<String, i64>, ErrorResponse> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT transport_type, COUNT(*) FROM mcp_tool_calls
         WHERE user_id = ? AND created_at >= ? AND created_at <= ?
         GROUP BY transport_type"
    )
    .bind(user_id)
    .bind(from_date)
    .bind(to_date)
    .fetch_all(db.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询传输类型统计失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    Ok(rows.into_iter().collect())
}

/// 获取最常用工具
async fn get_most_used_tools(
    db: &DatabaseConnection,
    user_id: i64,
    from_date: &str,
    to_date: &str,
    limit: i32,
) -> Result<Vec<McpToolCallStats>, ErrorResponse> {
    let rows = sqlx::query(
        "SELECT
            tool_name,
            COUNT(*) as total_calls,
            COUNT(CASE WHEN status = 'success' THEN 1 END) as success_calls,
            COUNT(CASE WHEN status = 'error' THEN 1 END) as error_calls,
            AVG(execution_time_ms) as avg_execution_time_ms,
            MAX(created_at) as last_called_at
        FROM mcp_tool_calls
        WHERE user_id = ? AND created_at >= ? AND created_at <= ?
        GROUP BY tool_name
        ORDER BY total_calls DESC
        LIMIT ?"
    )
    .bind(user_id)
    .bind(from_date)
    .bind(to_date)
    .bind(limit)
    .fetch_all(db.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询最常用工具失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    let mut tools = Vec::new();
    for row in rows {
        let tool_name: String = row.get("tool_name");
        let total_calls: i64 = row.get("total_calls");
        let success_calls: i64 = row.get("success_calls");
        let error_calls: i64 = row.get("error_calls");
        let avg_execution_time: Option<f64> = row.get("avg_execution_time_ms");
        let last_called_at: Option<String> = row.get("last_called_at");

        let last_called_dt = if let Some(date_str) = last_called_at {
            chrono::DateTime::parse_from_rfc3339(&date_str)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        } else {
            None
        };

        tools.push(McpToolCallStats {
            tool_name,
            total_calls,
            success_calls,
            error_calls,
            avg_execution_time_ms: avg_execution_time.unwrap_or(0.0),
            last_called_at: last_called_dt,
        });
    }

    Ok(tools)
}

/// 获取聊天统计数据
async fn get_chat_statistics(
    db: &DatabaseConnection,
    user_id: i64,
    from_date: &str,
    to_date: &str,
) -> Result<serde_json::Value, ErrorResponse> {
    let total_requests: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM conversations
         WHERE user_id = ? AND created_at >= ? AND created_at <= ?"
    )
    .bind(user_id)
    .bind(from_date)
    .bind(to_date)
    .fetch_one(db.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询聊天统计失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    let active_conversations: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM conversations
         WHERE user_id = ? AND updated_at >= ?"
    )
    .bind(user_id)
    .bind(from_date)
    .fetch_one(db.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询活跃对话失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    let total_messages: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages
         WHERE conversation_id IN (
             SELECT conversation_id FROM conversations
             WHERE user_id = ? AND created_at >= ? AND created_at <= ?
         )"
    )
    .bind(user_id)
    .bind(from_date)
    .bind(to_date)
    .fetch_one(db.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询消息统计失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    Ok(serde_json::json!({
        "total_requests": total_requests,
        "active_conversations": active_conversations,
        "total_messages": total_messages
    }))
}