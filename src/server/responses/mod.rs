use axum::{
    http::StatusCode,
    response::IntoResponse,
};
use crate::server::models::ErrorResponse;

// 实现ErrorResponse的IntoResponse
impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;

        let status = match self.error.as_str() {
            "model_not_supported" => StatusCode::BAD_REQUEST,
            "qwen_not_configured" => StatusCode::BAD_REQUEST,
            "chat_failed" => StatusCode::INTERNAL_SERVER_ERROR,
            "streaming_chat_failed" => StatusCode::INTERNAL_SERVER_ERROR,
            "missing_api_key" => StatusCode::UNAUTHORIZED,
            "invalid_api_key" => StatusCode::UNAUTHORIZED,
            "expired_api_key" => StatusCode::UNAUTHORIZED,
            "inactive_api_key" => StatusCode::FORBIDDEN,
            "subscription_expired" => StatusCode::FORBIDDEN,
            "auth_service_error" => StatusCode::SERVICE_UNAVAILABLE,
            "invalid_request" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, axum::Json(self)).into_response()
    }
}