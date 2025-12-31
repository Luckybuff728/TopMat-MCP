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
use utoipa::path;
use serde_json::json;

use crate::server::{
    database::DatabaseConnection,
    handlers::chat::ServerState,
    middleware::auth::AuthUser,
    models::{ErrorResponse, McpStatsQuery, McpUsageStats, McpSessionInfo, McpToolCallInfo, ComprehensiveStats},
};

/// 获取MCP使用统计汇总
#[utoipa::path(
    get,
    path = "/usage/mcp/stats",
    tag = "usage",
    summary = "获取MCP使用统计",
    description = "获取当前用户的MCP工具使用统计信息，包括会话数、工具调用数、成功率等。",
    params(
        ("from_date" = Option<String>, Query, description = "开始日期 (ISO 8601格式)"),
        ("to_date" = Option<String>, Query, description = "结束日期 (ISO 8601格式)"),
        ("page" = Option<i32>, Query, description = "分页页码 (从1开始)"),
        ("limit" = Option<i32>, Query, description = "每页数量"),
        ("transport_type" = Option<String>, Query, description = "传输类型过滤 (http, sse)"),
        ("tool_name" = Option<String>, Query, description = "工具名称过滤")
    ),
    responses(
        (status = 200, description = "请求成功", body = McpUsageStats,
         example = json!({
             "total_sessions": 25,
             "total_tool_calls": 150,
             "unique_tools_used": 8,
             "success_rate": 0.95,
             "transport_type_counts": {
                 "http": 80,
                 "sse": 70
             }
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
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
        timestamp: chrono::Local::now(),
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
        timestamp: chrono::Local::now(),
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
        timestamp: chrono::Local::now(),
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
        timestamp: chrono::Local::now(),
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
        timestamp: chrono::Local::now(),
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
#[utoipa::path(
    get,
    path = "/usage/mcp/sessions",
    tag = "usage",
    summary = "获取MCP会话列表",
    description = "获取当前用户的MCP会话历史，支持分页和过滤。",
    params(
        ("from_date" = Option<String>, Query, description = "开始日期 (ISO 8601格式)"),
        ("to_date" = Option<String>, Query, description = "结束日期 (ISO 8601格式)"),
        ("page" = Option<i32>, Query, description = "分页页码 (从1开始)"),
        ("limit" = Option<i32>, Query, description = "每页数量"),
        ("transport_type" = Option<String>, Query, description = "传输类型过滤 (http, sse)")
    ),
    responses(
        (status = 200, description = "请求成功", body = serde_json::Value,
         example = json!({
             "sessions": [
                 {
                     "session_id": "sess_123456",
                     "transport_type": "http",
                     "tool_calls_count": 5,
                     "created_at": "2024-01-01T12:00:00Z",
                     "last_activity_at": "2024-01-01T12:30:00Z"
                 }
             ],
             "total": 25,
             "page": 1,
             "page_size": 20,
             "total_pages": 2
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
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
            timestamp: chrono::Local::now(),
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
            timestamp: chrono::Local::now(),
        })?;

    let mut session_list: Vec<McpSessionInfo> = Vec::new();
    for row in sessions {
        let session_id: String = row.try_get("session_id").unwrap_or_default();
        let transport_type: String = row.try_get("transport_type").unwrap_or_default();
        let created_at: chrono::DateTime<chrono::Local> = row.try_get("created_at").unwrap_or_else(|_| chrono::Local::now());
        let last_activity_at: chrono::DateTime<chrono::Local> = row.try_get("last_activity_at").unwrap_or_else(|_| chrono::Local::now());

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
#[utoipa::path(
    get,
    path = "/usage/mcp/tool-calls",
    tag = "usage",
    summary = "获取MCP工具调用记录",
    description = "获取当前用户的MCP工具调用历史记录，支持分页和过滤。",
    params(
        ("from_date" = Option<String>, Query, description = "开始日期 (ISO 8601格式)"),
        ("to_date" = Option<String>, Query, description = "结束日期 (ISO 8601格式)"),
        ("page" = Option<i32>, Query, description = "分页页码 (从1开始)"),
        ("limit" = Option<i32>, Query, description = "每页数量"),
        ("transport_type" = Option<String>, Query, description = "传输类型过滤 (http, sse)"),
        ("tool_name" = Option<String>, Query, description = "工具名称过滤")
    ),
    responses(
        (status = 200, description = "请求成功", body = serde_json::Value,
         example = json!({
             "tool_calls": [
                 {
                     "session_id": "sess_123456",
                     "tool_name": "calpha_mesh_simulation",
                     "status": "success",
                     "transport_type": "http",
                     "endpoint": "/mcp",
                     "execution_time_ms": 1250,
                     "created_at": "2024-01-01T12:15:00Z"
                 }
             ],
             "total": 150,
             "page": 1,
             "page_size": 20,
             "total_pages": 8
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
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
            timestamp: chrono::Local::now(),
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
            timestamp: chrono::Local::now(),
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
            created_at: row.try_get("created_at").unwrap_or_else(|_| chrono::Local::now()),
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
#[utoipa::path(
    get,
    path = "/usage/comprehensive",
    tag = "usage",
    summary = "获取综合使用统计",
    description = "获取包含聊天和MCP工具使用的综合统计信息。",
    params(
        ("from_date" = Option<String>, Query, description = "开始日期 (ISO 8601格式)"),
        ("to_date" = Option<String>, Query, description = "结束日期 (ISO 8601格式)"),
        ("period" = Option<String>, Query, description = "统计周期 (day/week/month)")
    ),
    responses(
        (status = 200, description = "请求成功", body = ComprehensiveStats,
         example = json!({
             "mcp": {
                 "total_sessions": 25,
                 "total_tool_calls": 150,
                 "unique_tools_used": 8,
                 "success_rate": 0.95,
                 "transport_type_counts": {
                     "http": 80,
                     "sse": 70
                 }
             },
             "chat": {
                 "total_requests": 300,
                 "total_tokens": 15000,
                 "avg_response_time_ms": 1250.0
             },
             "summary": {
                 "total_api_calls": 450,
                 "most_active_day": "2024-01-01",
                 "cost_estimate": 2.50
             }
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
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