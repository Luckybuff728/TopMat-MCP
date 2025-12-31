use axum::{
    extract::Request,
    http::{header, StatusCode},
    response::{Response, IntoResponse},
};
use tracing::{debug, warn};

use crate::server::models::{ErrorResponse, AuthError, AuthResult, ApiKeyInfo, UserInfo};

/// 从请求中提取API Key
pub fn extract_api_key(request: &Request) -> Option<String> {
    extract_api_key_from_headers(request.headers())
}

pub fn extract_api_key_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    use axum::http::header;

    // 从Authorization header中提取Bearer token
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.to_lowercase().starts_with("bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }

    None
}
/// 创建鉴权成功响应
pub fn create_auth_response(auth_result: AuthResult) -> Response {
    let success_response = serde_json::json!({
        "valid": true,
        "message": "鉴权成功",
        "user": {
            "username": auth_result.user_info.username,
            "subscription_level": auth_result.user_info.subscription_level,
            "email": auth_result.user_info.email
        },
        "api_key": {
            "key_name": auth_result.api_key_info.key_name,
            "expires_at": auth_result.api_key_info.expires_at
        },
        "timestamp": chrono::Local::now()
    });

    axum::Json(success_response).into_response()
}

/// 创建鉴权错误响应
pub fn create_error_response(auth_error: AuthError) -> ErrorResponse {
    let (error_type, message) = match auth_error {
        AuthError::InvalidApiKey => ("invalid_api_key", "无效的API Key"),
        AuthError::ExpiredApiKey => ("expired_api_key", "API Key已过期"),
        AuthError::InactiveApiKey => ("inactive_api_key", "API Key未激活"),
        AuthError::SubscriptionExpired => ("subscription_expired", "用户订阅已过期"),
        AuthError::CacheExpired => ("cache_expired", "认证缓存已过期，需要重新验证"),
        AuthError::DatabaseError(_) => ("database_error", "数据库错误"),
        AuthError::RequestError(_) => ("auth_service_error", "鉴权服务暂时不可用，请稍后重试"),
        AuthError::HttpError(_) => ("auth_service_error", "鉴权服务暂时不可用，请稍后重试"),
        AuthError::JsonError(_) => ("auth_service_error", "鉴权服务暂时不可用，请稍后重试"),
    };

    ErrorResponse {
        error: error_type.to_string(),
        message: message.to_string(),
        details: Some(serde_json::json!({
            "auth_error": auth_error.to_string()
        })),
        timestamp: chrono::Local::now(),
    }
}

/// 创建缺少API Key错误响应
pub fn create_missing_api_key_response() -> ErrorResponse {
    ErrorResponse {
        error: "missing_api_key".to_string(),
        message: "请求中缺少API Key，请在Authorization header中提供 'Bearer <api_key>'".to_string(),
        details: None,
        timestamp: chrono::Local::now(),
    }
}

/// 验证API Key并返回AuthResult的通用函数
pub async fn verify_api_key_from_request(
    request: &Request,
    auth_client: &super::AuthClient,
) -> Result<AuthResult, AuthError> {
    let api_key = extract_api_key(request).ok_or_else(|| {
        warn!("请求中未找到API Key");
        AuthError::InvalidApiKey // 使用InvalidApiKey作为缺少API Key的错误
    })?;

    debug!("验证API Key: {}", &api_key[..api_key.len().min(8)]);

    auth_client.verify_api_key(&api_key).await
}
