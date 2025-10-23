// 导入嵌入相关类型
use crate::Embed;
// 导入嵌入相关的客户端 trait
use crate::client::{AsEmbeddings, ProviderClient};
// 导入嵌入模型动态类型
use crate::embeddings::embedding::EmbeddingModelDyn;
// 导入嵌入模型和嵌入构建器类型
use crate::embeddings::{EmbeddingModel, EmbeddingsBuilder};

/// 具有嵌入功能的提供商客户端。
/// 客户端类型之间的转换需要 Clone。
// 定义嵌入客户端 trait，继承 ProviderClient 和 Clone
pub trait EmbeddingsClient: ProviderClient + Clone {
    /// 客户端使用的 EmbeddingModel 类型
    // 定义关联类型，必须是 EmbeddingModel
    type EmbeddingModel: EmbeddingModel;

    /// 使用给定名称创建嵌入模型。
    /// 注意：如果模型未知，将使用默认嵌入维度 0。
    /// 如果是这种情况，最好使用函数 `embedding_model_with_ndims`
    ///
    /// # 示例
    /// ```
    /// use rig::prelude::*;
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let embedding_model = openai.embedding_model(openai::TEXT_EMBEDDING_3_LARGE);
    /// ```
    // 定义创建嵌入模型的方法，接受模型名称字符串
    fn embedding_model(&self, model: &str) -> Self::EmbeddingModel;

    /// 使用给定名称和模型生成的嵌入维度数创建嵌入模型。
    ///
    /// # OpenAI 示例
    /// ```
    /// use rig::prelude::*;
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let embedding_model = openai.embedding_model("model-unknown-to-rig", 3072);
    /// ```
    // 定义创建指定维度嵌入模型的方法
    fn embedding_model_with_ndims(&self, model: &str, ndims: usize) -> Self::EmbeddingModel;

    /// 使用给定的嵌入模型创建嵌入构建器。
    ///
    /// # OpenAI 示例
    /// ```
    /// use rig::prelude::*;
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let embeddings = openai.embeddings(openai::TEXT_EMBEDDING_3_LARGE)
    ///     .simple_document("doc0", "Hello, world!")
    ///     .simple_document("doc1", "Goodbye, world!")
    ///     .build()
    ///     .await
    ///     .expect("Failed to embed documents");
    /// ```
    // 定义创建嵌入构建器的方法，支持泛型类型 D
    fn embeddings<D: Embed>(&self, model: &str) -> EmbeddingsBuilder<Self::EmbeddingModel, D> {
        // 使用嵌入模型创建嵌入构建器
        EmbeddingsBuilder::new(self.embedding_model(model))
    }

    /// 使用给定名称和模型生成的嵌入维度数创建嵌入构建器。
    ///
    /// # OpenAI 示例
    /// ```
    /// use rig::prelude::*;
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let embeddings = openai.embeddings_with_ndims(openai::TEXT_EMBEDDING_3_LARGE, 3072)
    ///     .simple_document("doc0", "Hello, world!")
    ///     .simple_document("doc1", "Goodbye, world!")
    ///     .build()
    ///     .await
    ///     .expect("Failed to embed documents");
    /// ```
    // 定义创建指定维度嵌入构建器的方法
    fn embeddings_with_ndims<D: Embed>(
        &self,
        model: &str,
        ndims: usize,
    ) -> EmbeddingsBuilder<Self::EmbeddingModel, D> {
        // 使用指定维度的嵌入模型创建嵌入构建器
        EmbeddingsBuilder::new(self.embedding_model_with_ndims(model, ndims))
    }
}

// 定义动态嵌入客户端 trait，用于运行时多态
pub trait EmbeddingsClientDyn: ProviderClient {
    /// 使用给定名称创建嵌入模型。
    /// 注意：如果模型未知，将使用默认嵌入维度 0。
    /// 如果是这种情况，最好使用函数 `embedding_model_with_ndims`
    // 定义动态创建嵌入模型的方法，返回装箱的动态类型
    fn embedding_model<'a>(&self, model: &str) -> Box<dyn EmbeddingModelDyn + 'a>;

    /// 使用给定名称和模型生成的嵌入维度数创建嵌入模型。
    // 定义动态创建指定维度嵌入模型的方法
    fn embedding_model_with_ndims<'a>(
        &self,
        model: &str,
        ndims: usize,
    ) -> Box<dyn EmbeddingModelDyn + 'a>;
}

// 为实现了 EmbeddingsClient 的类型自动实现 EmbeddingsClientDyn
impl<M, T> EmbeddingsClientDyn for T
where
    // T 必须实现 EmbeddingsClient，其关联类型为 M
    T: EmbeddingsClient<EmbeddingModel = M>,
    // M 必须实现 EmbeddingModel 且生命周期为 'static
    M: EmbeddingModel + 'static,
{
    // 实现动态嵌入模型创建方法
    fn embedding_model<'a>(&self, model: &str) -> Box<dyn EmbeddingModelDyn + 'a> {
        // 调用具体的嵌入模型创建方法并装箱为动态类型
        Box::new(self.embedding_model(model))
    }

    // 实现动态指定维度嵌入模型创建方法
    fn embedding_model_with_ndims<'a>(
        &self,
        model: &str,
        ndims: usize,
    ) -> Box<dyn EmbeddingModelDyn + 'a> {
        // 调用具体的指定维度嵌入模型创建方法并装箱为动态类型
        Box::new(self.embedding_model_with_ndims(model, ndims))
    }
}

// 为实现了 EmbeddingsClientDyn 的类型自动实现 AsEmbeddings
impl<T> AsEmbeddings for T
where
    // T 必须实现 EmbeddingsClientDyn、Clone 且生命周期为 'static
    T: EmbeddingsClientDyn + Clone + 'static,
{
    // 实现嵌入转换方法
    fn as_embeddings(&self) -> Option<Box<dyn EmbeddingsClientDyn>> {
        // 克隆自身并装箱为动态类型
        Some(Box::new(self.clone()))
    }
}
