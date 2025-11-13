use std::collections::HashMap;

use crate::server::models::{ChatRequest, ChatResponse, ErrorResponse};
use crate::server::agent::{qwen, ollama, coating_optimization};

/// 模型路由器
pub struct ModelRouter {
    handlers: HashMap<String, HandlerFn>,
}

/// 处理器函数类型，返回(Response, ChatResponse)
type HandlerFn = fn(ChatRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(axum::response::Response, ChatResponse), ErrorResponse>> + Send>>;

impl ModelRouter {
    /// 创建新的模型路由器
    pub fn new() -> Self {
        let mut router = Self {
            handlers: HashMap::new(),
        };

        // 注册通义千问模型
        router.register("qwen-plus", |req| Box::pin(qwen::qwen_plus(req)));
        router.register("qwen-turbo", |req| Box::pin(qwen::qwen_turbo(req)));
        router.register("qwen-max", |req| Box::pin(qwen::qwen_max(req)));
        router.register("qwen-flash", |req| Box::pin(qwen::qwen_flash(req)));
        router.register("qwq-plus", |req| Box::pin(qwen::qwq_plus(req)));

        // 注册Ollama模型
        router.register("ollama-qwen3-4b", |req| Box::pin(ollama::ollama_qwen3_4b(req)));
        router.register("ollama-llama3", |req| Box::pin(ollama::ollama_llama3(req)));

        router.register("coating", |req| Box::pin(coating_optimization::coating_optimization(req)));
        router
    }

    /// 注册模型处理器
    fn register(&mut self, model_name: &str, handler: HandlerFn) {
        self.handlers.insert(model_name.to_string(), handler);
    }

    /// 处理聊天请求并返回ChatResponse用于保存助手消息
    pub async fn handle_chat_request_with_response(&self, request: ChatRequest) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
        let handler = self.handlers.get(&request.model)
            .ok_or_else(|| ErrorResponse {
                error: "model_not_supported".to_string(),
                message: format!("不支持的模型: {}", request.model),
                details: Some(serde_json::json!({
                    "available_models": self.get_available_models()
                })),
                timestamp: chrono::Utc::now(),
            })?;

        handler(request).await
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