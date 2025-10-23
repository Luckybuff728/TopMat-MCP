//! Cohere API 客户端和 Rig 集成
//!
//! # 示例
//! ```
//! use rig::providers::cohere;
//!
//! let client = cohere::Client::new("YOUR_API_KEY");
//!
//! let command_r = client.completion_model(cohere::COMMAND_R);
//! ```

pub mod client;
pub mod completion;
pub mod embeddings;
pub mod streaming;

pub use client::Client;
pub use client::{ApiErrorResponse, ApiResponse};
pub use completion::CompletionModel;
pub use embeddings::EmbeddingModel;

// ================================================================
// Cohere 完成模型
// ================================================================

/// `command-r-plus` 完成模型
pub const COMMAND_R_PLUS: &str = "command-r-plus";
/// `command-r` 完成模型
pub const COMMAND_R: &str = "command-r";
/// `command` 完成模型
pub const COMMAND: &str = "command";
/// `command-nightly` 完成模型
pub const COMMAND_NIGHTLY: &str = "command-nightly";
/// `command-light` 完成模型
pub const COMMAND_LIGHT: &str = "command-light";
/// `command-light-nightly` 完成模型
pub const COMMAND_LIGHT_NIGHTLY: &str = "command-light-nightly";

// ================================================================
// Cohere 嵌入模型
// ================================================================

/// `embed-english-v3.0` 嵌入模型
pub const EMBED_ENGLISH_V3: &str = "embed-english-v3.0";
/// `embed-english-light-v3.0` 嵌入模型
pub const EMBED_ENGLISH_LIGHT_V3: &str = "embed-english-light-v3.0";
/// `embed-multilingual-v3.0` 嵌入模型
pub const EMBED_MULTILINGUAL_V3: &str = "embed-multilingual-v3.0";
/// `embed-multilingual-light-v3.0` 嵌入模型
pub const EMBED_MULTILINGUAL_LIGHT_V3: &str = "embed-multilingual-light-v3.0";
/// `embed-english-v2.0` 嵌入模型
pub const EMBED_ENGLISH_V2: &str = "embed-english-v2.0";
/// `embed-english-light-v2.0` 嵌入模型
pub const EMBED_ENGLISH_LIGHT_V2: &str = "embed-english-light-v2.0";
/// `embed-multilingual-v2.0` 嵌入模型
pub const EMBED_MULTILINGUAL_V2: &str = "embed-multilingual-v2.0";
