//! OpenRouter Inference API 客户端和 Rig 集成
//!
//! # 示例
//! ```
//! use rig::providers::openrouter;
//!
//! let client = openrouter::Client::new("YOUR_API_KEY");
//!
//! let llama_3_1_8b = client.completion_model(openrouter::LLAMA_3_1_8B);
//! ```

pub mod client;
pub mod completion;
pub mod streaming;

pub use client::*;
pub use completion::*;
