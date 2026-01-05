use std::collections::HashMap;
use axum::Extension;

use crate::server::models::{ChatRequest, ChatResponse, ErrorResponse};
use crate::server::agent::{qwen, ollama, coating_optimization};
use crate::server::middleware::auth::AuthUser;

/// 模型路由器
pub struct ModelRouter {
    handlers: HashMap<String, HandlerFn>,
}

/// 处理器函数类型，返回(Response, ChatResponse)
type HandlerFn = fn(ChatRequest, crate::server::middleware::auth::AuthUser) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(axum::response::Response, ChatResponse), ErrorResponse>> + Send>>;

impl ModelRouter {
    /// 创建新的模型路由器
    pub fn new() -> Self {
        let mut router = Self {
            handlers: HashMap::new(),
        };

        // 注册通义千问模型
        router.register("qwen-plus", |req, auth_user| Box::pin(qwen::qwen_plus(req, auth_user)));
        router.register("qwen-turbo", |req, auth_user| Box::pin(qwen::qwen_turbo(req, auth_user)));
        router.register("qwen-max", |req, auth_user| Box::pin(qwen::qwen_max(req, auth_user)));
        router.register("qwen-flash", |req, auth_user| Box::pin(qwen::qwen_flash(req, auth_user)));
        router.register("qwq-plus", |req, auth_user| Box::pin(qwen::qwq_plus(req, auth_user)));

        // 注册Ollama模型
        router.register("ollama-qwen3-4b", |req, auth_user| Box::pin(ollama::ollama_qwen3_4b(req, auth_user)));
        router.register("ollama-llama3", |req, auth_user| Box::pin(ollama::ollama_llama3(req, auth_user)));
        router.register("calphamesh", |req, auth_user| Box::pin(qwen::CalphaMesh(req, auth_user)));
        router.register("phase-field", |req, auth_user| Box::pin(qwen::PhaseField(req, auth_user)));
        router.register("ml-server", |req, auth_user| Box::pin(qwen::ML_Server(req, auth_user)));

        router.register("coating", |req, auth_user| Box::pin(coating_optimization::coating_optimization(req, auth_user)));
        router
    }

    /// 注册模型处理器
    fn register(&mut self, model_name: &str, handler: HandlerFn) {
        self.handlers.insert(model_name.to_string(), handler);
    }

    /// 处理聊天请求并返回ChatResponse用于保存助手消息
    pub async fn handle_chat_request_with_response(&self, request: ChatRequest, auth_user: crate::server::middleware::auth::AuthUser) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
        let handler = self.handlers.get(&request.model)
            .ok_or_else(|| ErrorResponse {
                error: "model_not_supported".to_string(),
                message: format!("不支持的模型: {}", request.model),
                details: Some(serde_json::json!({
                    "available_models": self.get_available_models()
                })),
                timestamp: chrono::Local::now(),
            })?;

        handler(request, auth_user).await
    }

    
    /// 获取所有可用模型
    pub fn get_available_models(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    /// 检查模型是否可用
    pub fn is_model_available(&self, model_name: &str) -> bool {
        self.handlers.contains_key(model_name)
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局模型路由器实例
static MODEL_ROUTER: std::sync::OnceLock<std::sync::Arc<ModelRouter>> = std::sync::OnceLock::new();

/// 获取全局模型路由器实例
pub fn get_model_router() -> &'static std::sync::Arc<ModelRouter> {
    MODEL_ROUTER.get_or_init(|| {
        std::sync::Arc::new(ModelRouter::new())
    })
}