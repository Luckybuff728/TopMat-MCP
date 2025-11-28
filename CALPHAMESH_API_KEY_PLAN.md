# CalphaMesh API Key 用户隔离方案

## 背景问题

当前 `src/server/mcp/tools/calphaMesh.rs` 中所有工具的 API key 都是硬编码的：
```rust
let client = CalphaMeshClient::new("tk_zaEVQtzrfFIXKh7EnBoja8KnGIfjV0T8".to_string());
```

这导致所有用户共享同一个 API key，无法实现用户级别的隔离。工具在三个地方被使用：
1. **MCP 服务器** (`/mcp`) - 通过 `tool_registry.rs` 调用
2. **SSE 端点** (`/sse`) - 通过 MCP 协议调用
3. **Agent** (ollama.rs) - 直接作为 rig 工具使用

## 解决方案

### 核心思路
**统一方案**：所有三个场景都采用相同的策略 - 在工具参数中传递 API key，工具从参数中获取 API key。

- **MCP 和 SSE 场景**：MCP 服务器自动从认证上下文提取用户 API key 并注入到工具参数
- **Agent 场景**：Agent 在调用工具时显式传递用户 API key

### 设计原则
- **简洁统一**：所有场景使用相同的 API key 传递机制
- **向后兼容**：现有代码结构保持不变
- **完全隔离**：每个用户使用自己的 API key，确保多用户并发安全

## 实现方案

### 1. 修改工具参数结构 - 添加 API key 字段

**文件**: `src/server/mcp/tools/calphaMesh.rs`

```rust
// 为所有参数结构添加 api_key 字段
#[derive(Debug, Serialize, Deserialize)]
pub struct PointTaskParams {
    pub components: Vec<String>,
    pub composition: HashMap<String, f64>,
    pub temperature: f64,
    pub pressure: f64,
    pub database: String,
    /// API 密钥（由系统自动注入或用户显式提供）
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LineTaskParams {
    pub components: Vec<String>,
    pub start_composition: HashMap<String, f64>,
    pub start_temperature: f64,
    pub end_composition: HashMap<String, f64>,
    pub end_temperature: f64,
    pub pressure: f64,
    pub steps: i64,
    pub database: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheilTaskParams {
    pub components: Vec<String>,
    pub composition: HashMap<String, f64>,
    pub temperature: f64,
    pub pressure: f64,
    pub database: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskIdParams {
    pub task_id: i32,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTasksParams {
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_items_per_page")]
    pub items_per_page: i32,
    pub api_key: String,
}
```

### 2. 修改工具实现 - 使用参数中的 API key

**文件**: `src/server/mcp/tools/calphaMesh.rs`

```rust
impl Tool for SubmitPointTask {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 直接使用参数中的 API key
        let client = CalphaMeshClient::new(args.api_key);
        let task_response = client.submit_point_task(args).await?;

        Ok(format!(
            "✅ Point 计算任务提交成功！\n📋 任务ID: {}\n📊 状态: {}\n🔬 类型: point",
            task_response.id, task_response.status
        ))
    }
}

impl Tool for SubmitLineTask {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = CalphaMeshClient::new(args.api_key);
        let task_response = client.submit_line_task(args).await?;

        Ok(format!(
            "✅ Line 计算任务提交成功！\n📋 任务ID: {}\n📊 状态: {}\n🔬 类型: line",
            task_response.id, task_response.status
        ))
    }
}

impl Tool for SubmitScheilTask {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = CalphaMeshClient::new(args.api_key);
        let task_response = client.submit_scheil_task(args).await?;

        Ok(format!(
            "✅ Scheil 计算任务提交成功！\n📋 任务ID: {}\n📊 状态: {}\n🔬 类型: scheil",
            task_response.id, task_response.status
        ))
    }
}

impl Tool for GetTaskStatus {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = CalphaMeshClient::new(args.api_key);
        let task = client.get_task_status(args.task_id).await?;

        let status_emoji = match task.status.as_str() {
            "pending" => "⏳", "queued" => "📋", "running" => "⚙️",
            "completed" => "✅", "failed" => "❌", _ => "❓"
        };

        let mut result = format!(
            "{} 任务状态查询结果\n\n📋 任务ID: {}\n📝 标题: {}\n🔬 类型: {}\n📊 状态: {} {}\n👤 用户ID: {}\n🕐 创建时间: {}\n🕒 更新时间: {}",
            status_emoji, task.id, task.title, task.task_type, status_emoji, task.status,
            task.user_id, task.created_at, task.updated_at
        );

        if let Some(result_data) = &task.result {
            result.push_str("\n\n🎯 计算结果:\n");
            result.push_str(result_data);
        }

        if let Some(logs) = &task.logs {
            result.push_str(&format!("\n\n📄 日志:\n{}", logs));
        }

        Ok(result)
    }
}

impl Tool for ListTasks {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = CalphaMeshClient::new(args.api_key);
        let list = client.list_tasks(args.page, args.items_per_page).await?;

        let mut result = format!("📋 我的任务列表 (第 {} 页，共 {} 页)\n\n", list.page, list.total_pages);

        if list.data.is_empty() {
            result.push_str("🤷‍♂️ 暂无任务");
        } else {
            for (idx, task) in list.data.iter().enumerate() {
                let status_emoji = match task.status.as_str() {
                    "pending" => "⏳", "queued" => "📋", "running" => "⚙️",
                    "completed" => "✅", "failed" => "❌", _ => "❓"
                };
                result.push_str(&format!(
                    "{}. {} ID:{} | {} | {} | {}\n",
                    idx + 1, status_emoji, task.id, task.task_type, task.status, task.title
                ));
            }
        }

        Ok(result)
    }
}
```


### 3. 场景1&2：MCP 服务器自动注入 API key

**文件**: `src/server/mcp/mcp_server.rs`

```rust
impl ServerHandler for TopMatMcpServer {
    async fn call_tool(
        &self,
        CallToolRequestParam { name, arguments }: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        info!("调用工具: {}", name);

        // 为 calphamesh 工具自动注入 API key
        let mut modified_arguments = arguments;
        if name.starts_with("calphamesh_") {
            // 从认证上下文中提取用户的 API key
            if let Some(api_key) = extract_user_api_key(&context)? {
                let mut args_map: serde_json::Map<String, serde_json::Value> =
                    modified_arguments.unwrap_or_default().into_iter().collect();

                // 注入 API key 到参数中
                args_map.insert("api_key".to_string(), serde_json::Value::String(api_key));
                modified_arguments = Some(args_map);

                info!("为工具 {} 注入用户 API key", name);
            }
        }

        let args_value = modified_arguments.unwrap_or(json!({}));

        // 调用工具
        match ToolRegistry::call_tool(&name, args_value).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
            Err(e) => Err(McpError::new(format!("工具调用失败: {}", e))),
        }
    }
}

// 从认证上下文中提取用户 API key
// 直接使用认证中间件设置的 Extension<AuthUser>
fn extract_user_api_key(context: &RequestContext<RoleServer>) -> Result<Option<String>, McpError> {
    // 从 request extensions 中获取 AuthUser（由认证中间件设置）
    if let Some(auth_user) = context.extensions().get::<crate::server::middleware::auth::AuthUser>() {
        Ok(Some(auth_user.api_key.clone()))
    } else {
        tracing::warn!("未找到用户认证信息");
        Ok(None)
    }
}
```

### 4. 场景3：Agent 使用 - 直接从认证中间件获取

**文件**: `src/server/agent/ollama.rs`

```rust
pub async fn ollama_llama3(
    Extension(auth_user): Extension<crate::server::middleware::auth::AuthUser>,  // 直接接收认证中间件传递的用户信息
    Json(request): Json<ChatRequest>,
) -> Result<...> {
    // 直接使用认证后的用户 API key
    let user_api_key = auth_user.api_key;

    let system_prompt = format!(
        "你是一个材料方向的助理，擅长数学计算和使用工具进行计算。\
        \n\n重要：你的用户 CalphaMesh API key 是: {}\
        \n当调用任何 calphamesh 工具时，必须在参数中包含 'api_key' 字段，值为这个 API key。",
        user_api_key
    );
    // 后续代码
}
```

### 5. Agent 请求示例

客户端发送请求时需要在 HTTP Header 中包含用户的 API key：

```http
POST /v1/chat HTTP/1.1
Host: localhost:3000
Content-Type: application/json
Authorization: Bearer user_specific_api_key_here

{
    "message": "提交一个铝镁合金的点平衡计算任务",
    "model": "qwen3:8b",
    "stream": false
}
```

## 使用场景流程

### 场景1：MCP 工具调用 (`/mcp`)
```
客户端请求 → MCP 认证中间件 → MCP 服务器提取用户 API key → 注入到工具参数 → 工具调用
```

### 场景2：SSE 端点 (`/sse`)
```
客户端请求 → MCP 协议 → MCP 认证 → MCP 服务器提取用户 API key → 注入到工具参数 → 工具调用
```

### 场景3：Agent 使用 (ollama.rs)
```
用户对话 → HTTP Header 中的 API key → 认证中间件验证 → AuthUser 传递到处理函数 → Agent 在系统提示中获知 API key → Agent 在工具调用时包含 API key → 工具使用参数中的 API key
```

## 方案优势

1. **完全统一**：所有三个场景使用相同的 API key 传递机制
2. **完全用户隔离**：每个用户使用自己的 API key，确保多用户并发安全
3. **简洁透明**：对用户透明，系统自动处理 API key 传递
4. **向后兼容**：现有代码结构保持不变，只修改工具参数和实现
5. **易于维护**：逻辑集中，便于调试和扩展
6. **参考成熟方案**：基于已有项目的成熟代码模式

## 注意事项

1. **认证依赖**：MCP/SSE 场景依赖于正确的用户认证，确保认证中间件正确设置 `AuthUser`
2. **参数结构变更**：所有工具参数结构都需要添加 `api_key: String` 字段
3. **工具定义更新**：所有工具定义都需要声明 `api_key` 参数为必需字段
4. **错误处理**：当没有用户认证时，需要提供合适的错误处理机制
5. **Agent 可靠性**：需要确保 Agent 能够理解并在工具调用中正确包含 API key
6. **认证中间件依赖**：Agent 场景依赖于认证中间件正确设置 AuthUser 扩展

## 实现步骤

1. ✅ 分析当前架构和问题
2. ✅ 设计统一 API key 传递方案
3. ✅ 编写方案文档
4. 🔄 实现方案：
   - 修改 `calphaMesh.rs` 参数结构添加 `api_key` 字段
   - 修改所有工具实现使用参数中的 API key
   - 修改 `mcp_server.rs` 添加 API key 注入逻辑（从 AuthUser 扩展获取）
   - 修改 Agent 处理函数，从 `Extension<AuthUser>` 获取 API key
   - 更新所有工具定义，将 `api_key` 设为必需
   - 确保 Agent 处理函数参数正确（添加 `Extension<AuthUser>` 和 `Json<ChatRequest>`）
5. 🧪 测试三个使用场景

## 参考代码

本方案参考了以下成熟实现：
- `E:\fmq\work\TopMat-mcp\src\plugins.rs` (349-380行) - API key 注入逻辑
- `E:\fmq\work\TopMat-mcp\plugins\calphadmesh\src\lib.rs` (196-204行) - 参数中获取 API key