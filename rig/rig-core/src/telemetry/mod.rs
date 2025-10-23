//! 此模块主要关注能够在给定的管道或工作流中编排遥测。
//! 这包括追踪、能够将追踪发送到 OpenTelemetry 收集器、设置您的
//! 代理使用正确的追踪样式，以便您可以为 Langfuse 等平台发出正确的追踪，
//! 等等。

use crate::completion::GetTokenUsage;
use serde::Serialize;

pub trait ProviderRequestExt {
    type InputMessage: Serialize;

    fn get_input_messages(&self) -> Vec<Self::InputMessage>;
    fn get_system_prompt(&self) -> Option<String>;
    fn get_model_name(&self) -> String;
    fn get_prompt(&self) -> Option<String>;
}

pub trait ProviderResponseExt {
    type OutputMessage: Serialize;
    type Usage: Serialize;

    fn get_response_id(&self) -> Option<String>;

    fn get_response_model_name(&self) -> Option<String>;

    fn get_output_messages(&self) -> Vec<Self::OutputMessage>;

    fn get_text_response(&self) -> Option<String>;

    fn get_usage(&self) -> Option<Self::Usage>;
}

/// 专门设计用于与 Spans 一起使用的 trait，用于记录遥测。
/// 几乎所有方法
pub trait SpanCombinator {
    fn record_token_usage<U>(&self, usage: &U)
    where
        U: GetTokenUsage;

    fn record_response_metadata<R>(&self, response: &R)
    where
        R: ProviderResponseExt;

    fn record_model_input<T>(&self, messages: &T)
    where
        T: Serialize;

    fn record_model_output<T>(&self, messages: &T)
    where
        T: Serialize;
}

impl SpanCombinator for tracing::Span {
    fn record_token_usage<U>(&self, usage: &U)
    where
        U: GetTokenUsage,
    {
        if let Some(usage) = usage.token_usage() {
            self.record("gen_ai.usage.input_tokens", usage.input_tokens);
            self.record("gen_ai.usage.output_tokens", usage.output_tokens);
        }
    }

    fn record_response_metadata<R>(&self, response: &R)
    where
        R: ProviderResponseExt,
    {
        if let Some(id) = response.get_response_id() {
            self.record("gen_ai.response.id", id);
        }

        if let Some(model_name) = response.get_response_model_name() {
            self.record("gen_ai.response.model_name", model_name);
        }
    }

    fn record_model_input<T>(&self, input: &T)
    where
        T: Serialize,
    {
        let input_as_json_string =
            serde_json::to_string(input).expect("Serializing a Rust type to JSON should not break");

        self.record("gen_ai.input.messages", input_as_json_string);
    }

    fn record_model_output<T>(&self, input: &T)
    where
        T: Serialize,
    {
        let input_as_json_string =
            serde_json::to_string(input).expect("Serializing a Rust type to JSON should not break");

        self.record("gen_ai.input.messages", input_as_json_string);
    }
}
