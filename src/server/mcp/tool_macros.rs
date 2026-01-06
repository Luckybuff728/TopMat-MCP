//! MCP工具自动注册宏
//!
//! 提供编译时自动注册MCP工具的宏系统，减少重复代码并提高开发效率

/// 自动注册MCP工具的宏
///
/// # 语法
///
/// ```rust
/// register_mcp_tools!(registry,
///     ToolType {
///         args_type: ToolArgsType,
///         constructor: ToolConstructor
///     },
/// );
/// ```
///
/// # 参数
///
/// - `registry`: 可变引用的ToolRegistry实例
/// - `ToolType`: 实现了`rig::tool::Tool`的类型
/// - `ToolArgsType`: 工具的参数类型
/// - `ToolConstructor`: 工具构造函数（通常是类型名本身）
#[macro_export]
macro_rules! register_mcp_tools {
    (
        $registry:expr,
        $(
            $tool_type:ty {
                args_type: $args_type:ty,
                constructor: $constructor:expr
            }
        ),* $(,)?
    ) => {
        $(
            {
                use rig::tool::Tool;

                // 创建工具实例
                let tool = $constructor;

                // 获取工具定义
                let definition = tool.definition("".to_string()).await;

                // 创建工具调用函数
                let call_fn: $crate::server::mcp::tool_registry::ToolCallFn =
                    std::sync::Arc::new(|args: JsonValue| {

                        Box::pin(async move {
                            // 反序列化参数
                            let args: $args_type = serde_json::from_value(args)
                                .map_err(|e| format!("参数解析失败: {}", e))?;

                            // 创建新的工具实例（使用默认构造函数）
                            let tool_instance = <$tool_type>::default();

                            // 调用工具
                            tool_instance.call(args).await
                                .map_err(|e| format!("工具调用失败: {}", e))
                        })
                    });

                // 注册工具
                let tool_name = definition.name.clone();
                $registry.tools.insert(
                    tool_name.clone(),
                    $crate::server::mcp::tool_registry::ToolEntry {
                        name: definition.name,
                        description: definition.description,
                        input_schema: definition.parameters,
                        call_fn,
                    },
                );

                info!("✓ 注册工具: {}", tool_name);
            }
        )*
    };
}

/// 批量注册所有已知工具的便捷宏
///
/// 这个宏包含了项目中所有的MCP工具，新工具应该在这里添加
#[macro_export]
macro_rules! register_all_mcp_tools {
    ($registry:expr) => {
        $crate::register_mcp_tools!(
            $registry,
            // ThinkTool {
            //     args_type: ThinkArgs,
            //     constructor: ThinkTool
            // },
            // TopPhiSimulator {
            //     args_type: TopPhiArgs,
            //     constructor: TopPhiSimulator
            // },
            // MLPerformancePredictor {
            //     args_type: MLPredictorArgs,
            //     constructor: MLPerformancePredictor
            // },
            // HistoricalDataQuery {
            //     args_type: HistoricalQueryArgs,
            //     constructor: HistoricalDataQuery
            // },
            // ExperimentalDataReader {
            //     args_type: ExperimentalReaderArgs,
            //     constructor: ExperimentalDataReader
            // },

            // Calpha Mesh 工具
            SubmitPointTask {
                args_type: PointTaskParams,
                constructor: SubmitPointTask
            },
            SubmitLineTask {
                args_type: LineTaskParams,
                constructor: SubmitLineTask
            },
            SubmitScheilTask {
                args_type: ScheilTaskParams,
                constructor: SubmitScheilTask
            },
            GetTaskStatus {
                args_type: TaskIdParams,
                constructor: GetTaskStatus
            },
            ListTasks {
                args_type: ListTasksParams,
                constructor: ListTasks
            },
            // ONNX Service 工具
            // OnnxHealthCheck {
            //     args_type: EmptyParams,
            //     constructor: OnnxHealthCheck
            // },
            OnnxModelsList {
                args_type: EmptyParams,
                constructor: OnnxModelsList
            },
            OnnxScanModels {
                args_type: EmptyParams,
                constructor: OnnxScanModels
            },
            OnnxUnloadModel {
                args_type: UnloadModelRequest,
                constructor: OnnxUnloadModel
            },
            OnnxModelInference {
                args_type: InferenceRequest,
                constructor: OnnxModelInference
            },
            OnnxGetModelConfig {
                args_type: UuidParams,
                constructor: OnnxGetModelConfig
            },
            OnnxSayHello {
                args_type: EmptyParams,
                constructor: OnnxSayHello
            },
            // RAG 知识库检索工具
            SteelRagQuery {
                args_type: DifyQueryRequest,
                constructor: SteelRagQuery
            },
            CementedCarbideRagQuery {
                args_type: DifyQueryRequest,
                constructor: CementedCarbideRagQuery
            },
            AlIdmeWorkflow {
                args_type: DifyQueryRequest,
                constructor: AlIdmeWorkflow
            },
            // Phase Field 相场模拟工具
            SubmitSpinodalDecompositionTask {
                args_type: SpinodalDecompositionRequest,
                constructor: SubmitSpinodalDecompositionTask
            },
            SubmitPvdSimulationTask {
                args_type: PvdSimulationRequest,
                constructor: SubmitPvdSimulationTask
            },
            GetTaskList {
                args_type: TaskListParams,
                constructor: GetTaskList
            },
            PhaseFieldGetTaskStatus {
                args_type: PhaseFieldTaskIdParams,
                constructor: PhaseFieldGetTaskStatus
            },
            StopTask {
                args_type: PhaseFieldTaskIdParams,
                constructor: StopTask
            },
            ProbeTaskFiles {
                args_type: PhaseFieldTaskIdParams,
                constructor: ProbeTaskFiles
            },
            RetrieveFile {
                args_type: FileRetrieveParams,
                constructor: RetrieveFile
            },
        );
    };
}

/// 动态工具注册宏 - 用于支持从配置或其他源加载工具
#[macro_export]
macro_rules! create_tool_factory {
    ($tool_type:ty, $args_type:ty) => {
        {
            std::sync::Arc::new(move |args: JsonValue| -> BoxFuture<'static, Result<String, String>> {
                Box::pin(async move {
                    // 反序列化参数
                    let args: $args_type = serde_json::from_value(args)
                        .map_err(|e| format!("参数解析失败: {}", e))?;

                    // 创建工具实例
                    let tool = <$tool_type>::default();

                    // 调用工具
                    tool.call(args).await
                        .map_err(|e| format!("工具调用失败: {}", e))
                })
            })
        }
    };
}

/// 工具验证宏 - 验证工具是否正确实现了所需的trait
#[macro_export]
macro_rules! validate_tool {
    ($tool_type:ty) => {
        const _: () = {
            // 检查是否实现了Tool trait
            fn _check_tool<T: rig::tool::Tool>(_: T) {}

            // 如果没有实现Tool trait，这里会编译错误
            fn _validate() {
                let tool = <$tool_type>::default();
                _check_tool(tool);
            }
        };
    };
}

/// 批量验证所有工具的宏
#[macro_export]
macro_rules! validate_all_tools {
    () => {
        validate_tool!(ThinkTool);
        validate_tool!(TopPhiSimulator);
        validate_tool!(MLPerformancePredictor);
        validate_tool!(HistoricalDataQuery);
        validate_tool!(ExperimentalDataReader);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_tool_registration_macro() {
        use crate::server::mcp::tool_registry::ToolRegistry;

        let mut registry = ToolRegistry {
            tools: HashMap::new(),
        };

        // 测试单个工具注册
        register_mcp_tools!(
            registry,
            ThinkTool {
                args_type: ThinkArgs,
                constructor: ThinkTool
            },
        );

        assert_eq!(registry.tools.len(), 1);
        assert!(registry.tools.contains_key("think"));
    }

    #[tokio::test]
    async fn test_all_tools_registration_macro() {
        use crate::server::mcp::tool_registry::ToolRegistry;

        let mut registry = ToolRegistry {
            tools: HashMap::new(),
        };

        // 测试批量工具注册
        register_all_mcp_tools!(registry);

        // 验证注册的工具数量
        assert!(registry.tools.len() >= 5); // 至少有5个工具

        // 验证特定工具存在
        assert!(registry.tools.contains_key("think"));
        assert!(registry.tools.contains_key("topPhi_simulator"));
    }

    #[test]
    fn test_tool_validation_macro() {
        // 编译时测试 - 如果工具没有实现正确的trait，这里会编译失败
        validate_tool!(ThinkTool);
    }
}
