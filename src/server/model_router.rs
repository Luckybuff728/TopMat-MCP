use std::collections::HashMap;

use crate::server::agent::{coating_optimization, ollama, qwen};
use crate::server::models::{ChatRequest, ChatResponse, ErrorResponse};

/// 模型路由器
pub struct ModelRouter {
    handlers: HashMap<String, HandlerFn>,
}

/// 处理器函数类型，返回(Response, ChatResponse)
type HandlerFn = fn(
    crate::server::database::DatabaseConnection,
    ChatRequest,
    crate::server::middleware::auth::AuthUser,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<
                Output = Result<(axum::response::Response, ChatResponse), ErrorResponse>,
            > + Send,
    >,
>;

impl ModelRouter {
    /// 创建新的模型路由器
    pub fn new() -> Self {
        let mut router = Self {
            handlers: HashMap::new(),
        };

        // 注册通义千问模型
        router.register("qwen-plus", |db, req, auth_user| {
            Box::pin(qwen::qwen_plus(db, req, auth_user))
        });
        router.register("qwen-turbo", |db, req, auth_user| {
            Box::pin(qwen::qwen_turbo(db, req, auth_user))
        });
        router.register("qwen-max", |db, req, auth_user| {
            Box::pin(qwen::qwen_max(db, req, auth_user))
        });
        router.register("qwen-flash", |db, req, auth_user| {
            Box::pin(qwen::qwen_flash(db, req, auth_user))
        });
        router.register("qwq-plus", |db, req, auth_user| {
            Box::pin(qwen::qwq_plus(db, req, auth_user))
        });

        // 注册Ollama模型
        router.register("ollama-qwen3-4b", |db, req, auth_user| {
            Box::pin(ollama::ollama_qwen3_4b(db, req, auth_user))
        });
        router.register("ollama-llama3", |db, req, auth_user| {
            Box::pin(ollama::ollama_llama3(db, req, auth_user))
        });
        // 注册带工具的模型
        router.register("calphamesh", |db, req, auth_user| {
            Box::pin(qwen::calphamesh(db, req, auth_user))
        });
        router.register("phase-field", |db, req, auth_user| {
            Box::pin(qwen::phase_field(db, req, auth_user))
        });
        router.register("ml-server", |db, req, auth_user| {
            Box::pin(qwen::ml_server(db, req, auth_user))
        });

        router.register("coating", |db, req, auth_user| {
            Box::pin(coating_optimization::coating_optimization(
                db, req, auth_user,
            ))
        });
        router.register("battery", |db, req, auth_user| {
            Box::pin(qwen::battery(db, req, auth_user))
        });
        router
    }

    /// 注册模型处理器
    fn register(&mut self, model_name: &str, handler: HandlerFn) {
        self.handlers.insert(model_name.to_string(), handler);
    }

    /// 处理聊天请求并返回ChatResponse用于保存助手消息
    pub async fn handle_chat_request_with_response(
        &self,
        db: crate::server::database::DatabaseConnection,
        request: ChatRequest,
        auth_user: crate::server::middleware::auth::AuthUser,
    ) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
        let handler = self
            .handlers
            .get(&request.model)
            .ok_or_else(|| ErrorResponse {
                error: "model_not_supported".to_string(),
                message: format!("不支持的模型: {}", request.model),
                details: Some(serde_json::json!({
                    "available_models": self.get_available_models()
                })),
                timestamp: chrono::Local::now(),
            })?;

        handler(db, request, auth_user).await
    }

    /// 获取所有可用模型
    pub fn get_available_models(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    /// 检查模型是否可用
    #[allow(dead_code)]
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
    MODEL_ROUTER.get_or_init(|| std::sync::Arc::new(ModelRouter::new()))
}
