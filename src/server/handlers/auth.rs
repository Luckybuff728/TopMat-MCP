use axum::extract::{Request, State};
use tracing::info;

use super::chat::ServerState;
use crate::server::auth::{
    create_auth_response, create_error_response, create_missing_api_key_response, extract_api_key,
};
use crate::server::models::*;

/// 鉴权端点处理
#[utoipa::path(
    post,
    path = "/v1/auth",
    tag = "auth",
    summary = "API Key 鉴权",
    description = "验证API Key的有效性并获取用户信息\n\n**认证方式**: Bearer Token\n```\nAuthorization: Bearer <your_api_key>\n```",
    responses(
        (status = 200, description = "鉴权成功", body = AuthResponse),
        (status = 401, description = "API Key缺失、无效或已过期", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
)]

pub async fn auth_handler(
    State(state): State<ServerState>,
    request: Request,
) -> Result<axum::response::Response, ErrorResponse> {
    // 检查是否有API Key
    if extract_api_key(&request).is_none() {
        return Err(create_missing_api_key_response());
    }

    info!("收到鉴权请求");

    // 验证API Key
    match state
        .auth_client
        .verify_api_key(&extract_api_key(&request).unwrap())
        .await
    {
        Ok(auth_result) => {
            info!(
                "用户鉴权成功: {} (订阅级别: {})",
                auth_result.user_info.username, auth_result.user_info.subscription_level
            );

            Ok(create_auth_response(auth_result))
        }
        Err(auth_error) => {
            tracing::error!("API Key验证失败: {}", auth_error);
            Err(create_error_response(auth_error))
        }
    }
}
