//! 定义工具相关结构体和 trait 的模块。
//!
//! [Tool] trait 定义了创建可由 [Agents](crate::agent::Agent) 使用的工具的简单接口。
//!
//! [ToolEmbedding] trait 扩展了 [Tool] trait，允许可以存储在向量存储中并进行 RAG 的工具。
//!
//! [ToolSet] 结构体是可由 [Agent](crate::agent::Agent) 使用并可选择进行 RAG 的工具集合。

// 定义工具相关结构体和 trait 的模块
// Tool trait 定义了创建可由 Agents 使用的工具的简单接口
// ToolEmbedding trait 扩展了 Tool trait，允许可以存储在向量存储中并进行 RAG 的工具
// ToolSet 结构体是可由 Agent 使用并可选择进行 RAG 的工具集合

// 导入标准库的集合和 Pin 类型
use std::{collections::HashMap, pin::Pin};

// 导入异步 Future trait
use futures::Future;
// 导入序列化和反序列化 trait
use serde::{Deserialize, Serialize};

// 导入 crate 内部模块
use crate::{
    // 导入 completion 模块和 ToolDefinition
    completion::{self, ToolDefinition},
    // 导入 embeddings 模块的相关类型
    embeddings::{embed::EmbedError, tool::ToolSchema},
};

// 派生调试和错误 trait
#[derive(Debug, thiserror::Error)]
// 工具错误枚举
pub enum ToolError {
    /// 工具返回的错误
    // 工具调用错误，从实现了 Error trait 的类型转换而来
    #[error("ToolCallError: {0}")]
    ToolCallError(#[from] Box<dyn std::error::Error + Send + Sync>),

    // JSON 序列化/反序列化错误，从 serde_json::Error 转换而来
    #[error("JsonError: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// 表示简单 LLM 工具的 trait
///
/// # 示例
/// ```
/// use rig::{
///     completion::ToolDefinition,
///     tool::{ToolSet, Tool},
/// };
///
/// #[derive(serde::Deserialize)]
/// struct AddArgs {
///     x: i32,
///     y: i32,
/// }
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("Math error")]
/// struct MathError;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// struct Adder;
///
/// impl Tool for Adder {
///     const NAME: &'static str = "add";
///
///     type Error = MathError;
///     type Args = AddArgs;
///     type Output = i32;
///
///     async fn definition(&self, _prompt: String) -> ToolDefinition {
///         ToolDefinition {
///             name: "add".to_string(),
///             description: "Add x and y together".to_string(),
///             parameters: serde_json::json!({
///                 "type": "object",
///                 "properties": {
///                     "x": {
///                         "type": "number",
///                         "description": "The first number to add"
///                     },
///                     "y": {
///                         "type": "number",
///                         "description": "The second number to add"
///                     }
///                 }
///             })
///         }
///     }
///
///     async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
///         let result = args.x + args.y;
///         Ok(result)
///     }
/// }
/// ```
// 表示简单 LLM 工具的 trait
// 示例代码展示了如何实现一个加法工具
pub trait Tool: Sized + Send + Sync {
    /// The name of the tool. This name should be unique.
    // 工具的名称，这个名称应该是唯一的
    const NAME: &'static str;

    /// The error type of the tool.
    // 工具的错误类型，必须实现 Error trait 并且可以跨线程传递
    type Error: std::error::Error + Send + Sync + 'static;
    /// The arguments type of the tool.
    // 工具的参数类型，必须可以反序列化并且可以跨线程传递
    type Args: for<'a> Deserialize<'a> + Send + Sync;
    /// The output type of the tool.
    // 工具的输出类型，必须可以序列化
    type Output: Serialize;

    /// A method returning the name of the tool.
    // 返回工具名称的方法，默认实现返回 NAME 常量的字符串形式
    fn name(&self) -> String {
        // 将 NAME 常量转换为字符串
        Self::NAME.to_string()
    }

    /// A method returning the tool definition. The user prompt can be used to
    /// tailor the definition to the specific use case.
    // 返回工具定义的方法，用户提示可以用于根据特定用例定制定义
    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync;

    /// The tool execution method.
    /// Both the arguments and return value are a String since these values are meant to
    /// be the output and input of LLM models (respectively)
    // 工具执行方法
    // 参数和返回值都是字符串，因为这些值分别是 LLM 模型的输出和输入
    fn call(
        &self,
        args: Self::Args,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}

/// Trait that represents an LLM tool that can be stored in a vector store and RAGged
// 表示可以存储在向量存储中并进行 RAG 的 LLM 工具的 trait
pub trait ToolEmbedding: Tool {
    // 初始化错误类型，必须实现 Error trait 并且可以跨线程传递
    type InitError: std::error::Error + Send + Sync + 'static;

    /// Type of the tool' context. This context will be saved and loaded from the
    /// vector store when ragging the tool.
    /// This context can be used to store the tool's static configuration and local
    /// context.
    // 工具上下文的类型，这个上下文将在对工具进行 RAG 时从向量存储中保存和加载
    // 这个上下文可以用于存储工具的静态配置和本地上下文
    type Context: for<'a> Deserialize<'a> + Serialize;

    /// Type of the tool's state. This state will be passed to the tool when initializing it.
    /// This state can be used to pass runtime arguments to the tool such as clients,
    /// API keys and other configuration.
    // 工具状态的类型，这个状态将在初始化工具时传递给工具
    // 这个状态可以用于向工具传递运行时参数，如客户端、API 密钥和其他配置
    type State: Send;

    /// A method returning the documents that will be used as embeddings for the tool.
    /// This allows for a tool to be retrieved from multiple embedding "directions".
    /// If the tool will not be RAGged, this method should return an empty vector.
    // 返回将用作工具嵌入的文档的方法
    // 这允许从多个嵌入"方向"检索工具
    // 如果工具不会被 RAG，此方法应返回空向量
    fn embedding_docs(&self) -> Vec<String>;

    /// A method returning the context of the tool.
    // 返回工具上下文的方法
    fn context(&self) -> Self::Context;

    /// A method to initialize the tool from the context, and a state.
    // 从上下文和状态初始化工具的方法
    fn init(state: Self::State, context: Self::Context) -> Result<Self, Self::InitError>;
}

/// Wrapper trait to allow for dynamic dispatch of simple tools
// 包装器 trait，允许简单工具的动态分发
pub trait ToolDyn: Send + Sync {
    // 获取工具名称
    fn name(&self) -> String;

    // 获取工具定义，返回装箱的 Future
    fn definition(
        &self,
        prompt: String,
    ) -> Pin<Box<dyn Future<Output = ToolDefinition> + Send + Sync + '_>>;

    // 调用工具，返回装箱的 Future
    fn call(
        &self,
        args: String,
    ) -> Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send + '_>>;
}

// 为所有实现 Tool trait 的类型实现 ToolDyn trait
impl<T: Tool> ToolDyn for T {
    // 获取工具名称
    fn name(&self) -> String {
        // 调用 Tool trait 的 name 方法
        self.name()
    }

    // 获取工具定义
    fn definition(
        &self,
        prompt: String,
    ) -> Pin<Box<dyn Future<Output = ToolDefinition> + Send + Sync + '_>> {
        // 装箱 Tool trait 的 definition 方法
        Box::pin(<Self as Tool>::definition(self, prompt))
    }

    // 调用工具
    fn call(
        &self,
        args: String,
    ) -> Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send + '_>> {
        // 装箱异步闭包
        Box::pin(async move {
            // 尝试从 JSON 字符串反序列化参数
            match serde_json::from_str(&args) {
                // 如果成功，调用工具并处理结果
                Ok(args) => <Self as Tool>::call(self, args)
                    .await
                    // 将工具错误转换为 ToolError::ToolCallError
                    .map_err(|e| ToolError::ToolCallError(Box::new(e)))
                    // 将输出序列化为 JSON 字符串
                    .and_then(|output| {
                        serde_json::to_string(&output).map_err(ToolError::JsonError)
                    }),
                // 如果反序列化失败，返回 JSON 错误
                Err(e) => Err(ToolError::JsonError(e)),
            }
        })
    }
}

// 条件编译：仅在启用 rmcp 功能时编译
#[cfg(feature = "rmcp")]
// 文档属性：在文档中显示功能条件
#[cfg_attr(docsrs, doc(cfg(feature = "rmcp")))]
// RMCP 模块
pub mod rmcp {
    // 导入 crate 内部类型
    use crate::completion::ToolDefinition;
    use crate::tool::ToolDyn;
    use crate::tool::ToolError;
    // 导入 RMCP 相关类型
    use rmcp::model::RawContent;
    use std::borrow::Cow;
    use std::pin::Pin;

    // MCP 工具结构体
    pub struct McpTool {
        // MCP 工具定义
        definition: rmcp::model::Tool,
        // MCP 服务客户端
        client: rmcp::service::ServerSink,
    }

    // MCP 工具实现
    impl McpTool {
        // 从 MCP 服务器创建 MCP 工具
        pub fn from_mcp_server(
            definition: rmcp::model::Tool,
            client: rmcp::service::ServerSink,
        ) -> Self {
            // 创建新的 MCP 工具实例
            Self { definition, client }
        }
    }

    // 为 &rmcp::model::Tool 实现 From trait，转换为 ToolDefinition
    impl From<&rmcp::model::Tool> for ToolDefinition {
        fn from(val: &rmcp::model::Tool) -> Self {
            // 创建 ToolDefinition 实例
            Self {
                // 设置工具名称
                name: val.name.to_string(),
                // 设置工具描述，如果没有描述则使用空字符串
                description: val.description.clone().unwrap_or(Cow::from("")).to_string(),
                // 设置工具参数模式
                parameters: val.schema_as_json_value(),
            }
        }
    }

    // 为 rmcp::model::Tool 实现 From trait，转换为 ToolDefinition
    impl From<rmcp::model::Tool> for ToolDefinition {
        fn from(val: rmcp::model::Tool) -> Self {
            // 创建 ToolDefinition 实例
            Self {
                // 设置工具名称
                name: val.name.to_string(),
                // 设置工具描述，如果没有描述则使用空字符串
                description: val.description.clone().unwrap_or(Cow::from("")).to_string(),
                // 设置工具参数模式
                parameters: val.schema_as_json_value(),
            }
        }
    }

    // 派生调试和错误 trait
    #[derive(Debug, thiserror::Error)]
    // MCP 工具错误
    #[error("MCP tool error: {0}")]
    pub struct McpToolError(String);

    // 为 McpToolError 实现 From trait，转换为 ToolError
    impl From<McpToolError> for ToolError {
        fn from(e: McpToolError) -> Self {
            // 将 MCP 工具错误转换为工具调用错误
            ToolError::ToolCallError(Box::new(e))
        }
    }

    // 为 McpTool 实现 ToolDyn trait
    impl ToolDyn for McpTool {
        // 获取工具名称
        fn name(&self) -> String {
            // 返回工具定义的名称
            self.definition.name.to_string()
        }

        // 获取工具定义
        fn definition(
            &self,
            _prompt: String,
        ) -> Pin<Box<dyn Future<Output = ToolDefinition> + Send + Sync + '_>> {
            // 装箱异步闭包
            Box::pin(async move {
                // 创建工具定义
                ToolDefinition {
                    // 设置工具名称
                    name: self.definition.name.to_string(),
                    // 设置工具描述，如果没有描述则使用空字符串
                    description: self
                        .definition
                        .description
                        .clone()
                        .unwrap_or(Cow::from(""))
                        .to_string(),
                    // 设置工具参数模式，如果转换失败则使用默认值
                    parameters: serde_json::to_value(&self.definition.input_schema)
                        .unwrap_or_default(),
                }
            })
        }

        // 调用工具
        fn call(
            &self,
            args: String,
        ) -> Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send + '_>> {
            // 克隆工具名称
            let name = self.definition.name.clone();
            // 从 JSON 字符串解析参数，如果失败则使用默认值
            let arguments = serde_json::from_str(&args).unwrap_or_default();

            // 装箱异步闭包
            Box::pin(async move {
                // 调用 MCP 工具
                let result = self
                    .client
                    .call_tool(rmcp::model::CallToolRequestParam { name, arguments })
                    .await
                    // 如果调用失败，返回 MCP 工具错误
                    .map_err(|e| McpToolError(format!("Tool returned an error: {e}")))?;

                // 检查是否有错误
                if let Some(true) = result.is_error {
                    // 提取错误消息
                    let error_msg = result
                        .content
                        .into_iter()
                        .map(|x| x.raw.as_text().map(|y| y.to_owned()))
                        .map(|x| x.map(|x| x.clone().text))
                        .collect::<Option<Vec<String>>>();

                    // 连接错误消息
                    let error_message = error_msg.map(|x| x.join("\n"));
                    // 如果有错误消息，返回错误
                    if let Some(error_message) = error_message {
                        return Err(McpToolError(error_message).into());
                    } else {
                        // 如果没有错误消息，返回默认错误
                        return Err(McpToolError("No message returned".to_string()).into());
                    }
                };

                // 处理成功结果，将内容转换为字符串
                Ok(result
                    .content
                    .into_iter()
                    .map(|c| match c.raw {
                        // 处理文本内容
                        rmcp::model::RawContent::Text(raw) => raw.text,
                        // 处理图像内容，格式化为 data URL
                        rmcp::model::RawContent::Image(raw) => {
                            format!("data:{};base64,{}", raw.mime_type, raw.data)
                        }
                        // 处理资源内容
                        rmcp::model::RawContent::Resource(raw) => match raw.resource {
                            // 处理文本资源
                            rmcp::model::ResourceContents::TextResourceContents {
                                uri,
                                mime_type,
                                text,
                                ..
                            } => {
                                format!(
                                    "{mime_type}{uri}:{text}",
                                    mime_type = mime_type
                                        .map(|m| format!("data:{m};"))
                                        .unwrap_or_default(),
                                )
                            }
                            // 处理二进制资源
                            rmcp::model::ResourceContents::BlobResourceContents {
                                uri,
                                mime_type,
                                blob,
                                ..
                            } => format!(
                                "{mime_type}{uri}:{blob}",
                                mime_type = mime_type
                                    .map(|m| format!("data:{m};"))
                                    .unwrap_or_default(),
                            ),
                        },
                        // 音频内容暂未实现
                        RawContent::Audio(_) => {
                            unimplemented!("Support for audio results from an MCP tool is currently unimplemented. Come back later!")
                        }
                        // 其他未支持的类型
                        thing => {
                            unimplemented!("Unsupported type found: {thing:?}")
                        }
                    })
                    // 收集所有内容为单个字符串
                    .collect::<String>())
            })
        }
    }
}

/// Wrapper trait to allow for dynamic dispatch of raggable tools
// 包装器 trait，允许可 RAG 工具的动态分发
pub trait ToolEmbeddingDyn: ToolDyn {
    // 获取工具上下文，返回 JSON 值
    fn context(&self) -> serde_json::Result<serde_json::Value>;

    // 获取工具嵌入文档
    fn embedding_docs(&self) -> Vec<String>;
}

// 为所有实现 ToolEmbedding trait 的类型实现 ToolEmbeddingDyn trait
impl<T> ToolEmbeddingDyn for T
where
    // T 必须实现 ToolEmbedding trait
    T: ToolEmbedding,
{
    // 获取工具上下文
    fn context(&self) -> serde_json::Result<serde_json::Value> {
        // 将工具上下文序列化为 JSON 值
        serde_json::to_value(self.context())
    }

    // 获取工具嵌入文档
    fn embedding_docs(&self) -> Vec<String> {
        // 调用 ToolEmbedding trait 的 embedding_docs 方法
        self.embedding_docs()
    }
}

// 工具类型枚举，用于区分简单工具和可嵌入工具
pub(crate) enum ToolType {
    // 简单工具，包装在 ToolDyn trait 对象中
    Simple(Box<dyn ToolDyn>),
    // 可嵌入工具，包装在 ToolEmbeddingDyn trait 对象中
    Embedding(Box<dyn ToolEmbeddingDyn>),
}

// 工具类型实现
impl ToolType {
    // 获取工具名称
    pub fn name(&self) -> String {
        match self {
            // 简单工具的名称
            ToolType::Simple(tool) => tool.name(),
            // 可嵌入工具的名称
            ToolType::Embedding(tool) => tool.name(),
        }
    }

    // 获取工具定义
    pub async fn definition(&self, prompt: String) -> ToolDefinition {
        match self {
            // 简单工具的定义
            ToolType::Simple(tool) => tool.definition(prompt).await,
            // 可嵌入工具的定义
            ToolType::Embedding(tool) => tool.definition(prompt).await,
        }
    }

    // 调用工具
    pub async fn call(&self, args: String) -> Result<String, ToolError> {
        match self {
            // 调用简单工具
            ToolType::Simple(tool) => tool.call(args).await,
            // 调用可嵌入工具
            ToolType::Embedding(tool) => tool.call(args).await,
        }
    }
}

// 派生调试和错误 trait
#[derive(Debug, thiserror::Error)]
// 工具集错误枚举
pub enum ToolSetError {
    /// Error returned by the tool
    // 工具调用错误，从 ToolError 转换而来
    #[error("ToolCallError: {0}")]
    ToolCallError(#[from] ToolError),

    // 工具未找到错误
    #[error("ToolNotFoundError: {0}")]
    ToolNotFoundError(String),

    // TODO: Revisit this
    // JSON 序列化/反序列化错误，从 serde_json::Error 转换而来
    #[error("JsonError: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// A struct that holds a set of tools
// 保存工具集合的结构体
#[derive(Default)]
pub struct ToolSet {
    // 工具映射，键为工具名称，值为工具类型
    pub(crate) tools: HashMap<String, ToolType>,
}

// 工具集实现
impl ToolSet {
    /// Create a new ToolSet from a list of tools
    // 从工具列表创建新的工具集
    pub fn from_tools(tools: Vec<impl ToolDyn + 'static>) -> Self {
        // 创建默认工具集
        let mut toolset = Self::default();
        // 遍历工具列表并添加到工具集
        tools.into_iter().for_each(|tool| {
            toolset.add_tool(tool);
        });
        // 返回工具集
        toolset
    }

    /// Create a toolset builder
    // 创建工具集构建器
    pub fn builder() -> ToolSetBuilder {
        ToolSetBuilder::default()
    }

    /// Check if the toolset contains a tool with the given name
    // 检查工具集是否包含指定名称的工具
    pub fn contains(&self, toolname: &str) -> bool {
        self.tools.contains_key(toolname)
    }

    /// Add a tool to the toolset
    // 向工具集添加工具
    pub fn add_tool(&mut self, tool: impl ToolDyn + 'static) {
        self.tools
            .insert(tool.name(), ToolType::Simple(Box::new(tool)));
    }

    // 从工具集删除工具
    pub fn delete_tool(&mut self, tool_name: &str) {
        let _ = self.tools.remove(tool_name);
    }

    /// Merge another toolset into this one
    // 将另一个工具集合并到此工具集
    pub fn add_tools(&mut self, toolset: ToolSet) {
        self.tools.extend(toolset.tools);
    }

    // 获取指定名称的工具
    pub(crate) fn get(&self, toolname: &str) -> Option<&ToolType> {
        self.tools.get(toolname)
    }

    // 获取所有工具的定义
    pub async fn get_tool_definitions(&self) -> Result<Vec<ToolDefinition>, ToolSetError> {
        // 创建定义向量
        let mut defs = Vec::new();
        // 遍历所有工具并获取定义
        for tool in self.tools.values() {
            let def = tool.definition(String::new()).await;
            defs.push(def);
        }
        // 返回定义列表
        Ok(defs)
    }

    /// Call a tool with the given name and arguments
    // 使用给定名称和参数调用工具
    pub async fn call(&self, toolname: &str, args: String) -> Result<String, ToolSetError> {
        // 检查工具是否存在
        if let Some(tool) = self.tools.get(toolname) {
            // 记录工具调用信息
            tracing::info!(target: "rig",
                "Calling tool {toolname} with args:\n{}",
                serde_json::to_string_pretty(&args).unwrap()
            );
            // 调用工具并返回结果
            Ok(tool.call(args).await?)
        } else {
            // 如果工具不存在，返回错误
            Err(ToolSetError::ToolNotFoundError(toolname.to_string()))
        }
    }

    /// Get the documents of all the tools in the toolset
    // 获取工具集中所有工具的文档
    pub async fn documents(&self) -> Result<Vec<completion::Document>, ToolSetError> {
        // 创建文档向量
        let mut docs = Vec::new();
        // 遍历所有工具
        for tool in self.tools.values() {
            match tool {
                // 处理简单工具
                ToolType::Simple(tool) => {
                    docs.push(completion::Document {
                        // 设置文档 ID 为工具名称
                        id: tool.name(),
                        // 格式化工具文档内容
                        text: format!(
                            "\
                            Tool: {}\n\
                            Definition: \n\
                            {}\
                        ",
                            tool.name(),
                            serde_json::to_string_pretty(&tool.definition("".to_string()).await)?
                        ),
                        // 设置额外的属性为空映射
                        additional_props: HashMap::new(),
                    });
                }
                // 处理可嵌入工具
                ToolType::Embedding(tool) => {
                    docs.push(completion::Document {
                        // 设置文档 ID 为工具名称
                        id: tool.name(),
                        // 格式化工具文档内容
                        text: format!(
                            "\
                            Tool: {}\n\
                            Definition: \n\
                            {}\
                        ",
                            tool.name(),
                            serde_json::to_string_pretty(&tool.definition("".to_string()).await)?
                        ),
                        // 设置额外的属性为空映射
                        additional_props: HashMap::new(),
                    });
                }
            }
        }
        // 返回文档列表
        Ok(docs)
    }

    /// Convert tools in self to objects of type ToolSchema.
    /// This is necessary because when adding tools to the EmbeddingBuilder because all
    /// documents added to the builder must all be of the same type.
    // 将工具集中的工具转换为 ToolSchema 对象
    // 这是必要的，因为当向 EmbeddingBuilder 添加工具时，添加到构建器的所有文档必须都是相同类型
    pub fn schemas(&self) -> Result<Vec<ToolSchema>, EmbedError> {
        // 过滤出可嵌入工具并转换为 ToolSchema
        self.tools
            .values()
            .filter_map(|tool_type| {
                // 只处理可嵌入工具
                if let ToolType::Embedding(tool) = tool_type {
                    Some(ToolSchema::try_from(&**tool))
                } else {
                    None
                }
            })
            // 收集结果
            .collect::<Result<Vec<_>, _>>()
    }
}

// 派生默认 trait
#[derive(Default)]
// 工具集构建器结构体
pub struct ToolSetBuilder {
    // 工具类型向量
    tools: Vec<ToolType>,
}

// 工具集构建器实现
impl ToolSetBuilder {
    // 添加静态工具
    pub fn static_tool(mut self, tool: impl ToolDyn + 'static) -> Self {
        self.tools.push(ToolType::Simple(Box::new(tool)));
        self
    }

    // 添加动态工具
    pub fn dynamic_tool(mut self, tool: impl ToolEmbeddingDyn + 'static) -> Self {
        self.tools.push(ToolType::Embedding(Box::new(tool)));
        self
    }

    // 构建工具集
    pub fn build(self) -> ToolSet {
        ToolSet {
            // 将工具向量转换为以名称为键的映射
            tools: self
                .tools
                .into_iter()
                .map(|tool| (tool.name(), tool))
                .collect(),
        }
    }
}

// 条件编译：仅在测试时编译
#[cfg(test)]
mod tests {
    // 导入 serde_json 的 json 宏
    use serde_json::json;

    // 导入父模块的所有内容
    use super::*;

    // 获取测试工具集的函数
    fn get_test_toolset() -> ToolSet {
        // 创建默认工具集
        let mut toolset = ToolSet::default();

        // 派生反序列化 trait
        #[derive(Deserialize)]
        // 操作参数结构体
        struct OperationArgs {
            // 第一个数字
            x: i32,
            // 第二个数字
            y: i32,
        }

        // 派生调试和错误 trait
        #[derive(Debug, thiserror::Error)]
        // 数学错误
        #[error("Math error")]
        struct MathError;

        // 派生反序列化和序列化 trait
        #[derive(Deserialize, Serialize)]
        // 加法工具结构体
        struct Adder;

        // 为 Adder 实现 Tool trait
        impl Tool for Adder {
            // 工具名称
            const NAME: &'static str = "add";
            // 错误类型
            type Error = MathError;
            // 参数类型
            type Args = OperationArgs;
            // 输出类型
            type Output = i32;

            // 获取工具定义
            async fn definition(&self, _prompt: String) -> ToolDefinition {
                ToolDefinition {
                    name: "add".to_string(),
                    description: "Add x and y together".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "x": {
                                "type": "number",
                                "description": "The first number to add"
                            },
                            "y": {
                                "type": "number",
                                "description": "The second number to add"
                            }
                        },
                        "required": ["x", "y"]
                    }),
                }
            }

            // 调用工具
            async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
                let result = args.x + args.y;
                Ok(result)
            }
        }

        // 派生反序列化和序列化 trait
        #[derive(Deserialize, Serialize)]
        // 减法工具结构体
        struct Subtract;

        // 为 Subtract 实现 Tool trait
        impl Tool for Subtract {
            // 工具名称
            const NAME: &'static str = "subtract";
            // 错误类型
            type Error = MathError;
            // 参数类型
            type Args = OperationArgs;
            // 输出类型
            type Output = i32;

            // 获取工具定义
            async fn definition(&self, _prompt: String) -> ToolDefinition {
                serde_json::from_value(json!({
                    "name": "subtract",
                    "description": "Subtract y from x (i.e.: x - y)",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "x": {
                                "type": "number",
                                "description": "The number to subtract from"
                            },
                            "y": {
                                "type": "number",
                                "description": "The number to subtract"
                            }
                        },
                        "required": ["x", "y"]
                    }
                }))
                .expect("Tool Definition")
            }

            // 调用工具
            async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
                let result = args.x - args.y;
                Ok(result)
            }
        }

        // 添加工具到工具集
        toolset.add_tool(Adder);
        toolset.add_tool(Subtract);
        // 返回工具集
        toolset
    }

    // 异步测试：获取工具定义
    #[tokio::test]
    async fn test_get_tool_definitions() {
        // 获取测试工具集
        let toolset = get_test_toolset();
        // 获取工具定义
        let tools = toolset.get_tool_definitions().await.unwrap();
        // 验证工具数量为 2
        assert_eq!(tools.len(), 2);
    }

    // 测试：工具删除
    #[test]
    fn test_tool_deletion() {
        // 获取测试工具集
        let mut toolset = get_test_toolset();
        // 验证初始工具数量为 2
        assert_eq!(toolset.tools.len(), 2);
        // 删除 add 工具
        toolset.delete_tool("add");
        // 验证 add 工具不存在
        assert!(!toolset.contains("add"));
        // 验证工具数量为 1
        assert_eq!(toolset.tools.len(), 1);
    }
}
