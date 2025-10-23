//! 此模块定义了一个灵活的管道 API，用于定义一系列操作，
//! 这些操作可能使用也可能不使用 AI 组件（例如：语义搜索、LLM 提示等）。
//!
//! 管道 API 的灵感来源于通用编排管道，如 Airflow、Dagster 和 Prefect，
//! 但使用惯用的 Rust 模式实现，并提供一些开箱即用的 AI 特定操作以及通用组合器。
//!
//! 管道由一个或多个操作（或"ops"）组成，每个操作都必须实现 [Op] trait。
//! [Op] trait 只需要实现一个方法：`call`，它接受输入并返回输出。
//! 该 trait 提供了广泛的组合器来链接操作。
//!
//! 可以将管道视为 DAG（有向无环图），其中每个节点都是一个操作，
//! 边表示操作之间的数据流。当在某个输入上调用管道时，
//! 输入被传递给 DAG 的根节点（即：管道中定义的第一个操作）。
//! 然后每个操作的输出被传递给管道中的下一个操作，直到输出到达
//! 叶节点（即：管道中定义的最后一个操作）。然后叶节点的输出被返回
//! 作为管道的结果。
//!
//! ## 基本示例
//! 例如，下面的管道接受两个整数的元组，将它们相加，然后使用 [map](Op::map) 组合器方法
//! 将结果格式化为字符串，该方法将简单函数操作应用于前一个操作的输出：
//! ```rust
//! use rig::pipeline::{self, Op};
//!
//! let pipeline = pipeline::new()
//!     // op1: add two numbers
//!     .map(|(x, y)| x + y)
//!     // op2: format result
//!     .map(|z| format!("Result: {z}!"));
//!
//! let result = pipeline.call((1, 2)).await;
//! assert_eq!(result, "Result: 3!");
//! ```
//!
//! 这个管道可以可视化为以下 DAG：
//! ```text
//!          ┌─────────┐   ┌─────────┐         
//! Input───►│   op1   ├──►│   op2   ├──►Output
//!          └─────────┘   └─────────┘         
//! ```
//!
//! ## 并行操作
//! 管道 API 还提供了 [parallel!](crate::parallel!) 宏来并行运行操作。
//! 该宏接受一个操作列表并将它们转换为单个操作，该操作将复制输入
//! 并并发运行每个操作。然后收集每个操作的结果并作为元组返回。
//!
//! 例如，下面的管道并发运行两个操作：
//! ```rust
//! use rig::{pipeline::{self, Op, map}, parallel};
//!
//! let pipeline = pipeline::new()
//!     .chain(parallel!(
//!         // op1: add 1 to input
//!         map(|x| x + 1),
//!         // op2: subtract 1 from input
//!         map(|x| x - 1),
//!     ))
//!     // op3: format results
//!     .map(|(a, b)| format!("Results: {a}, {b}"));
//!
//! let result = pipeline.call(1).await;
//! assert_eq!(result, "Result: 2, 0");
//! ```
//!
//! 注意：
//! - [chain](Op::chain) 方法类似于 [map](Op::map) 方法，但它允许
//!   链接任意操作，只要它们实现 [Op] trait。
//! - [map] 是一个函数，用于初始化独立的 [Map](self::op::Map) 操作，无需现有管道/操作。
//!
//! 上面的管道可以可视化为以下 DAG：
//! ```text                 
//!           Input            
//!             │              
//!      ┌──────┴──────┐       
//!      ▼             ▼       
//! ┌─────────┐   ┌─────────┐  
//! │   op1   │   │   op2   │  
//! └────┬────┘   └────┬────┘  
//!      └──────┬──────┘       
//!             ▼              
//!        ┌─────────┐         
//!        │   op3   │         
//!        └────┬────┘         
//!             │              
//!             ▼              
//!          Output           
//! ```

pub mod agent_ops;
pub mod op;
pub mod try_op;
#[macro_use]
pub mod parallel;
#[macro_use]
pub mod conditional;

use std::future::Future;

pub use op::{Op, map, passthrough, then};
pub use try_op::TryOp;

use crate::{completion, extractor::Extractor, vector_store};

pub struct PipelineBuilder<E> {
    _error: std::marker::PhantomData<E>,
}

impl<E> PipelineBuilder<E> {
    /// Add a function to the current pipeline
    ///
    /// # Example
    /// ```rust
    /// use rig::pipeline::{self, Op};
    ///
    /// let pipeline = pipeline::new()
    ///    .map(|(x, y)| x + y)
    ///    .map(|z| format!("Result: {z}!"));
    ///
    /// let result = pipeline.call((1, 2)).await;
    /// assert_eq!(result, "Result: 3!");
    /// ```
    pub fn map<F, Input, Output>(self, f: F) -> op::Map<F, Input>
    where
        F: Fn(Input) -> Output + Send + Sync,
        Input: Send + Sync,
        Output: Send + Sync,
        Self: Sized,
    {
        op::Map::new(f)
    }

    /// Same as `map` but for asynchronous functions
    ///
    /// # Example
    /// ```rust
    /// use rig::pipeline::{self, Op};
    ///
    /// let pipeline = pipeline::new()
    ///     .then(|email: String| async move {
    ///         email.split('@').next().unwrap().to_string()
    ///     })
    ///     .then(|username: String| async move {
    ///         format!("Hello, {}!", username)
    ///     });
    ///
    /// let result = pipeline.call("bob@gmail.com".to_string()).await;
    /// assert_eq!(result, "Hello, bob!");
    /// ```
    pub fn then<F, Input, Fut>(self, f: F) -> op::Then<F, Input>
    where
        F: Fn(Input) -> Fut + Send + Sync,
        Input: Send + Sync,
        Fut: Future + Send + Sync,
        Fut::Output: Send + Sync,
        Self: Sized,
    {
        op::Then::new(f)
    }

    /// Add an arbitrary operation to the current pipeline.
    ///
    /// # Example
    /// ```rust
    /// use rig::pipeline::{self, Op};
    ///
    /// struct MyOp;
    ///
    /// impl Op for MyOp {
    ///     type Input = i32;
    ///     type Output = i32;
    ///
    ///     async fn call(&self, input: Self::Input) -> Self::Output {
    ///         input + 1
    ///     }
    /// }
    ///
    /// let pipeline = pipeline::new()
    ///    .chain(MyOp);
    ///
    /// let result = pipeline.call(1).await;
    /// assert_eq!(result, 2);
    /// ```
    pub fn chain<T>(self, op: T) -> T
    where
        T: Op,
        Self: Sized,
    {
        op
    }

    /// Chain a lookup operation to the current chain. The lookup operation expects the
    /// current chain to output a query string. The lookup operation will use the query to
    /// retrieve the top `n` documents from the index and return them with the query string.
    ///
    /// # Example
    /// ```rust
    /// use rig::pipeline::{self, Op};
    ///
    /// let pipeline = pipeline::new()
    ///     .lookup(index, 2)
    ///     .pipeline(|(query, docs): (_, Vec<String>)| async move {
    ///         format!("User query: {}\n\nTop documents:\n{}", query, docs.join("\n"))
    ///     });
    ///
    /// let result = pipeline.call("What is a flurbo?".to_string()).await;
    /// ```
    pub fn lookup<I, Input, Output>(self, index: I, n: usize) -> agent_ops::Lookup<I, Input, Output>
    where
        I: vector_store::VectorStoreIndex,
        Output: Send + Sync + for<'a> serde::Deserialize<'a>,
        Input: Into<String> + Send + Sync,
        // E: From<vector_store::VectorStoreError> + Send + Sync,
        Self: Sized,
    {
        agent_ops::Lookup::new(index, n)
    }

    /// Add a prompt operation to the current pipeline/op. The prompt operation expects the
    /// current pipeline to output a string. The prompt operation will use the string to prompt
    /// the given `agent`, which must implements the [Prompt](completion::Prompt) trait and return
    /// the response.
    ///
    /// # Example
    /// ```rust
    /// use rig::pipeline::{self, Op};
    ///
    /// let agent = &openai_client.agent("gpt-4").build();
    ///
    /// let pipeline = pipeline::new()
    ///    .map(|name| format!("Find funny nicknames for the following name: {name}!"))
    ///    .prompt(agent);
    ///
    /// let result = pipeline.call("Alice".to_string()).await;
    /// ```
    pub fn prompt<P, Input>(self, agent: P) -> agent_ops::Prompt<P, Input>
    where
        P: completion::Prompt,
        Input: Into<String> + Send + Sync,
        // E: From<completion::PromptError> + Send + Sync,
        Self: Sized,
    {
        agent_ops::Prompt::new(agent)
    }

    /// Add an extract operation to the current pipeline/op. The extract operation expects the
    /// current pipeline to output a string. The extract operation will use the given `extractor`
    /// to extract information from the string in the form of the type `T` and return it.
    ///
    /// # Example
    /// ```rust
    /// use rig::pipeline::{self, Op};
    ///
    /// #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
    /// struct Sentiment {
    ///     /// The sentiment score of the text (0.0 = negative, 1.0 = positive)
    ///     score: f64,
    /// }
    ///
    /// let extractor = &openai_client.extractor::<Sentiment>("gpt-4").build();
    ///
    /// let pipeline = pipeline::new()
    ///     .map(|text| format!("Analyze the sentiment of the following text: {text}!"))
    ///     .extract(extractor);
    ///
    /// let result: Sentiment = pipeline.call("I love ice cream!".to_string()).await?;
    /// assert!(result.score > 0.5);
    /// ```
    pub fn extract<M, Input, Output>(
        self,
        extractor: Extractor<M, Output>,
    ) -> agent_ops::Extract<M, Input, Output>
    where
        M: completion::CompletionModel,
        Output: schemars::JsonSchema + for<'a> serde::Deserialize<'a> + Send + Sync,
        Input: Into<String> + Send + Sync,
    {
        agent_ops::Extract::new(extractor)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    #[error("Failed to prompt agent: {0}")]
    PromptError(#[from] completion::PromptError),

    #[error("Failed to lookup documents: {0}")]
    LookupError(#[from] vector_store::VectorStoreError),
}

pub fn new() -> PipelineBuilder<ChainError> {
    PipelineBuilder {
        _error: std::marker::PhantomData,
    }
}

pub fn with_error<E>() -> PipelineBuilder<E> {
    PipelineBuilder {
        _error: std::marker::PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_ops::tests::{Foo, MockIndex, MockModel};
    use parallel::parallel;

    #[tokio::test]
    async fn test_prompt_pipeline() {
        let model = MockModel;

        let chain = super::new()
            .map(|input| format!("User query: {input}"))
            .prompt(model);

        let result = chain
            .call("What is a flurbo?")
            .await
            .expect("Failed to run chain");

        assert_eq!(result, "Mock response: User query: What is a flurbo?");
    }

    #[tokio::test]
    async fn test_prompt_pipeline_error() {
        let model = MockModel;

        let chain = super::with_error::<()>()
            .map(|input| format!("User query: {input}"))
            .prompt(model);

        let result = chain
            .try_call("What is a flurbo?")
            .await
            .expect("Failed to run chain");

        assert_eq!(result, "Mock response: User query: What is a flurbo?");
    }

    #[tokio::test]
    async fn test_lookup_pipeline() {
        let index = MockIndex;

        let chain = super::new()
            .lookup::<_, _, Foo>(index, 1)
            .map_ok(|docs| format!("Top documents:\n{}", docs[0].2.foo));

        let result = chain
            .try_call("What is a flurbo?")
            .await
            .expect("Failed to run chain");

        assert_eq!(result, "Top documents:\nbar");
    }

    #[tokio::test]
    async fn test_rag_pipeline() {
        let index = MockIndex;

        let chain = super::new()
            .chain(parallel!(
                passthrough(),
                agent_ops::lookup::<_, _, Foo>(index, 1),
            ))
            .map(|(query, maybe_docs)| match maybe_docs {
                Ok(docs) => format!("User query: {}\n\nTop documents:\n{}", query, docs[0].2.foo),
                Err(err) => format!("Error: {err}"),
            })
            .prompt(MockModel);

        let result = chain
            .call("What is a flurbo?")
            .await
            .expect("Failed to run chain");

        assert_eq!(
            result,
            "Mock response: User query: What is a flurbo?\n\nTop documents:\nbar"
        );
    }
}
