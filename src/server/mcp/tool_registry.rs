//! MCP 工具注册表
//! 
//! 自动注册和管理所有 rig::tool::Tool 工具

use rmcp::ErrorData as McpError;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use tracing::info;
use rig::tool::Tool as RigTool;

use super::tools::*;

/// 工具调用函数类型
type ToolCallFn = Box<dyn Fn(JsonValue) -> futures::future::BoxFuture<'static, Result<String, String>> + Send + Sync>;

/// 工具注册项
pub struct ToolEntry {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
    pub call_fn: ToolCallFn,
}

/// 工具注册表
pub struct ToolRegistry {
    tools: HashMap<String, ToolEntry>,
}

impl ToolRegistry {
    /// 创建新的工具注册表并自动注册所有工具
    pub async fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        
        // 注册所有工具
        registry.register_all_tools().await;
        
        registry
    }
    
    /// 注册所有工具
    async fn register_all_tools(&mut self) {
        info!("开始注册 MCP 工具...");
        
        // 注册 ThinkTool
        self.register_think_tool().await;
        
        // 注册 TopPhiSimulator
        self.register_tophi_simulator().await;
        
        // 注册 MLPerformancePredictor
        self.register_ml_predictor().await;
        
        // 注册 HistoricalDataQuery
        self.register_historical_query().await;
        
        // 注册 ExperimentalDataReader
        self.register_experimental_reader().await;
        
        info!("共注册了 {} 个工具", self.tools.len());
    }
    
    /// 注册 ThinkTool
    async fn register_think_tool(&mut self) {
        let tool = ThinkTool;
        let definition = tool.definition("".to_string()).await;
        
        let call_fn: ToolCallFn = Box::new(|args: JsonValue| {
            Box::pin(async move {
                let thought = args.get("thought")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let tool = ThinkTool;
                let args = ThinkArgs { thought };
                
                match tool.call(args).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e.to_string()),
                }
            })
        });
        
        self.tools.insert(
            definition.name.clone(),
            ToolEntry {
                name: definition.name,
                description: definition.description,
                input_schema: definition.parameters,
                call_fn,
            },
        );
    }
    
    /// 注册 TopPhiSimulator
    async fn register_tophi_simulator(&mut self) {
        let tool = TopPhiSimulator;
        let definition = tool.definition("".to_string()).await;
        
        let call_fn: ToolCallFn = Box::new(|args: JsonValue| {
            Box::pin(async move {
                let composition = args.get("composition")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let process_params = args.get("process_params")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let structure = args.get("structure")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let tool = TopPhiSimulator;
                let args = TopPhiArgs {
                    composition,
                    process_params,
                    structure,
                };
                
                match tool.call(args).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e.to_string()),
                }
            })
        });
        
        self.tools.insert(
            definition.name.clone(),
            ToolEntry {
                name: definition.name,
                description: definition.description,
                input_schema: definition.parameters,
                call_fn,
            },
        );
    }
    
    /// 注册 MLPerformancePredictor
    async fn register_ml_predictor(&mut self) {
        let tool = MLPerformancePredictor;
        let definition = tool.definition("".to_string()).await;
        
        let call_fn: ToolCallFn = Box::new(|args: JsonValue| {
            Box::pin(async move {
                let composition = args.get("composition")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let process_params = args.get("process_params")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let structure = args.get("structure")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let simulation_result = args.get("simulation_result")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let tool = MLPerformancePredictor;
                let args = MLPredictorArgs {
                    composition,
                    process_params,
                    structure,
                    simulation_result,
                };
                
                match tool.call(args).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e.to_string()),
                }
            })
        });
        
        self.tools.insert(
            definition.name.clone(),
            ToolEntry {
                name: definition.name,
                description: definition.description,
                input_schema: definition.parameters,
                call_fn,
            },
        );
    }
    
    /// 注册 HistoricalDataQuery
    async fn register_historical_query(&mut self) {
        let tool = HistoricalDataQuery;
        let definition = tool.definition("".to_string()).await;
        
        let call_fn: ToolCallFn = Box::new(|args: JsonValue| {
            Box::pin(async move {
                let composition_range = args.get("composition_range")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let performance_target = args.get("performance_target")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let tool = HistoricalDataQuery;
                let args = HistoricalQueryArgs {
                    composition_range,
                    performance_target,
                };
                
                match tool.call(args).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e.to_string()),
                }
            })
        });
        
        self.tools.insert(
            definition.name.clone(),
            ToolEntry {
                name: definition.name,
                description: definition.description,
                input_schema: definition.parameters,
                call_fn,
            },
        );
    }
    
    /// 注册 ExperimentalDataReader
    async fn register_experimental_reader(&mut self) {
        let tool = ExperimentalDataReader;
        let definition = tool.definition("".to_string()).await;
        
        let call_fn: ToolCallFn = Box::new(|args: JsonValue| {
            Box::pin(async move {
                let sample_id = args.get("sample_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let tool = ExperimentalDataReader;
                let args = ExperimentalReaderArgs { sample_id };
                
                match tool.call(args).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e.to_string()),
                }
            })
        });
        
        self.tools.insert(
            definition.name.clone(),
            ToolEntry {
                name: definition.name,
                description: definition.description,
                input_schema: definition.parameters,
                call_fn,
            },
        );
    }
    
    /// 获取所有工具定义（用于 list_tools）
    pub fn get_tool_definitions(&self) -> Vec<rmcp::model::Tool> {
        self.tools.values().map(|entry| {
            rmcp::model::Tool {
                name: entry.name.clone().into(),
                title: None,
                description: Some(entry.description.clone().into()),
                input_schema: std::sync::Arc::new(
                    entry.input_schema.as_object().unwrap().clone()
                ),
                annotations: None,
                icons: None,
                output_schema: None,
            }
        }).collect()
    }
    
    /// 调用工具
    pub async fn call_tool(&self, name: &str, arguments: JsonValue) -> Result<String, String> {
        let entry = self.tools.get(name)
            .ok_or_else(|| format!("Unknown tool: {}", name))?;
        
        info!("调用工具: {}", name);
        (entry.call_fn)(arguments).await
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


