use rig::prelude::*;
use rig::providers::openai;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 表示一个人的记录
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct Person {
    /// 此人的名字，如果提供的话（否则为 null）
    #[schemars(required)]
    pub first_name: Option<String>,
    /// 此人的姓氏，如果提供的话（否则为 null）
    #[schemars(required)]
    pub last_name: Option<String>,
    /// 此人的工作，如果提供的话（否则为 null）
    #[schemars(required)]
    pub job: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 创建 OpenAI 客户端
    let openai_client = openai::Client::from_env();

    // 创建提取器
    let data_extractor = openai_client.extractor::<Person>("gpt-4").build();
    let person = data_extractor
        .extract("Hello my name is John Doe! I am a software engineer.")
        .await?;

    println!("GPT-4: {}", serde_json::to_string_pretty(&person).unwrap());

    Ok(())
}
