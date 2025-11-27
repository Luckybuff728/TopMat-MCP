//! MCP 工具注册表
//!
//! 使用编译时宏自动注册和管理所有 rig::tool::Tool 工具

use rmcp::ErrorData as McpError;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::OnceCell;
use tracing::info;
use futures::future::BoxFuture;

use super::tools::*;
use crate::register_all_mcp_tools;

/// 工具调用函数类型
type ToolCallFn = std::sync::Arc<dyn Fn(JsonValue) -> futures::future::BoxFuture<'static, Result<String, String>> + Send + Sync>;

/// 工具注册项
pub struct ToolEntry {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
    pub call_fn: ToolCallFn,
}

/// 工具注册表
pub struct ToolRegistry {
    /// 工具集合
    tools: HashMap<String, ToolEntry>,
}

// 静态单例实例
static TOOL_REGISTRY: OnceCell<Mutex<ToolRegistry>> = OnceCell::const_new();

impl ToolRegistry {
    /// 获取静态单例工具注册表（只初始化一次）
    pub async fn get_instance() -> &'static Mutex<ToolRegistry> {
        TOOL_REGISTRY.get_or_init(|| async {
            let mut registry = ToolRegistry {
                tools: HashMap::new(),
            };

            // 只在第一次创建时注册工具
            info!("开始注册 MCP 工具（单例模式）...");
            register_all_mcp_tools!(registry);
            info!("✅ 共注册了 {} 个工具（全局共享）", registry.tools.len());

            Mutex::new(registry)
        }).await
    }

        
    /// 动态注册单个工具（用于运行时添加工具）
    pub async fn register_tool<T>(&mut self) -> Result<(), String>
    where
        T: rig::tool::Tool<Output = String> + Default + Send + Sync + 'static,
        T::Args: serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        let tool = T::default();
        let definition = tool.definition("".to_string()).await;

        let call_fn: ToolCallFn = std::sync::Arc::new(move |args: JsonValue| {
            Box::pin(async move {
                // 反序列化参数
                let args: T::Args = serde_json::from_value(args)
                    .map_err(|e| format!("参数解析失败: {}", e))?;

                // 创建工具实例并调用
                let tool_instance = T::default();
                tool_instance.call(args).await
                    .map_err(|e| format!("工具调用失败: {}", e))
            })
        });

        let tool_name = definition.name.clone();
        self.tools.insert(
            tool_name.clone(),
            ToolEntry {
                name: definition.name,
                description: definition.description,
                input_schema: definition.parameters,
                call_fn,
            },
        );

        info!("✅ 动态注册工具: {}", tool_name);
        Ok(())
    }

    /// 批量注册工具（使用宏）
    pub fn register_tools_from_macro<F>(&mut self, f: F)
    where
        F: Fn(&mut Self),
    {
        f(self);
        info!("✅ 批量工具注册完成，当前工具总数: {}", self.tools.len());
    }

    /// 重新加载所有工具（先清空再注册）
    pub async fn reload_all_tools(&mut self) {
        let old_count = self.tools.len();
        self.tools.clear();
        info!("🗑️  清空了 {} 个旧工具", old_count);

        // 重新注册所有工具
        register_all_mcp_tools!(self);
        info!("🔄 重新注册完成，新工具总数: {}", self.tools.len());
    }

  
    /// 调用工具（使用静态单例）
    pub async fn call_tool(
        name: &str,
        arguments: JsonValue,
    ) -> Result<String, String> {
        let registry = Self::get_instance().await;
        let entry = {
            let registry = registry.lock().unwrap();
            registry.tools.get(name)
                .ok_or_else(|| format!("Unknown tool: {}", name))?
                .call_fn.clone()
        };

        info!("调用工具: {} (单例模式)", name);
        entry(arguments).await
    }

    /// 获取所有工具定义（使用单例实例）
    pub async fn get_tool_definitions() -> Vec<rmcp::model::Tool> {
        let registry = Self::get_instance().await;
        let registry = registry.lock().unwrap();

        registry.tools
            .iter()
            .map(|(name, entry)| rmcp::model::Tool {
                name: name.clone().into(),
                title: None,
                description: Some(entry.description.clone().into()),
                input_schema: std::sync::Arc::new(
                    entry.input_schema.as_object().unwrap().clone()
                ),
                annotations: None,
                icons: None,
                output_schema: None,
            })
            .collect()
    }
    
    /// 获取工具数量
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}
