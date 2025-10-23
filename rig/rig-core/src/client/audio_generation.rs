// 当启用 "audio" 功能时编译此模块
#[cfg(feature = "audio")]
// 定义音频生成模块
mod audio {
    // 导入音频生成相关的类型
    use crate::audio_generation::{
        AudioGenerationError, AudioGenerationModel, AudioGenerationModelDyn,
        AudioGenerationRequest, AudioGenerationResponse,
    };
    // 导入客户端相关的 trait
    use crate::client::{AsAudioGeneration, ProviderClient};
    // 导入 Future trait 用于异步操作
    use std::future::Future;
    // 导入 Arc 用于原子引用计数
    use std::sync::Arc;

    /// 具有音频生成功能的提供商客户端。
    /// 客户端类型之间的转换需要 Clone。
    // 定义音频生成客户端 trait，继承 ProviderClient 和 Clone
    pub trait AudioGenerationClient: ProviderClient + Clone {
        /// 客户端使用的 AudioGenerationModel
        // 定义关联类型，必须是 AudioGenerationModel
        type AudioGenerationModel: AudioGenerationModel;

        /// 使用给定名称创建音频生成模型。
        ///
        /// # 示例
        /// ```
        /// use rig::providers::openai::{Client, self};
        ///
        /// // 初始化 OpenAI 客户端
        /// let openai = Client::new("your-open-ai-api-key");
        ///
        /// let tts = openai.audio_generation_model(openai::TTS_1);
        /// ```
        // 定义创建音频生成模型的方法，接受模型名称字符串
        fn audio_generation_model(&self, model: &str) -> Self::AudioGenerationModel;
    }

    // 定义动态音频生成客户端 trait，用于运行时多态
    pub trait AudioGenerationClientDyn: ProviderClient {
        // 定义动态创建音频生成模型的方法，返回装箱的动态类型
        fn audio_generation_model<'a>(&self, model: &str) -> Box<dyn AudioGenerationModelDyn + 'a>;
    }

    // 为实现了 AudioGenerationClient 的类型自动实现 AudioGenerationClientDyn
    impl<T, M> AudioGenerationClientDyn for T
    where
        // T 必须实现 AudioGenerationClient，其关联类型为 M
        T: AudioGenerationClient<AudioGenerationModel = M>,
        // M 必须实现 AudioGenerationModel 且生命周期为 'static
        M: AudioGenerationModel + 'static,
    {
        // 实现动态音频生成模型创建方法
        fn audio_generation_model<'a>(&self, model: &str) -> Box<dyn AudioGenerationModelDyn + 'a> {
            // 调用具体的音频生成模型创建方法并装箱为动态类型
            Box::new(self.audio_generation_model(model))
        }
    }

    // 为实现了 AudioGenerationClientDyn 的类型自动实现 AsAudioGeneration
    impl<T> AsAudioGeneration for T
    where
        // T 必须实现 AudioGenerationClientDyn、Clone 且生命周期为 'static
        T: AudioGenerationClientDyn + Clone + 'static,
    {
        // 实现音频生成转换方法
        fn as_audio_generation(&self) -> Option<Box<dyn AudioGenerationClientDyn>> {
            // 克隆自身并装箱为动态类型
            Some(Box::new(self.clone()))
        }
    }

    /// 以 dyn 兼容的方式包装 AudioGenerationModel，用于 AudioGenerationRequestBuilder。
    // 派生 Clone trait 以便可以克隆此结构体
    #[derive(Clone)]
    // 定义音频生成模型句柄结构体，包含生命周期参数
    pub struct AudioGenerationModelHandle<'a> {
        // 内部使用 Arc 包装的动态音频生成模型，确保线程安全
        pub(crate) inner: Arc<dyn AudioGenerationModelDyn + 'a>,
    }

    // 为 AudioGenerationModelHandle 实现 AudioGenerationModel trait
    impl AudioGenerationModel for AudioGenerationModelHandle<'_> {
        // 定义响应类型为空元组
        type Response = ();

        // 实现音频生成方法
        fn audio_generation(
            &self,
            request: AudioGenerationRequest,
        ) -> impl Future<
            Output = Result<AudioGenerationResponse<Self::Response>, AudioGenerationError>,
        > + Send {
            // 委托给内部的音频生成模型执行
            self.inner.audio_generation(request)
        }
    }
}

// 当启用 "audio" 功能时，导出音频模块的所有公共项
#[cfg(feature = "audio")]
// 重新导出音频模块中的所有公共类型和 trait
pub use audio::*;
