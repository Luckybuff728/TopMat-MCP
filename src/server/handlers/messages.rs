use axum::{
    extract::{Extension, Path, Query, State},
    response::Json,
};
use sqlx::Row;
use tracing::{error, info, warn};

use super::chat::ServerState;
use crate::server::middleware::AuthUser;
use crate::server::models::*;

/// 获取对话的消息历史
#[utoipa::path(
    get,
    path = "/v1/conversations/{id}/messages",
    tag = "conversations",
    summary = "获取消息列表",
    description = "获取指定对话的消息历史，支持分页查询。",
    params(
        ("id" = String, Path, description = "对话ID"),
        ("limit" = i64, Query, description = "分页大小，默认50，最大100"),
        ("offset" = i64, Query, description = "偏移量，默认0"),
        ("before" = Option<i32>, Query, description = "获取指定消息ID之前的消息")
    ),
    responses(
        (status = 200, description = "请求成功", body = MessageListResponse,
         example = json!({
             "messages": [
                 {
                     "id": 1,
                     "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
                     "role": "user",
                     "content": "你好，请介绍一下你自己",
                     "model": null,
                     "usage": null,
                     "metadata": null,
                     "created_at": "2024-01-01T12:00:00Z"
                 },
                 {
                     "id": 2,
                     "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
                     "role": "assistant",
                     "content": "你好！我是一个AI助手，很高兴为您服务。",
                     "model": "qwen-plus",
                     "usage": {
                         "prompt_tokens": 20,
                         "completion_tokens": 15,
                         "total_tokens": 35
                     },
                     "metadata": {"response_time_ms": 1500},
                     "created_at": "2024-01-01T12:00:01Z"
                 }
             ],
             "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
             "total": 2,
             "page": 1,
             "page_size": 50,
             "total_pages": 1,
             "has_more": false
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 404, description = "对话不存在或无权访问", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn list_messages_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Path(conversation_id): Path<String>,
    Query(params): Query<ListMessagesQuery>,
) -> Result<Json<MessageListResponse>, ErrorResponse> {
    info!(
        "获取消息历史: conversation_id={}, limit={}, offset={}, user_id={}",
        conversation_id, params.limit, params.offset, auth_user.user_id
    );

    // 验证用户是否有权限访问该对话
    let conversation_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM conversations WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(&conversation_id)
    .bind(auth_user.user_id as i64)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| {
        error!("验证对话权限失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "验证对话权限失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Local::now(),
        }
    })?;

    if !conversation_exists {
        warn!(
            "用户 {} 尝试访问无权限的对话 {}",
            auth_user.user_id, conversation_id
        );
        return Err(ErrorResponse {
            error: "conversation_not_found".to_string(),
            message: "对话不存在或无权访问".to_string(),
            details: Some(serde_json::json!({
                "conversation_id": conversation_id
            })),
            timestamp: chrono::Local::now(),
        });
    }

    // 构建基本查询SQL - 使用PostgreSQL参数占位符
    let base_sql = "SELECT message_id, conversation_id, role, content, reasoning_content, tool_calls, tool_call_id, model, prompt_tokens, completion_tokens, total_tokens, metadata, created_at \
         FROM messages WHERE conversation_id = $1";

    let (sql, _param_offset) = if let Some(_before_id) = params.before {
        (
            format!(
                "{} AND message_id < $2 ORDER BY created_at ASC, message_id ASC LIMIT $3 OFFSET $4",
                base_sql
            ),
            2,
        )
    } else {
        (
            format!(
                "{} ORDER BY created_at ASC, message_id ASC LIMIT $2 OFFSET $3",
                base_sql
            ),
            1,
        )
    };

    // 执行查询
    let mut query = sqlx::query(&sql);
    query = query.bind(&conversation_id);

    if let Some(before_id) = params.before {
        query = query.bind(before_id as i64);
    }

    query = query.bind(params.limit);
    query = query.bind(params.offset);

    let rows = query.fetch_all(state.database.pool()).await.map_err(|e| {
        error!("查询消息历史失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "查询消息历史失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Local::now(),
        }
    })?;

    // 转换为API模型
    let messages: Vec<Message> = rows
        .into_iter()
        .map(|row| {
            let metadata_str: Option<String> =
                row.try_get::<Option<String>, _>("metadata").ok().flatten();
            let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

            Message {
                id: Some(row.try_get::<i64, _>("message_id").unwrap_or(0) as i32),
                conversation_id: row
                    .try_get::<String, _>("conversation_id")
                    .unwrap_or_default(),
                role: row.try_get::<String, _>("role").unwrap_or_default(),
                content: row.try_get::<Option<String>, _>("content").ok().flatten(),
                reasoning_content: row
                    .try_get::<Option<String>, _>("reasoning_content")
                    .ok()
                    .flatten(),
                tool_calls: row
                    .try_get::<Option<String>, _>("tool_calls")
                    .ok()
                    .flatten()
                    .and_then(|s| serde_json::from_str(&s).ok()),
                tool_call_id: row
                    .try_get::<Option<String>, _>("tool_call_id")
                    .ok()
                    .flatten(),
                model: row.try_get::<Option<String>, _>("model").ok().flatten(),
                usage: if row
                    .try_get::<Option<i32>, _>("prompt_tokens")
                    .ok()
                    .flatten()
                    .is_some()
                {
                    Some(TokenUsage {
                        prompt_tokens: row
                            .try_get::<Option<i32>, _>("prompt_tokens")
                            .ok()
                            .flatten()
                            .unwrap_or(0) as u32,
                        completion_tokens: row
                            .try_get::<Option<i32>, _>("completion_tokens")
                            .ok()
                            .flatten()
                            .unwrap_or(0) as u32,
                        total_tokens: row
                            .try_get::<Option<i32>, _>("total_tokens")
                            .ok()
                            .flatten()
                            .unwrap_or(0) as u32,
                    })
                } else {
                    None
                },
                metadata,
                created_at: row
                    .try_get::<chrono::DateTime<chrono::Local>, _>("created_at")
                    .unwrap_or_else(|_| chrono::Local::now()),
            }
        })
        .collect();

    // 获取总数
    let total_sql = "SELECT COUNT(*) FROM messages WHERE conversation_id = $1";
    let mut total_query = sqlx::query(total_sql);
    total_query = total_query.bind(&conversation_id);

    let total_row = total_query
        .fetch_one(state.database.pool())
        .await
        .map_err(|e| {
            error!("查询消息总数失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "查询消息总数失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Local::now(),
            }
        })?;

    let total: i64 = total_row.try_get::<i64, _>(0).unwrap_or(0);

    let page_size = params.limit.clamp(1, 100);
    let page = (params.offset / page_size) + 1;
    let total_pages = (total + page_size - 1) / page_size;
    let has_more = (params.offset + page_size) < total;

    Ok(Json(MessageListResponse {
        messages,
        conversation_id: conversation_id.clone(),
        total,
        page,
        page_size,
        total_pages,
        has_more,
    }))
}

/// 获取单个消息详情
#[utoipa::path(
    get,
    path = "/v1/conversations/{id}/messages/{message_id}",
    tag = "conversations",
    summary = "获取消息详情",
    description = "获取指定对话中单个消息的详细信息。",
    params(
        ("id" = String, Path, description = "对话ID"),
        ("message_id" = i32, Path, description = "消息ID")
    ),
    responses(
        (status = 200, description = "请求成功", body = Message,
         example = json!({
             "id": 2,
             "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
             "role": "assistant",
             "content": "你好！我是一个AI助手，很高兴为您服务。",
             "model": "qwen-plus",
             "usage": {
                 "prompt_tokens": 20,
                 "completion_tokens": 15,
                 "total_tokens": 35
             },
             "metadata": {"response_time_ms": 1500},
             "created_at": "2024-01-01T12:00:01Z"
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 404, description = "消息不存在或无权访问", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn get_message_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Path((conversation_id, message_id)): Path<(String, i32)>,
) -> Result<Json<Message>, ErrorResponse> {
    info!(
        "获取消息详情: conversation_id={}, message_id={}, user_id={}",
        conversation_id, message_id, auth_user.user_id
    );

    // 验证用户是否有权限访问该对话
    let conversation_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM conversations WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(&conversation_id)
    .bind(auth_user.user_id as i64)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| {
        error!("验证对话权限失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "验证对话权限失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Local::now(),
        }
    })?;

    if !conversation_exists {
        warn!(
            "用户 {} 尝试访问无权限的对话 {} 的消息 {}",
            auth_user.user_id, conversation_id, message_id
        );
        return Err(ErrorResponse {
            error: "conversation_not_found".to_string(),
            message: "对话不存在或无权访问".to_string(),
            details: Some(serde_json::json!({
                "conversation_id": conversation_id,
                "message_id": message_id
            })),
            timestamp: chrono::Local::now(),
        });
    }

    let sql = "SELECT message_id, conversation_id, role, content, reasoning_content, tool_calls, tool_call_id, model, prompt_tokens, completion_tokens, total_tokens, metadata, created_at \
              FROM messages WHERE message_id = $1 AND conversation_id = $2";

    let row = sqlx::query(sql)
        .bind(message_id as i64)
        .bind(&conversation_id)
        .fetch_one(state.database.pool())
        .await
        .map_err(|e| {
            error!("查询消息详情失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "查询消息详情失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Local::now(),
            }
        })?;

    let metadata_str: Option<String> = row.try_get::<Option<String>, _>("metadata").ok().flatten();
    let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

    let message = Message {
        id: Some(row.try_get::<i64, _>("message_id").unwrap_or(0) as i32),
        conversation_id: row
            .try_get::<String, _>("conversation_id")
            .unwrap_or_default(),
        role: row.try_get::<String, _>("role").unwrap_or_default(),
        content: row.try_get::<Option<String>, _>("content").ok().flatten(),
        reasoning_content: row
            .try_get::<Option<String>, _>("reasoning_content")
            .ok()
            .flatten(),
        tool_calls: row
            .try_get::<Option<String>, _>("tool_calls")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok()),
        tool_call_id: row
            .try_get::<Option<String>, _>("tool_call_id")
            .ok()
            .flatten(),
        model: row.try_get::<Option<String>, _>("model").ok().flatten(),
        usage: if row
            .try_get::<Option<i32>, _>("prompt_tokens")
            .ok()
            .flatten()
            .is_some()
        {
            Some(TokenUsage {
                prompt_tokens: row
                    .try_get::<Option<i32>, _>("prompt_tokens")
                    .ok()
                    .flatten()
                    .unwrap_or(0) as u32,
                completion_tokens: row
                    .try_get::<Option<i32>, _>("completion_tokens")
                    .ok()
                    .flatten()
                    .unwrap_or(0) as u32,
                total_tokens: row
                    .try_get::<Option<i32>, _>("total_tokens")
                    .ok()
                    .flatten()
                    .unwrap_or(0) as u32,
            })
        } else {
            None
        },
        metadata,
        created_at: row
            .try_get::<chrono::DateTime<chrono::Local>, _>("created_at")
            .unwrap_or_else(|_| chrono::Local::now()),
    };

    Ok(Json(message))
}

/// 删除消息
#[utoipa::path(
    delete,
    path = "/v1/conversations/{id}/messages/{message_id}",
    tag = "conversations",
    summary = "删除消息",
    description = "删除指定对话中的单个消息。",
    params(
        ("id" = String, Path, description = "对话ID"),
        ("message_id" = i32, Path, description = "消息ID")
    ),
    responses(
        (status = 200, description = "删除成功", body = serde_json::Value,
         example = json!({
             "success": true,
             "message": "消息删除成功",
             "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
             "message_id": 2,
             "timestamp": "2024-01-01T12:00:00Z"
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 404, description = "消息不存在或无权访问", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn delete_message_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Path((conversation_id, message_id)): Path<(String, i32)>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    info!(
        "删除消息: conversation_id={}, message_id={}, user_id={}",
        conversation_id, message_id, auth_user.user_id
    );

    // 验证用户是否有权限访问该对话
    let conversation_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM conversations WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(&conversation_id)
    .bind(auth_user.user_id as i64)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| {
        error!("验证对话权限失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "验证对话权限失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Local::now(),
        }
    })?;

    if !conversation_exists {
        warn!(
            "用户 {} 尝试删除无权限对话 {} 的消息 {}",
            auth_user.user_id, conversation_id, message_id
        );
        return Err(ErrorResponse {
            error: "conversation_not_found".to_string(),
            message: "对话不存在或无权访问".to_string(),
            details: Some(serde_json::json!({
                "conversation_id": conversation_id,
                "message_id": message_id
            })),
            timestamp: chrono::Local::now(),
        });
    }

    let sql = "DELETE FROM messages WHERE message_id = $1 AND conversation_id = $2";

    let result = sqlx::query(sql)
        .bind(message_id as i64)
        .bind(&conversation_id)
        .execute(state.database.pool())
        .await
        .map_err(|e| {
            error!("删除消息失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "删除消息失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Local::now(),
            }
        })?;

    let rows_affected = result.rows_affected();

    if rows_affected == 0 {
        return Err(ErrorResponse {
            error: "not_found".to_string(),
            message: "消息不存在".to_string(),
            details: Some(serde_json::json!({
                "message_id": message_id,
                "conversation_id": conversation_id
            })),
            timestamp: chrono::Local::now(),
        });
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "消息删除成功",
        "message_id": message_id,
        "conversation_id": conversation_id,
        "timestamp": chrono::Local::now()
    })))
}

// /// 添加新消息到对话
// pub async fn add_message_handler(
//     Extension(auth_user): Extension<AuthUser>,
//     State(state): State<ServerState>,
//     Path(conversation_id): Path<String>,
//     Json(request): Json<Message>,
// ) -> Result<Json<Message>, ErrorResponse> {
//     info!(
//         "添加消息到对话: conversation_id={}, role={}, user_id={}",
//         conversation_id, request.role, auth_user.user_id
//     );

//     // 验证用户是否有权限访问该对话
//     let conversation_exists: bool = sqlx::query_scalar(
//         "SELECT COUNT(*) > 0 FROM conversations WHERE conversation_id = $1 AND user_id = $2",
//     )
//     .bind(&conversation_id)
//     .bind(auth_user.user_id as i64)
//     .fetch_one(state.database.pool())
//     .await
//     .map_err(|e| {
//         error!("验证对话权限失败: {}", e);
//         ErrorResponse {
//             error: "database_error".to_string(),
//             message: "验证对话权限失败".to_string(),
//             details: Some(serde_json::json!({
//                 "error": e.to_string()
//             })),
//             timestamp: chrono::Local::now(),
//         }
//     })?;

//     if !conversation_exists {
//         warn!(
//             "用户 {} 尝试向无权限对话 {} 添加消息",
//             auth_user.user_id, conversation_id
//         );
//         return Err(ErrorResponse {
//             error: "conversation_not_found".to_string(),
//             message: "对话不存在或无权访问".to_string(),
//             details: Some(serde_json::json!({
//                 "conversation_id": conversation_id
//             })),
//             timestamp: chrono::Local::now(),
//         });
//     }

//     // 将metadata转换为JSON字符串
//     let metadata_str = request
//         .metadata
//         .as_ref()
//         .and_then(|m| serde_json::to_string(m).ok());

//     let sql = "INSERT INTO messages (conversation_id, role, content, model, prompt_tokens, completion_tokens, total_tokens, metadata, created_at) \
//               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING message_id";

//     let result = sqlx::query(sql)
//         .bind(&conversation_id)
//         .bind(&request.role)
//         .bind(&request.content)
//         .bind(&request.model)
//         .bind(request.usage.as_ref().map(|u| u.prompt_tokens as i32))
//         .bind(request.usage.as_ref().map(|u| u.completion_tokens as i32))
//         .bind(request.usage.as_ref().map(|u| u.total_tokens as i32))
//         .bind(metadata_str)
//         .bind(chrono::Local::now())
//         .fetch_one(state.database.pool())
//         .await
//         .map_err(|e| {
//             error!("添加消息失败: {}", e);
//             ErrorResponse {
//                 error: "database_error".to_string(),
//                 message: "添加消息失败".to_string(),
//                 details: Some(serde_json::json!({
//                     "error": e.to_string()
//                 })),
//                 timestamp: chrono::Local::now(),
//             }
//         })?;

//     let message_id: i64 = result.try_get("message_id").unwrap_or(0);

//     // 更新对话的消息计数和更新时间
//     let update_sql = "UPDATE conversations SET message_count = message_count + 1, updated_at = $1 WHERE conversation_id = $2";
//     sqlx::query(update_sql)
//         .bind(chrono::Local::now())
//         .bind(&conversation_id)
//         .execute(state.database.pool())
//         .await
//         .map_err(|e| {
//             error!("更新对话信息失败: {}", e);
//             ErrorResponse {
//                 error: "database_error".to_string(),
//                 message: "更新对话信息失败".to_string(),
//                 details: Some(serde_json::json!({
//                     "error": e.to_string()
//                 })),
//                 timestamp: chrono::Local::now(),
//             }
//         })?;

//     let message = Message {
//         id: Some(message_id as i32),
//         conversation_id: conversation_id.clone(),
//         role: request.role,
//         content: request.content,
//         model: request.model,
//         usage: request.usage,
//         metadata: request.metadata,
//         created_at: chrono::Local::now(),
//     };

//     Ok(Json(message))
// }
