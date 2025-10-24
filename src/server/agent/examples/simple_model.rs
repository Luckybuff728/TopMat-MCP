use rig::prelude::*;
use rig::{completion::Prompt, providers::openai};

#[tokio::main]
async fn main() {
    // 创建 OpenAI 客户端和模型
    let openai_client = openai::Client::from_env();
    let gpt4 = openai_client.agent("gpt-4").build();

    // 提示模型并打印其响应
    let response = gpt4
        .prompt("Who are you?")
        .await
        .expect("Failed to prompt GPT-4");

    println!("GPT-4: {response}");
}
