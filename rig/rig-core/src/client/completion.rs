// 导入代理构建器类型
use crate::agent::AgentBuilder;
// 导入完成相关的客户端 trait
use crate::client::{AsCompletion, ProviderClient};
// 导入完成模型相关的类型和 trait
use crate::completion::{
    CompletionError, CompletionModel, CompletionModelDyn, CompletionRequest, CompletionResponse,
    GetTokenUsage,
};
// 导入提取器构建器
use crate::extractor::ExtractorBuilder;
// 导入流式完成响应类型
use crate::streaming::StreamingCompletionResponse;
// 导入 JSON Schema 宏
use schemars::JsonSchema;
// 导入序列化和反序列化宏
use serde::{Deserialize, Serialize};
// 导入 Future trait 用于异步操作
use std::future::Future;
// 导入 Arc 用于原子引用计数
use std::sync::Arc;

/// 具有完成功能的提供商客户端。
/// 客户端类型之间的转换需要 Clone。
// 定义完成客户端 trait，继承 ProviderClient 和 Clone
pub trait CompletionClient: ProviderClient + Clone {
    /// 客户端使用的 CompletionModel 类型。
    // 定义关联类型，必须是 CompletionModel
    type CompletionModel: CompletionModel;

    /// 使用给定名称创建完成模型。
    ///
    /// # OpenAI 示例
    /// ```
    /// use rig::prelude::*;
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let gpt4 = openai.completion_model(openai::GPT_4);
    /// ```
    // 定义创建完成模型的方法，接受模型名称字符串
    fn completion_model(&self, model: &str) -> Self::CompletionModel;

    /// 使用给定的完成模型创建代理构建器。
    ///
    /// # OpenAI 示例
    /// ```
    /// use rig::prelude::*;
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let agent = openai.agent(openai::GPT_4)
    ///    .preamble("You are comedian AI with a mission to make people laugh.")
    ///    .temperature(0.0)
    ///    .build();
    /// ```
    // 定义创建代理构建器的方法
    fn agent(&self, model: &str) -> AgentBuilder<Self::CompletionModel> {
        // 使用完成模型创建代理构建器
        AgentBuilder::new(self.completion_model(model))
    }

    /// 使用给定的完成模型创建提取器构建器。
    // 定义创建提取器构建器的方法，支持泛型类型 T
    fn extractor<T>(&self, model: &str) -> ExtractorBuilder<Self::CompletionModel, T>
    where
        // T 必须实现 JsonSchema、反序列化、序列化等 trait
        T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync,
    {
        // 使用完成模型创建提取器构建器
        ExtractorBuilder::new(self.completion_model(model))
    }
}

/// 以 dyn 兼容的方式包装 CompletionModel 以供 AgentBuilder 使用。
// 派生 Clone trait 以便可以克隆此结构体
#[derive(Clone)]
// 定义完成模型句柄结构体，包含生命周期参数
pub struct CompletionModelHandle<'a> {
    // 内部使用 Arc 包装的动态完成模型，确保线程安全
    pub inner: Arc<dyn CompletionModelDyn + 'a>,
}

// 为 CompletionModelHandle 实现 CompletionModel trait
impl CompletionModel for CompletionModelHandle<'_> {
    // 定义响应类型为空元组
    type Response = ();
    // 定义流式响应类型为空元组
    type StreamingResponse = ();

    // 实现完成方法
    fn completion(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = Result<CompletionResponse<Self::Response>, CompletionError>> + Send
    {
        // 委托给内部的完成模型执行
        self.inner.completion(request)
    }

    // 实现流式方法
    fn stream(
        &self,
        request: CompletionRequest,
    ) -> impl Future<
        Output = Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>,
    > + Send {
        // 委托给内部的完成模型执行流式操作
        self.inner.stream(request)
    }
}

// 定义动态完成客户端 trait，用于运行时多态
pub trait CompletionClientDyn: ProviderClient {
    /// 使用给定名称创建完成模型。
    // 定义动态创建完成模型的方法，返回装箱的动态类型
    fn completion_model<'a>(&self, model: &str) -> Box<dyn CompletionModelDyn + 'a>;

    /// 使用给定的完成模型创建代理构建器。
    // 定义动态创建代理构建器的方法
    fn agent<'a>(&self, model: &str) -> AgentBuilder<CompletionModelHandle<'a>>;
}

// 为实现了 CompletionClient 的类型自动实现 CompletionClientDyn
impl<T, M, R> CompletionClientDyn for T
where
    // T 必须实现 CompletionClient，其关联类型为 M
    T: CompletionClient<CompletionModel = M>,
    // M 必须实现 CompletionModel 且生命周期为 'static
    M: CompletionModel<StreamingResponse = R> + 'static,
    // R 必须实现 Clone、Unpin、GetTokenUsage 且生命周期为 'static
    R: Clone + Unpin + GetTokenUsage + 'static,
{
    // 实现动态完成模型创建方法
    fn completion_model<'a>(&self, model: &str) -> Box<dyn CompletionModelDyn + 'a> {
        // 调用具体的完成模型创建方法并装箱为动态类型
        Box::new(self.completion_model(model))
    }

    // 实现动态代理构建器创建方法
    fn agent<'a>(&self, model: &str) -> AgentBuilder<CompletionModelHandle<'a>> {
        // 使用 CompletionModelHandle 包装动态模型创建代理构建器
        AgentBuilder::new(CompletionModelHandle {
            // 将完成模型用 Arc 包装后存储在句柄中
            inner: Arc::new(self.completion_model(model)),
        })
    }
}

// 为实现了 CompletionClientDyn 的类型自动实现 AsCompletion
impl<T> AsCompletion for T
where
    // T 必须实现 CompletionClientDyn、Clone 且生命周期为 'static
    T: CompletionClientDyn + Clone + 'static,
{
    // 实现完成转换方法
    fn as_completion(&self) -> Option<Box<dyn CompletionClientDyn>> {
        // 克隆自身并装箱为动态类型
        Some(Box::new(self.clone()))
    }
}
