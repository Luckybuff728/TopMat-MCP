//! MCP 认证中间件
//! 
//! 专门用于 MCP 端点的认证，只对 POST 请求进行认证
//! GET 请求（如协议初始化）不需要认证

use axum::{
    extract::{Request, State},
    http::Method,
    middleware::Next,
    response::Response,
    Extension,
};
use tracing::{info, warn, debug};

use crate::server::auth::extract_api_key;
use crate::server::models::ErrorResponse;
use crate::server::handlers::chat::ServerState;
use crate::server::middleware::auth::AuthUser;

/// MCP 认证中间件 - 只对 POST 请求进行认证
pub struct McpAuthMiddleware;

impl McpAuthMiddleware {
    pub async fn authenticate_request(
        State(state): State<ServerState>,
        request: Request,
        next: Next,
    ) -> Result<Response, ErrorResponse> {
        let method = request.method().clone();
        
        // 只对 POST 请求进行认证
        if method != Method::POST {
            debug!("MCP: {} 请求无需认证: {}", method, request.uri());
            return Ok(next.run(request).await);
        }
        
        debug!("MCP: POST 请求需要认证: {}", request.uri());
        
        // 提取API密钥
        let api_key = extract_api_key(&request).ok_or_else(|| ErrorResponse {
            error: "missing_api_key".to_string(),
            message: "缺少API密钥".to_string(),
            details: None,
            timestamp: chrono::Utc::now(),
        })?;

        debug!("MCP: 提取到API密钥: {}...", &api_key[..std::cmp::min(4, api_key.len())]);

        // 验证API密钥
        let auth_result = state.auth_client.verify_api_key(&api_key).await
            .map_err(|e| {
                warn!("MCP: API密钥验证失败: {}", e);
                ErrorResponse {
                    error: "auth_service_error".to_string(),
                    message: "认证服务不可用".to_string(),
                    details: Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                    timestamp: chrono::Utc::now(),
                }
            })?;

        // 检查API密钥状态
        if !auth_result.api_key_info.is_active {
            warn!("MCP: API密钥未激活: {}", api_key);
            return Err(ErrorResponse {
                error: "inactive_api_key".to_string(),
                message: "API密钥未激活".to_string(),
                details: None,
                timestamp: chrono::Utc::now(),
            });
        }

        // 检查API密钥是否过期
        if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(&auth_result.api_key_info.expires_at) {
            if expires_at < chrono::Utc::now() {
                warn!("MCP: API密钥已过期: {}", api_key);
                return Err(ErrorResponse {
                    error: "expired_api_key".to_string(),
                    message: "API密钥已过期".to_string(),
                    details: None,
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        // 检查用户订阅是否过期
        if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(&auth_result.user_info.subscription_expires_at) {
            if expires_at < chrono::Utc::now() {
                warn!("MCP: 用户订阅已过期: user_id={}", auth_result.user_info.id);
                return Err(ErrorResponse {
                    error: "subscription_expired".to_string(),
                    message: "用户订阅已过期".to_string(),
                    details: None,
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        // 创建认证用户信息
        let auth_user = AuthUser {
            user_id: auth_result.user_info.id,
            username: auth_result.user_info.username.clone(),
            email: auth_result.user_info.email.clone(),
            subscription_level: auth_result.user_info.subscription_level.clone(),
            api_key: api_key.clone(),
        };

        debug!(
            "MCP: 认证成功: user_id={}, username={}",
            auth_user.user_id, auth_user.username
        );

        // 将认证用户信息注入到请求中
        let mut request = request;
        request.extensions_mut().insert(auth_user);

        Ok(next.run(request).await)
    }
}

