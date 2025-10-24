use axum::{
    extract::{State, Request},
    response::IntoResponse,
};
use tracing::info;

use crate::server::models::*;
use crate::server::auth::{AuthClient, extract_api_key, create_auth_response, create_error_response, create_missing_api_key_response};
use super::chat::ServerState;

/// 鉴权端点处理
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
    match state.auth_client.verify_api_key(&extract_api_key(&request).unwrap()).await {
        Ok(auth_result) => {
            info!("用户鉴权成功: {} (订阅级别: {})",
                  auth_result.user_info.username,
                  auth_result.user_info.subscription_level);

            Ok(create_auth_response(auth_result))
        }
        Err(auth_error) => {
            tracing::error!("API Key验证失败: {}", auth_error);
            Err(create_error_response(auth_error))
        }
    }
}