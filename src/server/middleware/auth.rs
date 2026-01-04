use axum::{
    extract::{Request, State},
    http::{header, Method, StatusCode},
    middleware::Next,
    response::Response,
    Extension,
};
use tracing::{error, warn, debug};

use crate::server::auth::{AuthClient, extract_api_key};
use crate::server::models::{ErrorResponse, ApiKeyInfo, UserInfo};
use crate::server::handlers::chat::ServerState;

/// 认证用户信息
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: u32,
    pub username: String,
    pub email: String,
    pub subscription_level: String,
    pub api_key: String,
}

/// 认证中间件
pub struct AuthMiddleware;

impl AuthMiddleware {
    /// 验证请求中的API密钥（所有请求都需要认证）
    pub async fn authenticate_request(
        State(state): State<ServerState>,
        request: Request,
        next: Next,
    ) -> Result<Response, ErrorResponse> {
        Self::authenticate_request_impl(state, request, next, false).await
    }

    /// 验证请求中的API密钥（GET请求跳过认证，用于MCP协议初始化）
    pub async fn authenticate_request_skip_get(
        State(state): State<ServerState>,
        request: Request,
        next: Next,
    ) -> Result<Response, ErrorResponse> {
        Self::authenticate_request_impl(state, request, next, true).await
    }

    /// 认证实现（内部方法）
    async fn authenticate_request_impl(
        state: ServerState,
        request: Request,
        next: Next,
        skip_get_requests: bool,
    ) -> Result<Response, ErrorResponse> {
        let method = request.method().clone();

        // 如果启用了跳过GET请求，且当前是GET请求，则直接放行
        if skip_get_requests && method == Method::GET {
            debug!("GET 请求无需认证: {}", request.uri());
            return Ok(next.run(request).await);
        }

        debug!("开始认证请求: {}", request.uri());

        // 提取API密钥
        let api_key = extract_api_key(&request).ok_or_else(|| ErrorResponse {
            error: "missing_api_key".to_string(),
            message: "缺少API密钥".to_string(),
            details: None,
            timestamp: chrono::Local::now(),
        })?;

        debug!("提取到API密钥: {}...", &api_key[..std::cmp::min(4, api_key.len())]);

        // 验证API密钥
        let auth_result = state.auth_client.verify_api_key(&api_key).await
            .map_err(|e| {
                error!("API密钥验证失败: {}", e);
                ErrorResponse {
                    error: "auth_service_error".to_string(),
                    message: "认证服务不可用".to_string(),
                    details: Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                    timestamp: chrono::Local::now(),
                }
            })?;

        // 检查API密钥状态
        if !auth_result.api_key_info.is_active {
            warn!("API密钥未激活: {}", api_key);
            return Err(ErrorResponse {
                error: "inactive_api_key".to_string(),
                message: "API密钥未激活".to_string(),
                details: None,
                timestamp: chrono::Local::now(),
            });
        }

        // 检查API密钥是否过期
        if let Some(expires_str) = &auth_result.api_key_info.expires_at {
            if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(expires_str) {
                if expires_at < chrono::Local::now() {
                    warn!("API密钥已过期: {}", api_key);
                    return Err(ErrorResponse {
                        error: "expired_api_key".to_string(),
                        message: "API密钥已过期".to_string(),
                        details: None,
                        timestamp: chrono::Local::now(),
                    });
                }
            }
        }

        // 检查用户订阅是否过期
        if let Some(sub_expires_str) = &auth_result.user_info.subscription_expires_at {
            if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(sub_expires_str) {
                if expires_at < chrono::Local::now() {
                    warn!("用户订阅已过期: user_id={}", auth_result.user_info.id);
                    return Err(ErrorResponse {
                        error: "subscription_expired".to_string(),
                        message: "用户订阅已过期".to_string(),
                        details: None,
                        timestamp: chrono::Local::now(),
                    });
                }
            }
        }

        // 创建认证用户信息
        let auth_user = AuthUser {
            user_id: auth_result.user_info.id,
            username: auth_result.user_info.username.clone(),
            email: auth_result.user_info.email.clone(),
            subscription_level: auth_result.user_info.subscription_level.clone(),
            api_key: auth_result.api_key_info.api_key.clone(),
        };

        debug!("用户认证成功: user_id={}, username={}, subscription_level={}",
              auth_user.user_id, auth_user.username, auth_user.subscription_level);

        // 将用户信息注入到请求扩展中
        let mut request = request;
        request.extensions_mut().insert(auth_user.clone());

        // 继续处理请求
        let mut response = next.run(request).await;

        // 在响应头中添加用户ID，供其他中间件使用
        response.headers_mut().insert(
            "X-User-ID",
            auth_user.user_id.to_string().parse().unwrap_or_else(|_| {
                warn!("Failed to parse user ID as header value");
                axum::http::HeaderValue::from_static("1")
            })
        );

        Ok(response)
    }

  }

/// 用于从请求扩展中提取认证用户信息的trait
pub trait AuthExtractor {
    fn get_auth_user(&self) -> Result<&AuthUser, ErrorResponse>;
}

impl AuthExtractor for Extension<AuthUser> {
    fn get_auth_user(&self) -> Result<&AuthUser, ErrorResponse> {
        Ok(&self.0)
    }
}

/// 为处理器提供的便捷宏，用于提取认证用户
#[macro_export]
macro_rules! get_auth_user {
    ($extensions:expr) => {
        $extensions.get::<AuthUser>().ok_or_else(|| {
            crate::server::models::ErrorResponse {
                error: "authentication_required".to_string(),
                message: "需要认证".to_string(),
                details: None,
                timestamp: chrono::Local::now(),
            }
        })
    };
}

/// 检查用户订阅级别的便捷函数
pub fn check_subscription_level(user: &AuthUser, required_level: &str) -> bool {
    match (user.subscription_level.as_str(), required_level) {
        // Pro用户可以访问所有功能
        ("pro", _) => true,
        // Basic用户只能访问basic级别及以下
        ("basic", "basic") => true,
        // 其他情况需要精确匹配
        (user_level, required) => user_level == required,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_extract_api_key_bearer() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header(header::AUTHORIZATION, "Bearer test-api-key-123")
            .body(Body::empty())
            .unwrap();

        let api_key = extract_api_key(&request).unwrap();
        assert_eq!(api_key, "test-api-key-123");
    }

    #[tokio::test]
    async fn test_extract_api_key_header() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("X-API-Key", "test-api-key-456")
            .body(Body::empty())
            .unwrap();

        let api_key = extract_api_key(&request).unwrap();
        assert_eq!(api_key, "test-api-key-456");
    }

    #[tokio::test]
    async fn test_extract_api_key_missing() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = extract_api_key(&request);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_subscription_level() {
        let pro_user = AuthUser {
            user_id: 1,
            username: "pro_user".to_string(),
            email: "pro@example.com".to_string(),
            subscription_level: "pro".to_string(),
            api_key: "test-key".to_string(),
        };

        let basic_user = AuthUser {
            user_id: 2,
            username: "basic_user".to_string(),
            email: "basic@example.com".to_string(),
            subscription_level: "basic".to_string(),
            api_key: "test-key".to_string(),
        };

        assert!(check_subscription_level(&pro_user, "pro"));
        assert!(check_subscription_level(&pro_user, "basic"));
        assert!(check_subscription_level(&basic_user, "basic"));
        assert!(!check_subscription_level(&basic_user, "pro"));
    }
}