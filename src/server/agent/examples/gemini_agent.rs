use rig::prelude::*;
use rig::providers::gemini::completion::gemini_api_types::AdditionalParameters;
use rig::{
    completion::Prompt,
    providers::gemini::{self, completion::gemini_api_types::GenerationConfig},
};
#[tracing::instrument(ret)]
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .init();

    // 初始化 Google Gemini 客户端
    let client = gemini::Client::from_env();

    let gen_cfg = GenerationConfig {
        top_k: Some(1),
        top_p: Some(0.95),
        candidate_count: Some(1),
        ..Default::default()
    };
    let cfg = AdditionalParameters::default().with_config(gen_cfg);

    // 创建具有单一上下文提示的代理
    let agent = client
        .agent("gemini-2.5-pro")
        .preamble("Be creative and concise. Answer directly and clearly.")
        .temperature(0.5)
        // `GenerationConfig` 实用结构体有助于构造类型安全的 `additional_params`
        .additional_params(serde_json::to_value(cfg)?) // 展开 Result 以获得 Value
        .build();
    tracing::info!("Prompting the agent...");

    // 提示代理并打印响应
    let response = agent
        .prompt("How much wood would a woodchuck chuck if a woodchuck could chuck wood? Infer an answer.")
        .await;

    tracing::info!("Response: {:?}", response);

    match response {
        Ok(response) => println!("{response}"),
        Err(e) => {
            tracing::error!("Error: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
