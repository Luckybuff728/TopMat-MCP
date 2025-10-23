//! 此模块定义了 [ToolSchema] 结构体，用于嵌入实现 [crate::tool::ToolEmbedding] 的对象

// 导入 Embed trait 和 ToolEmbeddingDyn trait
use crate::{Embed, tool::ToolEmbeddingDyn};
// 导入 serde 的 Serialize trait
use serde::Serialize;

// 导入父模块的 EmbedError
use super::embed::EmbedError;

/// 可嵌入的文档，用作工具在 RAG 工具时的中间表示。
// 可嵌入的文档，用作工具在 RAG 工具时的中间表示
// 派生 Clone, Serialize, Default, Eq, PartialEq trait
#[derive(Clone, Serialize, Default, Eq, PartialEq)]
pub struct ToolSchema {
    // 工具名称
    pub name: String,
    // 工具的上下文信息（JSON 格式）
    pub context: serde_json::Value,
    // 用于嵌入的文档列表
    pub embedding_docs: Vec<String>,
}

// 为 ToolSchema 实现 Embed trait
impl Embed for ToolSchema {
    // 将工具模式嵌入到文本嵌入器中
    fn embed(&self, embedder: &mut super::embed::TextEmbedder) -> Result<(), EmbedError> {
        // 遍历所有嵌入文档
        for doc in &self.embedding_docs {
            // 将每个文档添加到嵌入器中
            embedder.embed(doc.clone());
        }
        // 返回成功
        Ok(())
    }
}

// 为 ToolSchema 实现方法
impl ToolSchema {
    /// 将实现 [ToolEmbeddingDyn] 的项目转换为 [ToolSchema]。
    ///
    /// # 示例
    /// ```rust
    /// use rig::{
    ///     completion::ToolDefinition,
    ///     embeddings::ToolSchema,
    ///     tool::{Tool, ToolEmbedding, ToolEmbeddingDyn},
    /// };
    /// use serde_json::json;
    ///
    /// #[derive(Debug, thiserror::Error)]
    /// #[error("Math error")]
    /// struct NothingError;
    ///
    /// #[derive(Debug, thiserror::Error)]
    /// #[error("Init error")]
    /// struct InitError;
    ///
    /// struct Nothing;
    /// impl Tool for Nothing {
    ///     const NAME: &'static str = "nothing";
    ///
    ///     type Error = NothingError;
    ///     type Args = ();
    ///     type Output = ();
    ///
    ///     async fn definition(&self, _prompt: String) -> ToolDefinition {
    ///         serde_json::from_value(json!({
    ///             "name": "nothing",
    ///             "description": "nothing",
    ///             "parameters": {}
    ///         }))
    ///         .expect("Tool Definition")
    ///     }
    ///
    ///     async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// impl ToolEmbedding for Nothing {
    ///     type InitError = InitError;
    ///     type Context = ();
    ///     type State = ();
    ///
    ///     fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
    ///         Ok(Nothing)
    ///     }
    ///
    ///     fn embedding_docs(&self) -> Vec<String> {
    ///         vec!["Do nothing.".into()]
    ///     }
    ///
    ///     fn context(&self) -> Self::Context {}
    /// }
    ///
    /// let tool = ToolSchema::try_from(&Nothing).unwrap();
    ///
    /// assert_eq!(tool.name, "nothing".to_string());
    /// assert_eq!(tool.embedding_docs, vec!["Do nothing.".to_string()]);
    /// ```
    // 将实现 [ToolEmbeddingDyn] 的项目转换为 [ToolSchema]
    //
    // # 示例
    // ```rust
    // use rig::{
    //     completion::ToolDefinition,
    //     embeddings::ToolSchema,
    //     tool::{Tool, ToolEmbedding, ToolEmbeddingDyn},
    // };
    // use serde_json::json;
    //
    // #[derive(Debug, thiserror::Error)]
    // #[error("Math error")]
    // struct NothingError;
    //
    // #[derive(Debug, thiserror::Error)]
    // #[error("Init error")]
    // struct InitError;
    //
    // struct Nothing;
    // impl Tool for Nothing {
    //     const NAME: &'static str = "nothing";
    //
    //     type Error = NothingError;
    //     type Args = ();
    //     type Output = ();
    //
    //     async fn definition(&self, _prompt: String) -> ToolDefinition {
    //         serde_json::from_value(json!({
    //             "name": "nothing",
    //             "description": "nothing",
    //             "parameters": {}
    //         }))
    //         .expect("Tool Definition")
    //     }
    //
    //     async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    //         Ok(())
    //     }
    // }
    //
    // impl ToolEmbedding for Nothing {
    //     type InitError = InitError;
    //     type Context = ();
    //     type State = ();
    //
    //     fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
    //         Ok(Nothing)
    //     }
    //
    //     fn embedding_docs(&self) -> Vec<String> {
    //         vec!["Do nothing.".into()]
    //     }
    //
    //     fn context(&self) -> Self::Context {}
    // }
    //
    // let tool = ToolSchema::try_from(&Nothing).unwrap();
    //
    // assert_eq!(tool.name, "nothing".to_string());
    // assert_eq!(tool.embedding_docs, vec!["Do nothing.".to_string()]);
    // ```
    // 尝试从动态工具嵌入对象创建 ToolSchema
    pub fn try_from(tool: &dyn ToolEmbeddingDyn) -> Result<Self, EmbedError> {
        // 创建 ToolSchema 实例
        Ok(ToolSchema {
            // 获取工具名称
            name: tool.name(),
            // 获取工具上下文，如果出错则转换为 EmbedError
            context: tool.context().map_err(EmbedError::new)?,
            // 获取嵌入文档列表
            embedding_docs: tool.embedding_docs(),
        })
    }
}
