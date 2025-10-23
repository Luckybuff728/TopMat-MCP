// 导入代理相关类型
use crate::{
    // 导入代理、多轮流项目、文本类型
    agent::{Agent, MultiTurnStreamItem, Text},
    // 导入聊天、完成错误、完成模型、提示错误、使用统计类型
    completion::{Chat, CompletionError, CompletionModel, PromptError, Usage},
    // 导入消息类型
    message::Message,
    // 导入流式助手内容、流式提示类型
    streaming::{StreamedAssistantContent, StreamingPrompt},
};
// 导入流处理扩展方法
use futures::StreamExt;
// 导入 IO 操作相关类型
use std::io::{self, Write};

// 定义无实现提供的标记结构体
pub struct NoImplProvided;

// 定义聊天实现结构体，支持泛型聊天类型
pub struct ChatImpl<T>(T)
where
    // T 必须实现 Chat trait
    T: Chat;

// 定义代理实现结构体，支持泛型完成模型类型
pub struct AgentImpl<M>
where
    // M 必须实现 CompletionModel 且生命周期为 'static
    M: CompletionModel + 'static,
{
    // 代理实例
    agent: Agent<M>,
    // 多轮对话深度
    multi_turn_depth: usize,
    // 是否显示使用统计
    show_usage: bool,
    // 使用统计信息
    usage: Usage,
}

// 定义聊天机器人构建器，支持泛型类型
pub struct ChatBotBuilder<T>(T);

// 定义聊天机器人，支持泛型类型
pub struct ChatBot<T>(T);

/// 用于从 cli_chat/`run` 循环中抽象消息行为的 trait
// 允许私有接口的 trait 定义
#[allow(private_interfaces)]
// 定义 CLI 聊天 trait
trait CliChat {
    // 定义异步请求方法，接受提示和历史消息
    async fn request(&mut self, prompt: &str, history: Vec<Message>)
    -> Result<String, PromptError>;

    // 定义是否显示使用统计的方法，默认返回 false
    fn show_usage(&self) -> bool {
        false
    }

    // 定义获取使用统计的方法，默认返回 None
    fn usage(&self) -> Option<Usage> {
        None
    }
}

// 为 ChatImpl 实现 CliChat trait
impl<T> CliChat for ChatImpl<T>
where
    // T 必须实现 Chat trait
    T: Chat,
{
    // 实现异步请求方法
    async fn request(
        &mut self,
        prompt: &str,
        history: Vec<Message>,
    ) -> Result<String, PromptError> {
        // 调用内部聊天实现
        let res = self.0.chat(prompt, history).await?;
        // 打印响应
        println!("{res}");

        // 返回响应结果
        Ok(res)
    }
}

// 为 AgentImpl 实现 CliChat trait
impl<M> CliChat for AgentImpl<M>
where
    // M 必须实现 CompletionModel 且生命周期为 'static
    M: CompletionModel + 'static,
{
    // 实现异步请求方法
    async fn request(
        &mut self,
        prompt: &str,
        history: Vec<Message>,
    ) -> Result<String, PromptError> {
        // 创建流式响应流
        let mut response_stream = self
            .agent
            // 流式处理提示
            .stream_prompt(prompt)
            // 设置历史消息
            .with_history(history)
            // 设置多轮对话深度
            .multi_turn(self.multi_turn_depth)
            .await;

        // 初始化累积字符串
        let mut acc = String::new();

        loop {
            let Some(chunk) = response_stream.next().await else {
                break Ok(acc);
            };

            match chunk {
                Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::Text(Text {
                    text,
                }))) => {
                    print!("{}", text);
                    acc.push_str(&text);
                }
                Ok(MultiTurnStreamItem::FinalResponse(final_response)) => {
                    self.usage = final_response.usage();
                }
                Err(e) => {
                    break Err(PromptError::CompletionError(
                        CompletionError::ResponseError(e.to_string()),
                    ));
                }
                _ => continue,
            }
        }
    }

    fn show_usage(&self) -> bool {
        self.show_usage
    }

    fn usage(&self) -> Option<Usage> {
        Some(self.usage)
    }
}

impl Default for ChatBotBuilder<NoImplProvided> {
    fn default() -> Self {
        Self(NoImplProvided)
    }
}

impl ChatBotBuilder<NoImplProvided> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn agent<M: CompletionModel + 'static>(
        self,
        agent: Agent<M>,
    ) -> ChatBotBuilder<AgentImpl<M>> {
        ChatBotBuilder(AgentImpl {
            agent,
            multi_turn_depth: 1,
            show_usage: false,
            usage: Usage::default(),
        })
    }

    pub fn chat<T: Chat>(self, chatbot: T) -> ChatBotBuilder<ChatImpl<T>> {
        ChatBotBuilder(ChatImpl(chatbot))
    }
}

impl<T> ChatBotBuilder<ChatImpl<T>>
where
    T: Chat,
{
    pub fn build(self) -> ChatBot<ChatImpl<T>> {
        let ChatBotBuilder(chat_impl) = self;
        ChatBot(chat_impl)
    }
}

impl<M> ChatBotBuilder<AgentImpl<M>>
where
    M: CompletionModel + 'static,
{
    pub fn multi_turn_depth(self, multi_turn_depth: usize) -> Self {
        ChatBotBuilder(AgentImpl {
            multi_turn_depth,
            ..self.0
        })
    }

    pub fn show_usage(self) -> Self {
        ChatBotBuilder(AgentImpl {
            show_usage: true,
            ..self.0
        })
    }

    pub fn build(self) -> ChatBot<AgentImpl<M>> {
        ChatBot(self.0)
    }
}

#[allow(private_bounds)]
impl<T> ChatBot<T>
where
    T: CliChat,
{
    pub async fn run(mut self) -> Result<(), PromptError> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut history = vec![];

        loop {
            print!("> ");
            stdout.flush().unwrap();

            let mut input = String::new();
            match stdin.read_line(&mut input) {
                Ok(_) => {
                    let input = input.trim();
                    if input == "exit" {
                        break;
                    }

                    tracing::info!("Prompt:\n{input}\n");

                    println!();
                    println!("========================== Response ============================");

                    let response = self.0.request(input, history.clone()).await?;
                    history.push(Message::user(input));
                    history.push(Message::assistant(response));

                    println!("================================================================");
                    println!();

                    if self.0.show_usage() {
                        let Usage {
                            input_tokens,
                            output_tokens,
                            ..
                        } = self.0.usage().unwrap();
                        println!("Input {input_tokens} tokens\nOutput {output_tokens} tokens");
                    }
                }
                Err(e) => println!("Error reading request: {e}"),
            }
        }

        Ok(())
    }
}
