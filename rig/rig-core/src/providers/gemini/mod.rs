//! Google Gemini API 客户端和 Rig 集成
//!
//! # 示例
//! ```
//! use rig::providers::google;
//!
//! let client = google::Client::new("YOUR_API_KEY");
//!
//! let gemini_embedding_model = client.embedding_model(google::EMBEDDING_001);
//! ```

pub mod client;
pub mod completion;
pub mod embedding;
pub mod streaming;
pub mod transcription;

pub use client::Client;

pub mod gemini_api_types {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ExecutionLanguage {
        /// 未指定的语言。不应使用此值。
        LanguageUnspecified,
        /// Python >= 3.10，带有 numpy 和简单可用。
        Python,
    }

    /// 由模型生成的要执行的代码，结果返回给模型。
    /// 仅在使用 CodeExecution 工具时生成，其中代码将自动执行，
    /// 并生成相应的 CodeExecutionResult。
    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
    pub struct ExecutableCode {
        /// 代码的编程语言。
        pub language: ExecutionLanguage,
        /// 要执行的代码。
        pub code: String,
    }
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct CodeExecutionResult {
        /// 代码执行的结果。
        pub outcome: CodeExecutionOutcome,
        /// 代码执行成功时包含 stdout，否则包含 stderr 或其他描述。
        #[serde(skip_serializing_if = "Option::is_none")]
        pub output: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub enum CodeExecutionOutcome {
        /// 未指定的状态。不应使用此值。
        #[serde(rename = "OUTCOME_UNSPECIFIED")]
        Unspecified,
        /// 代码执行成功完成。
        #[serde(rename = "OUTCOME_OK")]
        Ok,
        /// 代码执行完成但失败。stderr 应包含原因。
        #[serde(rename = "OUTCOME_FAILED")]
        Failed,
        /// 代码执行时间过长，被取消。可能存在或不存在部分输出。
        #[serde(rename = "OUTCOME_DEADLINE_EXCEEDED")]
        DeadlineExceeded,
    }
}
