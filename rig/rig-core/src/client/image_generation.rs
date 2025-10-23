// 当启用 "image" 功能时编译此模块
#[cfg(feature = "image")]
// 定义图像生成模块
mod image {
    // 导入图像生成相关的客户端 trait
    use crate::client::{AsImageGeneration, ProviderClient};
    // 导入图像生成相关的类型和 trait
    use crate::image_generation::{
        ImageGenerationError, ImageGenerationModel, ImageGenerationModelDyn,
        ImageGenerationRequest, ImageGenerationResponse,
    };
    // 导入 Future trait 用于异步操作
    use std::future::Future;
    // 导入 Arc 用于原子引用计数
    use std::sync::Arc;

    /// 具有图像生成功能的提供商客户端。
    /// 客户端类型之间的转换需要 Clone。
    // 定义图像生成客户端 trait，继承 ProviderClient 和 Clone
    pub trait ImageGenerationClient: ProviderClient + Clone {
        /// 客户端使用的 ImageGenerationModel
        // 定义关联类型，必须是 ImageGenerationModel
        type ImageGenerationModel: ImageGenerationModel;

        /// 使用给定名称创建图像生成模型。
        ///
        /// # OpenAI 示例
        /// ```
        /// use rig::prelude::*;
        /// use rig::providers::openai::{Client, self};
        ///
        /// // 初始化 OpenAI 客户端
        /// let openai = Client::new("your-open-ai-api-key");
        ///
        /// let gpt4 = openai.image_generation_model(openai::DALL_E_3);
        /// ```
        // 定义创建图像生成模型的方法，接受模型名称字符串
        fn image_generation_model(&self, model: &str) -> Self::ImageGenerationModel;
    }

    // 定义动态图像生成客户端 trait，用于运行时多态
    pub trait ImageGenerationClientDyn: ProviderClient {
        /// 使用给定名称创建图像生成模型。
        // 定义动态创建图像生成模型的方法，返回装箱的动态类型
        fn image_generation_model<'a>(&self, model: &str) -> Box<dyn ImageGenerationModelDyn + 'a>;
    }

    // 为实现了 ImageGenerationClient 的类型自动实现 ImageGenerationClientDyn
    impl<T: ImageGenerationClient<ImageGenerationModel = M>, M: ImageGenerationModel + 'static>
        ImageGenerationClientDyn for T
    {
        // 实现动态图像生成模型创建方法
        fn image_generation_model<'a>(&self, model: &str) -> Box<dyn ImageGenerationModelDyn + 'a> {
            // 调用具体的图像生成模型创建方法并装箱为动态类型
            Box::new(self.image_generation_model(model))
        }
    }

    // 为实现了 ImageGenerationClientDyn 的类型自动实现 AsImageGeneration
    impl<T: ImageGenerationClientDyn + Clone + 'static> AsImageGeneration for T {
        // 实现图像生成转换方法
        fn as_image_generation(&self) -> Option<Box<dyn ImageGenerationClientDyn>> {
            // 克隆自身并装箱为动态类型
            Some(Box::new(self.clone()))
        }
    }

    /// 以 dyn 兼容的方式包装 ImageGenerationModel，用于 ImageGenerationRequestBuilder。
    // 派生 Clone trait 以便可以克隆此结构体
    #[derive(Clone)]
    // 定义图像生成模型句柄结构体，包含生命周期参数
    pub struct ImageGenerationModelHandle<'a> {
        // 内部使用 Arc 包装的动态图像生成模型，确保线程安全
        pub(crate) inner: Arc<dyn ImageGenerationModelDyn + 'a>,
    }
    // 为 ImageGenerationModelHandle 实现 ImageGenerationModel trait
    impl ImageGenerationModel for ImageGenerationModelHandle<'_> {
        // 定义响应类型为空元组
        type Response = ();

        // 实现图像生成方法
        fn image_generation(
            &self,
            request: ImageGenerationRequest,
        ) -> impl Future<
            Output = Result<ImageGenerationResponse<Self::Response>, ImageGenerationError>,
        > + Send {
            // 委托给内部的图像生成模型执行
            self.inner.image_generation(request)
        }
    }
}

// 当启用 "image" 功能时，导出图像模块的所有公共项
#[cfg(feature = "image")]
// 重新导出图像模块中的所有公共类型和 trait
pub use image::*;
