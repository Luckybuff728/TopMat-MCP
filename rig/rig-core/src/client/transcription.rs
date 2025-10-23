// 导入转录相关的客户端 trait
use crate::client::{AsTranscription, ProviderClient};
// 导入转录模型相关的类型和 trait
use crate::transcription::{
    TranscriptionError, TranscriptionModel, TranscriptionModelDyn, TranscriptionRequest,
    TranscriptionResponse,
};
// 导入 Arc 用于原子引用计数
use std::sync::Arc;

/// 具有转录功能的提供商客户端。
/// 客户端类型之间的转换需要 Clone。
// 定义转录客户端 trait，继承 ProviderClient 和 Clone
pub trait TranscriptionClient: ProviderClient + Clone {
    /// 客户端使用的 TranscriptionModel 类型
    // 定义关联类型，必须是 TranscriptionModel
    type TranscriptionModel: TranscriptionModel;

    /// 使用给定名称创建转录模型。
    ///
    /// # OpenAI 示例
    /// ```
    /// use rig::prelude::*;
    /// use rig::providers::openai::{Client, self};
    ///
    /// // 初始化 OpenAI 客户端
    /// let openai = Client::new("your-open-ai-api-key");
    ///
    /// let whisper = openai.transcription_model(openai::WHISPER_1);
    /// ```
    // 定义创建转录模型的方法，接受模型名称字符串
    fn transcription_model(&self, model: &str) -> Self::TranscriptionModel;
}

// 定义动态转录客户端 trait，用于运行时多态
pub trait TranscriptionClientDyn: ProviderClient {
    /// 使用给定名称创建转录模型。
    // 定义动态创建转录模型的方法，返回装箱的动态类型
    fn transcription_model<'a>(&self, model: &str) -> Box<dyn TranscriptionModelDyn + 'a>;
}

// 为实现了 TranscriptionClient 的类型自动实现 TranscriptionClientDyn
impl<M, T> TranscriptionClientDyn for T
where
    // T 必须实现 TranscriptionClient，其关联类型为 M
    T: TranscriptionClient<TranscriptionModel = M>,
    // M 必须实现 TranscriptionModel 且生命周期为 'static
    M: TranscriptionModel + 'static,
{
    // 实现动态转录模型创建方法
    fn transcription_model<'a>(&self, model: &str) -> Box<dyn TranscriptionModelDyn + 'a> {
        // 调用具体的转录模型创建方法并装箱为动态类型
        Box::new(self.transcription_model(model))
    }
}

// 为实现了 TranscriptionClientDyn 的类型自动实现 AsTranscription
impl<T> AsTranscription for T
where
    // T 必须实现 TranscriptionClientDyn、Clone 且生命周期为 'static
    T: TranscriptionClientDyn + Clone + 'static,
{
    // 实现转录转换方法
    fn as_transcription(&self) -> Option<Box<dyn TranscriptionClientDyn>> {
        // 克隆自身并装箱为动态类型
        Some(Box::new(self.clone()))
    }
}

/// 以 dyn 兼容的方式包装 TranscriptionModel，用于 TranscriptionRequestBuilder。
// 派生 Clone trait 以便可以克隆此结构体
#[derive(Clone)]
// 定义转录模型句柄结构体，包含生命周期参数
pub struct TranscriptionModelHandle<'a> {
    // 内部使用 Arc 包装的动态转录模型，确保线程安全
    pub inner: Arc<dyn TranscriptionModelDyn + 'a>,
}

// 为 TranscriptionModelHandle 实现 TranscriptionModel trait
impl TranscriptionModel for TranscriptionModelHandle<'_> {
    // 定义响应类型为空元组
    type Response = ();

    // 实现转录方法
    async fn transcription(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResponse<Self::Response>, TranscriptionError> {
        // 委托给内部的转录模型执行异步转录操作
        self.inner.transcription(request).await
    }
}
