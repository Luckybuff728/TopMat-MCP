//! 此模块包含 Rig 支持的不同 LLM 提供商的客户端。
//!
//! 目前支持以下提供商：
//! - Cohere
//! - OpenAI
//! - Perplexity
//! - Anthropic
//! - Google Gemini
//! - xAI
//! - EternalAI
//! - DeepSeek
//! - Azure OpenAI
//! - Mira
//! - Qwen（通义千问）
//!
//! 每个提供商都有自己的模块，其中包含可用于初始化完成和嵌入模型并执行对这些模型的请求的 `Client` 实现。
//!
//! 客户端还包含轻松创建更高级别 AI 构造（如代理和 RAG 系统）的方法，减少了样板代码的需求。
//!
//! # 示例
//! ```
//! use rig::{providers::openai, agent::AgentBuilder};
//!
//! // 初始化 OpenAI 客户端
//! let openai = openai::Client::new("your-openai-api-key");
//!
//! // 创建模型并初始化代理
//! let gpt_4o = openai.completion_model("gpt-4o");
//!
//! let agent = AgentBuilder::new(gpt_4o)
//!     .preamble("\
//!         You are Gandalf the white and you will be conversing with other \
//!         powerful beings to discuss the fate of Middle Earth.\
//!     ")
//!     .build();
//!
//! // 或者，您可以直接初始化代理
//! let agent = openai.agent("gpt-4o")
//!     .preamble("\
//!         You are Gandalf the white and you will be conversing with other \
//!         powerful beings to discuss the fate of Middle Earth.\
//!     ")
//!     .build();
//! ```
//! 注意：上面的示例使用 OpenAI 提供商客户端，但相同的模式可以与 Cohere 提供商客户端一起使用。
pub mod anthropic;
pub mod azure;
pub mod cohere;
pub mod deepseek;
pub mod galadriel;
pub mod gemini;
pub mod groq;
pub mod huggingface;
pub mod hyperbolic;
pub mod mira;
pub mod mistral;
pub mod moonshot;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod perplexity;
pub mod qwen;
pub mod together;
pub mod voyageai;
pub mod xai;
