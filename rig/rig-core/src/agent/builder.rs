// 导入集合和原子引用计数相关类型
use std::{collections::HashMap, sync::Arc};

// 导入完成模型和文档相关类型
use crate::{
    completion::{CompletionModel, Document},
    message::ToolChoice,
    tool::{Tool, ToolSet},
    vector_store::VectorStoreIndexDyn,
};

// 当启用 "rmcp" 功能时导入 RMCP 工具
#[cfg(feature = "rmcp")]
// 为文档生成添加功能条件
#[cfg_attr(docsrs, doc(cfg(feature = "rmcp")))]
// 导入 RMCP 工具类型
use crate::tool::rmcp::McpTool as RmcpTool;

// 导入父模块中的代理类型
use super::Agent;

/// 用于创建代理的构建器
///
/// # 示例
/// ```
/// use rig::{providers::openai, agent::AgentBuilder};
///
/// let openai = openai::Client::from_env();
///
/// let gpt4o = openai.completion_model("gpt-4o");
///
/// // 配置代理
/// let agent = AgentBuilder::new(model)
///     .preamble("System prompt")
///     .context("Context document 1")
///     .context("Context document 2")
///     .tool(tool1)
///     .tool(tool2)
///     .temperature(0.8)
///     .additional_params(json!({"foo": "bar"}))
///     .build();
/// ```
// 定义代理构建器结构体，支持泛型完成模型类型
pub struct AgentBuilder<M>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    /// 用于日志记录和调试的代理名称
    // 可选的代理名称，用于标识和调试
    name: Option<String>,
    /// 完成模型（例如：OpenAI 的 gpt-3.5-turbo-1106，Cohere 的 command-r）
    // 完成模型实例
    model: M,
    /// 系统提示
    // 可选的系统提示，定义代理的行为
    preamble: Option<String>,
    /// 代理始终可用的上下文文档
    // 静态上下文文档列表
    static_context: Vec<Document>,
    /// 代理始终可用的工具（按名称）
    // 静态工具名称列表
    static_tools: Vec<String>,
    /// 传递给模型的附加参数
    additional_params: Option<serde_json::Value>,
    /// 完成的最大 token 数
    max_tokens: Option<u64>,
    /// 向量存储列表，包含样本数量
    dynamic_context: Vec<(usize, Box<dyn VectorStoreIndexDyn>)>,
    /// 动态工具
    dynamic_tools: Vec<(usize, Box<dyn VectorStoreIndexDyn>)>,
    /// 模型的温度
    temperature: Option<f64>,
    /// 实际工具实现
    tools: ToolSet,
    /// 底层 LLM 是否应在提供响应之前强制使用工具。
    tool_choice: Option<ToolChoice>,
}

// 为代理构建器实现方法，支持泛型完成模型
impl<M> AgentBuilder<M>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    // 公共函数：创建新的代理构建器
    pub fn new(model: M) -> Self {
        // 返回新的代理构建器实例
        Self {
            // 初始化名称为空
            name: None,
            // 设置完成模型
            model,
            // 初始化前言为空
            preamble: None,
            // 初始化静态上下文为空向量
            static_context: vec![],
            // 初始化静态工具为空向量
            static_tools: vec![],
            // 初始化温度为 None
            temperature: None,
            // 初始化最大 token 数为 None
            max_tokens: None,
            // 初始化附加参数为 None
            additional_params: None,
            // 初始化动态上下文为空向量
            dynamic_context: vec![],
            // 初始化动态工具为空向量
            dynamic_tools: vec![],
            // 初始化工具集为默认值
            tools: ToolSet::default(),
            // 初始化工具选择为 None
            tool_choice: None,
        }
    }

    /// Set the name of the agent
    // 设置代理名称的公共方法
    pub fn name(mut self, name: &str) -> Self {
        // 将字符串转换为 String 并设置名称
        self.name = Some(name.into());
        // 返回修改后的构建器
        self
    }

    /// Set the system prompt
    // 设置系统提示的公共方法
    pub fn preamble(mut self, preamble: &str) -> Self {
        // 将字符串转换为 String 并设置前言
        self.preamble = Some(preamble.into());
        // 返回修改后的构建器
        self
    }

    /// Remove the system prompt
    // 移除系统提示的公共方法
    pub fn without_preamble(mut self) -> Self {
        // 将前言设置为 None
        self.preamble = None;
        // 返回修改后的构建器
        self
    }

    /// Append to the preamble of the agent
    // 追加到代理前言的公共方法
    pub fn append_preamble(mut self, doc: &str) -> Self {
        // 格式化新的前言，包含原有前言和新文档
        self.preamble = Some(format!(
            "{}\n{}",
            // 获取现有前言，如果为空则使用空字符串
            self.preamble.unwrap_or_else(|| "".into()),
            // 新文档内容
            doc
        ));
        // 返回修改后的构建器
        self
    }

    /// Add a static context document to the agent
    // 添加静态上下文文档的公共方法
    pub fn context(mut self, doc: &str) -> Self {
        // 将新文档添加到静态上下文列表
        self.static_context.push(Document {
            // 生成文档 ID，基于当前文档数量
            id: format!("static_doc_{}", self.static_context.len()),
            // 设置文档文本内容
            text: doc.into(),
            // 初始化附加属性为空映射
            additional_props: HashMap::new(),
        });
        // 返回修改后的构建器
        self
    }

    /// Add a static tool to the agent
    // 添加静态工具的公共方法
    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        // 获取工具名称
        let toolname = tool.name();
        // 将工具添加到工具集
        self.tools.add_tool(tool);
        // 将工具名称添加到静态工具列表
        self.static_tools.push(toolname);
        // 返回修改后的构建器
        self
    }

    // Add an MCP tool (from `rmcp`) to the agent
    // 为代理添加 MCP 工具（来自 `rmcp`）
    // 为文档生成添加功能条件
    #[cfg_attr(docsrs, doc(cfg(feature = "rmcp")))]
    // 当启用 "rmcp" 功能时
    #[cfg(feature = "rmcp")]
    // 添加 MCP 工具的公共方法
    pub fn rmcp_tool(mut self, tool: rmcp::model::Tool, client: rmcp::service::ServerSink) -> Self {
        // 克隆工具名称
        let toolname = tool.name.clone();
        // 将 MCP 工具添加到工具集
        self.tools.add_tool(RmcpTool::from_mcp_server(tool, client));
        // 将工具名称添加到静态工具列表
        self.static_tools.push(toolname.to_string());
        // 返回修改后的构建器
        self
    }

    /// Add some dynamic context to the agent. On each prompt, `sample` documents from the
    /// dynamic context will be inserted in the request.
    // 为代理添加一些动态上下文。在每个提示中，将从动态上下文中插入 `sample` 个文档到请求中
    // 添加动态上下文的公共方法
    pub fn dynamic_context(
        mut self,
        // 样本数量
        sample: usize,
        // 动态上下文实现，必须实现 VectorStoreIndexDyn trait 并且具有静态生命周期
        dynamic_context: impl VectorStoreIndexDyn + 'static,
    ) -> Self {
        // 将样本数量和动态上下文添加到动态上下文列表
        self.dynamic_context
            .push((sample, Box::new(dynamic_context)));
        // 返回修改后的构建器
        self
    }

    // 设置工具选择的公共方法
    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        // 设置工具选择
        self.tool_choice = Some(tool_choice);
        // 返回修改后的构建器
        self
    }

    /// Add some dynamic tools to the agent. On each prompt, `sample` tools from the
    /// dynamic toolset will be inserted in the request.
    // 为代理添加一些动态工具。在每个提示中，将从动态工具集中插入 `sample` 个工具到请求中
    // 添加动态工具的公共方法
    pub fn dynamic_tools(
        mut self,
        // 样本数量
        sample: usize,
        // 动态工具实现，必须实现 VectorStoreIndexDyn trait 并且具有静态生命周期
        dynamic_tools: impl VectorStoreIndexDyn + 'static,
        // 工具集
        toolset: ToolSet,
    ) -> Self {
        // 将样本数量和动态工具添加到动态工具列表
        self.dynamic_tools.push((sample, Box::new(dynamic_tools)));
        // 将工具集添加到工具集中
        self.tools.add_tools(toolset);
        // 返回修改后的构建器
        self
    }

    /// Set the temperature of the model
    // 设置模型温度的公共方法
    pub fn temperature(mut self, temperature: f64) -> Self {
        // 设置模型温度
        self.temperature = Some(temperature);
        // 返回修改后的构建器
        self
    }

    /// Set the maximum number of tokens for the completion
    // 设置完成的最大 token 数的公共方法
    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        // 设置最大 token 数
        self.max_tokens = Some(max_tokens);
        // 返回修改后的构建器
        self
    }

    /// Set additional parameters to be passed to the model
    // 设置传递给模型的附加参数的公共方法
    pub fn additional_params(mut self, params: serde_json::Value) -> Self {
        // 设置附加参数
        self.additional_params = Some(params);
        // 返回修改后的构建器
        self
    }

    /// Build the agent
    // 构建代理实例的公共方法
    pub fn build(self) -> Agent<M> {
        // 返回新的代理实例
        Agent {
            // 设置代理名称
            name: self.name,
            // 设置模型，使用 Arc 包装
            model: Arc::new(self.model),
            // 设置前言
            preamble: self.preamble,
            // 设置静态上下文
            static_context: self.static_context,
            // 设置静态工具
            static_tools: self.static_tools,
            // 设置温度
            temperature: self.temperature,
            // 设置最大 token 数
            max_tokens: self.max_tokens,
            // 设置附加参数
            additional_params: self.additional_params,
            // 设置工具选择
            tool_choice: self.tool_choice,
            // 设置动态上下文，使用 Arc 包装
            dynamic_context: Arc::new(self.dynamic_context),
            // 设置动态工具，使用 Arc 包装
            dynamic_tools: Arc::new(self.dynamic_tools),
            // 设置工具集，使用 Arc 包装
            tools: Arc::new(self.tools),
        }
    }
}
