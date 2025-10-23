# Agent 制作方法指南

基于对 `src/agent/examples` 中所有示例文件的分析，本文档提供了使用 Rig 框架制作 Agent 的完整方法指南。

## 目录
1. [基础 Agent 创建](#基础-agent-创建)
2. [带工具的 Agent](#带工具的-agent)
3. [多轮对话 Agent](#多轮对话-agent)
4. [流式处理 Agent](#流式处理-agent)
5. [RAG Agent](#rag-agent)
6. [多 Agent 系统](#多-agent-系统)
7. [Agent 编排与路由](#agent-编排与路由)
8. [并行处理 Agent](#并行处理-agent)
9. [自主运行 Agent](#自主运行-agent)
10. [API Key 配置与使用](#api-key-配置与使用)
11. [最佳实践](#最佳实践)

## 基础 Agent 创建

### 最简单的 Agent
```rust
use rig::prelude::*;
use rig::{completion::Prompt, providers};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 创建客户端
    let client = providers::openai::Client::from_env();
    
    // 创建基础 agent
    let agent = client
        .agent("gpt-4o")
        .preamble("You are a helpful assistant.")
        .build();
    
    // 使用 agent
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 带上下文的 Agent
```rust
use rig::prelude::*;
use rig::{agent::AgentBuilder, completion::Prompt, providers::cohere};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = cohere::Client::new(&env::var("COHERE_API_KEY")?);
    let model = client.completion_model("command-r");
    
    // 创建带多个上下文的 agent
    let agent = AgentBuilder::new(model)
        .context("Definition of a *flurbo*: A flurbo is a green alien that lives on cold planets")
        .context("Definition of a *glarb-glarb*: A glarb-glarb is an ancient tool used by the ancestors")
        .context("Definition of a *linglingdong*: A term used by inhabitants of the far side of the moon")
        .build();
    
    let response = agent.prompt("What does \"glarb-glarb\" mean?").await?;
    println!("{response}");
    
    Ok(())
}
```

## 带工具的 Agent

### 定义工具
```rust
use anyhow::Result;
use rig::prelude::*;
use rig::{
    completion::{Prompt, ToolDefinition},
    providers,
    tool::Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
struct OperationArgs {
    x: i32,
    y: i32,
}

#[derive(Debug, thiserror::Error)]
#[error("Math error")]
struct MathError;

#[derive(Deserialize, Serialize)]
struct Adder;

impl Tool for Adder {
    const NAME: &'static str = "add";
    type Error = MathError;
    type Args = OperationArgs;
    type Output = i32;

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
                "required": ["x", "y"],
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        println!("[tool-call] Adding {} and {}", args.x, args.y);
        let result = args.x + args.y;
        Ok(result)
    }
}
```

### 使用工具的 Agent
```rust
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = providers::openai::Client::from_env();
    
    // 创建带工具的 agent
    let calculator_agent = client
        .agent(providers::openai::GPT_4O)
        .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.")
        .max_tokens(1024)
        .tool(Adder)
        .tool(Subtract)  // 假设已定义
        .build();
    
    let response = calculator_agent.prompt("Calculate 2 - 5").await?;
    println!("{response}");
    
    Ok(())
}
```

## 多轮对话 Agent

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = providers::anthropic::Client::from_env();
    
    let agent = client
        .agent(providers::anthropic::CLAUDE_3_5_SONNET)
        .preamble("You are an assistant here to help the user select which tool is most appropriate to perform arithmetic operations.")
        .tool(Add)
        .tool(Subtract)
        .tool(Multiply)
        .tool(Divide)
        .build();
    
    // 多轮对话
    let result = agent
        .prompt("Calculate 5 - 2 = ?. Describe the result to me.")
        .multi_turn(20)  // 最多20轮对话
        .await?;
    
    println!("Result: {result}");
    
    Ok(())
}
```

## 流式处理 Agent

```rust
use rig::agent::stream_to_stdout;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let calculator_agent = providers::openai::Client::from_env()
        .agent(providers::openai::GPT_4O)
        .preamble("You are a calculator here to help the user perform arithmetic operations.")
        .max_tokens(1024)
        .tool(Adder)
        .tool(Subtract)
        .build();
    
    // 流式处理
    let mut stream = calculator_agent.stream_prompt("Calculate 2 - 5").await;
    let res = stream_to_stdout(&mut stream).await?;
    
    println!("Token usage: {:?}", res.usage());
    println!("Final response: {:?}", res.response());
    
    Ok(())
}
```

## RAG Agent

### 数据准备
```rust
use rig::{
    Embed, completion::Prompt, embeddings::EmbeddingsBuilder,
    providers::openai::TEXT_EMBEDDING_ADA_002, vector_store::in_memory_store::InMemoryVectorStore,
};
use serde::Serialize;

#[derive(Embed, Serialize, Clone, Debug, Eq, PartialEq, Default)]
struct WordDefinition {
    id: String,
    word: String,
    #[embed]
    definitions: Vec<String>,
}
```

### 创建 RAG Agent
```rust
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let openai_client = providers::openai::Client::from_env();
    let embedding_model = openai_client.embedding_model(TEXT_EMBEDDING_ADA_002);
    
    // 生成嵌入向量
    let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
        .documents(vec![
            WordDefinition {
                id: "doc0".to_string(),
                word: "flurbo".to_string(),
                definitions: vec!["A flurbo is a green alien that lives on cold planets.".to_string()]
            },
            // 更多文档...
        ])?
        .build()
        .await?;
    
    // 创建向量存储
    let vector_store = InMemoryVectorStore::from_documents(embeddings);
    let index = vector_store.index(embedding_model);
    
    // 创建 RAG agent
    let rag_agent = openai_client.agent("gpt-4")
        .preamble("You are a dictionary assistant here to assist the user in understanding the meaning of words.")
        .dynamic_context(1, index)  // 动态上下文，返回最相关的1个文档
        .build();
    
    let response = rag_agent.prompt("What does \"glarb-glarb\" mean?").await?;
    println!("{response}");
    
    Ok(())
}
```

## 多 Agent 系统

### Agent 作为工具
```rust
use rig::cli_chatbot::ChatBotBuilder;

// 定义翻译工具
struct TranslatorTool<M: CompletionModel>(Agent<M>);

#[derive(Deserialize)]
struct TranslatorArgs {
    prompt: String,
}

impl<M: CompletionModel> Tool for TranslatorTool<M> {
    const NAME: &'static str = "translator";
    type Args = TranslatorArgs;
    type Error = PromptError;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": Self::NAME,
            "description": "Translate any text to English. If already in English, fix grammar and syntax issues.",
            "parameters": {
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "The text to translate to English"
                    },
                },
                "required": ["prompt"]
            }
        })).expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        match self.0.chat(&args.prompt, vec![]).await {
            Ok(response) => {
                println!("Translated prompt: {response}");
                Ok(response)
            }
            Err(e) => Err(e),
        }
    }
}
```

### 多 Agent 系统实现
```rust
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let openai_client = providers::openai::Client::from_env();
    let model = openai_client.completion_model("gpt-4");
    
    // 创建翻译 agent
    let translator_agent = AgentBuilder::new(model.clone())
        .preamble("You are a translator assistant that will translate any input text into english.")
        .build();
    
    let translator_tool = TranslatorTool(translator_agent);
    
    // 创建主 agent
    let multi_agent_system = AgentBuilder::new(model)
        .preamble("You are a helpful assistant that can work with text in any language.")
        .tool(translator_tool)
        .build();
    
    // 创建 CLI 聊天机器人
    let chatbot = ChatBotBuilder::new()
        .agent(multi_agent_system)
        .multi_turn_depth(1)
        .build();
    
    chatbot.run().await?;
    
    Ok(())
}
```

## Agent 编排与路由

### 基于分类的路由
```rust
use rig::pipeline::{self, Op, TryOp};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let openai_client = providers::openai::Client::from_env();
    
    // 分类 agent
    let animal_agent = openai_client.agent("gpt-4")
        .preamble("Your role is to categorise the user's statement using the following values: [sheep, cow, dog]. Return only the value.")
        .build();
    
    let default_agent = openai_client.agent("gpt-4").build();
    
    // 创建管道
    let chain = pipeline::new()
        .prompt(animal_agent)
        .map_ok(|x: String| match x.trim() {
            "cow" => Ok("Tell me a fact about the United States of America.".to_string()),
            "sheep" => Ok("Calculate 5+5 for me. Return only the number.".to_string()),
            "dog" => Ok("Write me a poem about cashews".to_string()),
            message => Err(format!("Could not process - received category: {message}")),
        })
        .map(|x| x.unwrap().unwrap())
        .prompt(default_agent);
    
    let response = chain.try_call("Sheep can self-medicate").await?;
    println!("Pipeline result: {response:?}");
    
    Ok(())
}
```

## 并行处理 Agent

```rust
use rig::{
    parallel,
    pipeline::{self, Op, passthrough},
};
use schemars::JsonSchema;

#[derive(serde::Deserialize, JsonSchema, serde::Serialize)]
struct DocumentScore {
    score: f32,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let openai_client = providers::openai::Client::from_env();
    
    // 创建多个评分 agent
    let manipulation_agent = openai_client
        .extractor::<DocumentScore>("gpt-4")
        .preamble("Your role is to score a user's statement on how manipulative it sounds between 0 and 1.")
        .build();
    
    let depression_agent = openai_client
        .extractor::<DocumentScore>("gpt-4")
        .preamble("Your role is to score a user's statement on how depressive it sounds between 0 and 1.")
        .build();
    
    let intelligent_agent = openai_client
        .extractor::<DocumentScore>("gpt-4")
        .preamble("Your role is to score a user's statement on how intelligent it sounds between 0 and 1.")
        .build();
    
    // 并行处理管道
    let chain = pipeline::new()
        .chain(parallel!(
            passthrough(),
            extract(manipulation_agent),
            extract(depression_agent),
            extract(intelligent_agent)
        ))
        .map(|(statement, manip_score, dep_score, int_score)| {
            format!(
                "Original statement: {statement}\n\
                Manipulation sentiment score: {}\n\
                Depression sentiment score: {}\n\
                Intelligence sentiment score: {}",
                manip_score.unwrap().score,
                dep_score.unwrap().score,
                int_score.unwrap().score
            )
        });
    
    let response = chain
        .call("I hate swimming. The water always gets in my eyes.")
        .await;
    
    println!("Pipeline run: {response:?}");
    
    Ok(())
}
```

## 自主运行 Agent

### 使用 Extractor 进行自主运行
```rust
use schemars::JsonSchema;

#[derive(Debug, serde::Deserialize, JsonSchema, serde::Serialize)]
struct Counter {
    number: u32,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let openai_client = providers::openai::Client::from_env();
    
    let agent = openai_client.extractor::<Counter>("gpt-4")
        .preamble("Your role is to add a random number between 1 and 64 (using only integers) to the previous number.")
        .build();
    
    let mut number: u32 = 0;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    
    // 自主运行循环
    loop {
        let response = agent.extract(&number.to_string()).await.unwrap();
        if response.number >= 2000 {
            break;
        } else {
            number += response.number
        }
        interval.tick().await;
    }
    
    println!("Finished with number: {number:?}");
    
    Ok(())
}
```
## API Key 配置与使用

基于对 `src/agent/examples` 中所有示例文件的分析，Rig 框架支持多种 API Key 配置方式。以下是各种提供商的具体使用方法：

### 1. OpenAI API Key

#### 方式一：使用 `from_env()` 方法（推荐）
```rust
use rig::prelude::*;
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 自动从环境变量 OPENAI_API_KEY 读取
    let client = providers::openai::Client::from_env();
    
    let agent = client
        .agent("gpt-4o")
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

#### 方式二：手动指定 API Key
```rust
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let client = providers::openai::Client::new(&api_key);
    
    let agent = client.agent("gpt-4o").build();
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 2. Anthropic API Key

```rust
use rig::prelude::*;
use rig::providers::anthropic;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 方式一：使用 from_env()
    let client = providers::anthropic::Client::from_env();
    
    // 方式二：手动指定
    // let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");
    // let client = providers::anthropic::Client::new(&api_key);
    
    // 方式三：使用 Builder 模式（支持 beta 功能）
    // let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");
    // let client = providers::anthropic::ClientBuilder::new(&api_key)
    //     .anthropic_beta("token-efficient-tools-2025-02-19")
    //     .build()?;
    
    let agent = client
        .agent(providers::anthropic::CLAUDE_3_5_SONNET)
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 3. 通义千问 (Qwen) API Key

```rust
use rig::prelude::*;
use rig::providers::qwen;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 从环境变量 DASHSCOPE_API_KEY 读取
    let client = providers::qwen::Client::from_env();
    
    let agent = client
        .agent("qwen-plus")
        .preamble("你是一个有用的助手。")
        .build();
    
    let response = agent.prompt("你好！").await?;
    println!("{response}");
    
    Ok(())
}
```

### 4. Cohere API Key

```rust
use rig::prelude::*;
use rig::providers::cohere;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 方式一：手动指定 API Key
    let api_key = env::var("COHERE_API_KEY").expect("COHERE_API_KEY not set");
    let client = providers::cohere::Client::new(&api_key);
    
    let agent = client
        .agent("command-r")
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 5. Google Gemini API Key

```rust
use rig::prelude::*;
use rig::providers::gemini;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 从环境变量读取
    let client = providers::gemini::Client::from_env();
    
    let agent = client
        .agent("gemini-pro")
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 6. Groq API Key

```rust
use rig::prelude::*;
use rig::providers::groq;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let api_key = env::var("GROQ_API_KEY").expect("GROQ_API_KEY not set");
    let client = providers::groq::Client::new(&api_key);
    
    let agent = client
        .agent("llama-3.1-70b-versatile")
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 7. xAI (Grok) API Key

```rust
use rig::prelude::*;
use rig::providers::xai;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let api_key = env::var("XAI_API_KEY").expect("XAI_API_KEY not set");
    let client = providers::xai::Client::new(&api_key);
    
    let agent = client
        .agent(providers::xai::GROK_3_MINI)
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 8. Together AI API Key

```rust
use rig::prelude::*;
use rig::providers::together;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 方式一：使用 from_env()
    let client = providers::together::Client::from_env();
    
    // 方式二：手动指定
    // let api_key = env::var("TOGETHER_API_KEY").expect("TOGETHER_API_KEY not set");
    // let client = providers::together::Client::new(&api_key);
    
    let agent = client
        .agent(providers::together::LLAMA_3_8B_CHAT_HF)
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 9. Hugging Face API Key

```rust
use rig::prelude::*;
use rig::providers::huggingface;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 方式一：直接创建客户端
    let api_key = env::var("HUGGINGFACE_API_KEY").expect("HUGGINGFACE_API_KEY not set");
    let client = providers::huggingface::Client::new(&api_key);
    
    // 方式二：使用 Builder 模式（支持子提供商）
    // let client = providers::huggingface::ClientBuilder::new(&api_key)
    //     .sub_provider(huggingface::SubProvider::Together)
    //     .build()?;
    
    let agent = client
        .agent("meta-llama/Llama-3.1-8B-Instruct")
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 10. OpenRouter API Key

```rust
use rig::prelude::*;
use rig::providers::openrouter;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let api_key = env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY not set");
    let client = providers::openrouter::Client::new(&api_key);
    
    let agent = client
        .agent(providers::openrouter::GEMINI_FLASH_2_0)
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 11. Perplexity API Key

```rust
use rig::prelude::*;
use rig::providers::perplexity;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let api_key = env::var("PERPLEXITY_API_KEY").expect("PERPLEXITY_API_KEY not set");
    let client = providers::perplexity::Client::new(&api_key);
    
    let agent = client
        .agent("llama-3.1-sonar-small-128k-online")
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 12. Ollama（本地部署，无需 API Key）

```rust
use rig::prelude::*;
use rig::providers::ollama;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Ollama 不需要 API Key，直接创建客户端
    let client = providers::ollama::Client::new();
    
    let agent = client
        .agent("llama3.2")
        .preamble("You are a helpful assistant.")
        .build();
    
    let response = agent.prompt("Hello!").await?;
    println!("{response}");
    
    Ok(())
}
```

### 环境变量配置

根据示例文件分析，以下是各种提供商所需的环境变量名称：

| 提供商 | 环境变量名 | 说明 |
|--------|------------|------|
| OpenAI | `OPENAI_API_KEY` | OpenAI API 密钥 |
| Anthropic | `ANTHROPIC_API_KEY` | Anthropic API 密钥 |
| 通义千问 | `DASHSCOPE_API_KEY` | 阿里云 DashScope API 密钥 |
| Cohere | `COHERE_API_KEY` | Cohere API 密钥 |
| Google Gemini | `GOOGLE_API_KEY` | Google AI API 密钥 |
| Groq | `GROQ_API_KEY` | Groq API 密钥 |
| xAI | `XAI_API_KEY` | xAI API 密钥 |
| Together AI | `TOGETHER_API_KEY` | Together AI API 密钥 |
| Hugging Face | `HUGGINGFACE_API_KEY` | Hugging Face API 密钥 |
| OpenRouter | `OPENROUTER_API_KEY` | OpenRouter API 密钥 |
| Perplexity | `PERPLEXITY_API_KEY` | Perplexity API 密钥 |
| Azure | `AZURE_API_KEY` | Azure OpenAI API 密钥 |

### 运行示例时的环境变量设置

根据示例文件中的注释，运行示例时需要设置相应的环境变量：

```bash
# 通义千问示例
DASHSCOPE_API_KEY=your_api_key cargo run --example qwen_streaming

# OpenAI 示例
OPENAI_API_KEY=your_api_key cargo run --example agent

# Anthropic 示例
ANTHROPIC_API_KEY=your_api_key cargo run --example anthropic_agent

# Cohere 示例
COHERE_API_KEY=your_api_key cargo run --example agent_with_cohere

# Groq 示例
GROQ_API_KEY=your_api_key cargo run --example agent_with_groq

# 多提供商示例
OPENAI_API_KEY=your_openai_key ANTHROPIC_API_KEY=your_anthropic_key cargo run --example multi_agent
```

### API Key 安全最佳实践

1. **使用环境变量**：永远不要在代码中硬编码 API Key
2. **使用 `.env` 文件**：在开发环境中使用 `.env` 文件管理环境变量
3. **权限最小化**：只授予必要的 API 权限
4. **定期轮换**：定期更换 API Key
5. **监控使用情况**：监控 API 使用量和费用

```rust
// 使用 dotenv 加载环境变量
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 加载 .env 文件
    dotenv().ok();
    
    let client = providers::openai::Client::from_env();
    // ... 其他代码
}
```

## 总结

本指南涵盖了使用 Rig 框架创建各种类型 Agent 的完整方法：

1. **基础 Agent**: 简单的对话助手
2. **工具增强 Agent**: 具备特定功能的工具调用能力
3. **多轮对话 Agent**: 支持上下文记忆的对话
4. **流式处理 Agent**: 实时响应生成
5. **RAG Agent**: 基于检索增强生成的智能助手
6. **多 Agent 系统**: 多个 Agent 协作完成复杂任务
7. **编排与路由**: 智能任务分发和处理
8. **并行处理**: 同时处理多个任务
9. **自主运行**: 无需人工干预的自动化 Agent
10. **API Key 配置**: 各种提供商的 API Key 使用方式

通过合理组合这些模式，可以构建出功能强大、适应性强的 AI Agent 系统。
