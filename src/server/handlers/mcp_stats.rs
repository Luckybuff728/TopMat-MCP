//! MCP使用统计处理器 - 简化版本
//!
//! 提供MCP工具调用和会话数据的统计查询API

use axum::{
    extract::{Query, State, Extension},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use sqlx::Row;

use crate::server::{
    database::DatabaseConnection,
    handlers::chat::ServerState,
    middleware::auth::AuthUser,
    models::ErrorResponse,
};

/// MCP统计查询参数
#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Clone)]
pub struct McpUsageStats {
    pub total_sessions: i64,
    pub total_tool_calls: i64,
    pub unique_tools_used: i64,
    pub success_rate: f64,
    pub transport_type_counts: serde_json::Value,
}

/// MCP会话信息
#[derive(Debug, Serialize)]
pub struct McpSessionInfo {
    pub session_id: String,
    pub transport_type: String,
    pub tool_calls_count: i64,
    pub created_at: String,
    pub last_activity_at: String,
}

/// MCP工具调用信息
#[derive(Debug, Serialize)]
pub struct McpToolCallInfo {
    pub session_id: Option<String>,
    pub tool_name: String,
    pub status: String,
    pub transport_type: String,
    pub endpoint: String,
    pub execution_time_ms: Option<i32>,
    pub created_at: String,
}

/// 综合使用统计
#[derive(Debug, Serialize)]
pub struct ComprehensiveStats {
    pub mcp: McpUsageStats,
    pub chat: serde_json::Value,
    pub summary: serde_json::Value,
}

/// 获取MCP使用统计汇总
pub async fn get_mcp_usage_stats_handler(
    State(state): State<ServerState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<McpStatsQuery>,
) -> Result<Json<McpUsageStats>, ErrorResponse> {
    let user_id = auth_user.user_id as i64;

    // 获取会话统计
    let total_sessions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mcp_sessions WHERE user_id = ?"
    )
    .bind(user_id)
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
        "SELECT COUNT(*) FROM mcp_tool_calls WHERE user_id = ?"
    )
    .bind(user_id)
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
        "SELECT COUNT(*) FROM mcp_tool_calls WHERE user_id = ? AND status = 'success'"
    )
    .bind(user_id)
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
        "SELECT COUNT(DISTINCT tool_name) FROM mcp_tool_calls WHERE user_id = ? AND status = 'success'"
    )
    .bind(user_id)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询唯一工具统计失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    // 获取传输类型统计
    let transport_rows = sqlx::query(
        "SELECT transport_type, COUNT(*) as count FROM mcp_tool_calls WHERE user_id = ? GROUP BY transport_type"
    )
    .bind(user_id)
    .fetch_all(state.database.pool())
    .await
    .map_err(|e| ErrorResponse {
        error: "database_error".to_string(),
        message: "查询传输类型统计失败".to_string(),
        details: Some(serde_json::json!({ "error": e.to_string() })),
        timestamp: chrono::Utc::now(),
    })?;

    let mut transport_counts = serde_json::Map::new();
    for row in transport_rows {
        let transport_type: String = row.try_get("transport_type").unwrap_or_default();
        let count: i64 = row.try_get("count").unwrap_or(0);
        transport_counts.insert(transport_type, serde_json::Value::Number(count.into()));
    }

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
        transport_type_counts: serde_json::Value::Object(transport_counts),
    }))
}

/// 获取MCP会话列表
pub async fn get_mcp_sessions_handler(
    State(state): State<ServerState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<McpStatsQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let user_id = auth_user.user_id as i64;
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * limit;

    // 构建查询条件
    let mut where_clause = "WHERE user_id = ?".to_string();
    if let Some(ref transport) = params.transport_type {
        where_clause.push_str(&format!(" AND transport_type = '{}'", transport));
    }

    // 获取总数
    let total_query = format!("SELECT COUNT(*) FROM mcp_sessions {}", where_clause);
    let total: i64 = sqlx::query_scalar(&total_query)
        .bind(user_id)
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
        "SELECT session_id, transport_type, created_at, last_activity_at FROM mcp_sessions {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let sessions = sqlx::query(&sessions_query)
        .bind(user_id)
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

    let mut session_list: Vec<McpSessionInfo> = Vec::new();
    for row in sessions {
        let session_id: String = row.try_get("session_id").unwrap_or_default();
        let transport_type: String = row.try_get("transport_type").unwrap_or_default();
        let created_at: String = row.try_get("created_at").unwrap_or_default();
        let last_activity_at: String = row.try_get("last_activity_at").unwrap_or_default();

        // 获取每个会话的工具调用数量
        let tool_calls_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mcp_tool_calls WHERE session_id = ?"
        )
        .bind(&session_id)
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0);

        session_list.push(McpSessionInfo {
            session_id,
            transport_type,
            tool_calls_count,
            created_at,
            last_activity_at,
        });
    }

    Ok(Json(serde_json::json!({
        "data": session_list,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "total_pages": ((total as f64) / (limit as f64)).ceil() as i32
        }
    })))
}

/// 获取MCP工具调用记录
pub async fn get_mcp_tool_calls_handler(
    State(state): State<ServerState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<McpStatsQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let user_id = auth_user.user_id as i64;
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * limit;

    // 构建查询条件
    let mut where_clause = "WHERE user_id = ?".to_string();
    if let Some(ref transport) = params.transport_type {
        where_clause.push_str(&format!(" AND transport_type = '{}'", transport));
    }
    if let Some(ref tool_name) = params.tool_name {
        where_clause.push_str(&format!(" AND tool_name = '{}'", tool_name));
    }

    // 获取总数
    let total_query = format!("SELECT COUNT(*) FROM mcp_tool_calls {}", where_clause);
    let total: i64 = sqlx::query_scalar(&total_query)
        .bind(user_id)
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
        "SELECT session_id, tool_name, status, transport_type, endpoint, execution_time_ms, created_at FROM mcp_tool_calls {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let tool_calls = sqlx::query(&calls_query)
        .bind(user_id)
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

    let mut call_list: Vec<McpToolCallInfo> = Vec::new();
    for row in tool_calls {
        call_list.push(McpToolCallInfo {
            session_id: row.try_get("session_id").ok(),
            tool_name: row.try_get("tool_name").unwrap_or_default(),
            status: row.try_get("status").unwrap_or_default(),
            transport_type: row.try_get("transport_type").unwrap_or_default(),
            endpoint: row.try_get("endpoint").unwrap_or_default(),
            execution_time_ms: row.try_get("execution_time_ms").ok(),
            created_at: row.try_get("created_at").unwrap_or_default(),
        });
    }

    Ok(Json(serde_json::json!({
        "data": call_list,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "total_pages": ((total as f64) / (limit as f64)).ceil() as i32
        }
    })))
}

/// 获取综合使用统计
pub async fn get_comprehensive_stats_handler(
    State(state): State<ServerState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<McpStatsQuery>,
) -> Result<Json<ComprehensiveStats>, ErrorResponse> {
    let user_id = auth_user.user_id as i64;

    // 获取MCP统计
    let mcp_stats = get_mcp_usage_stats_handler(
        State(state.clone()),
        Extension(auth_user.clone()),
        Query(params.clone()),
    ).await?;
    let mcp_data = mcp_stats.0;

    // 获取聊天统计（简化版本）
    let total_conversations: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM conversations WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_one(state.database.pool())
    .await
    .unwrap_or(0);

    let total_messages: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE conversation_id IN (SELECT conversation_id FROM conversations WHERE user_id = ?)"
    )
    .bind(user_id)
    .fetch_one(state.database.pool())
    .await
    .unwrap_or(0);

    Ok(Json(ComprehensiveStats {
        mcp: mcp_data.clone(),
        chat: serde_json::json!({
            "total_conversations": total_conversations,
            "total_messages": total_messages
        }),
        summary: serde_json::json!({
            "total_requests": total_conversations + mcp_data.total_tool_calls,
            "active_sessions": total_conversations + mcp_data.total_sessions,
            "data_points": total_messages + mcp_data.total_tool_calls
        })
    }))
}