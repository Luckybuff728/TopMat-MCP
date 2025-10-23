//! 此模块定义了 [Embed] trait，必须为可以被
//! [crate::embeddings::EmbeddingsBuilder] 嵌入的类型实现。
//!
//! 此模块还定义了 [EmbedError] 结构体，用于当 [Embed] trait 的
//! [Embed::embed] 方法失败时。
//!
//! 此模块还定义了 [TextEmbedder] 结构体，它累积需要嵌入的字符串值。
//! 它直接与 [Embed] trait 一起使用。
//!
//! 最后，此模块为许多常见的原始类型实现了 [Embed]。

/// 当 [Embed] trait 的 [Embed::embed] 方法失败时使用的错误类型。
/// 用于常见类型的 [Embed] 默认实现。
// 当 [Embed] trait 的 [Embed::embed] 方法失败时使用的错误类型
// 用于常见类型的 [Embed] 默认实现
#[derive(Debug, thiserror::Error)]
// 错误格式化宏，使用内部错误的消息
#[error("{0}")]
// 嵌入错误结构体，包含一个装箱的错误对象
pub struct EmbedError(#[from] Box<dyn std::error::Error + Send + Sync>);

// 为 EmbedError 实现方法
impl EmbedError {
    // 创建新的嵌入错误实例
    pub fn new<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        // 将错误装箱并创建 EmbedError
        EmbedError(Box::new(error))
    }
}

/// 为需要转换为向量嵌入的对象派生此 trait。
/// [Embed::embed] 方法通过将需要嵌入的字符串值添加到 [TextEmbedder] 来累积它们。
/// 如果发生错误，方法应返回 [EmbedError]。
/// # 示例
/// ```rust
/// use std::env;
///
/// use serde::{Deserialize, Serialize};
/// use rig::{Embed, embeddings::{TextEmbedder, EmbedError}};
///
/// struct WordDefinition {
///     id: String,
///     word: String,
///     definitions: String,
/// }
///
/// impl Embed for WordDefinition {
///     fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
///        // Embeddings only need to be generated for `definition` field.
///        // Split the definitions by comma and collect them into a vector of strings.
///        // That way, different embeddings can be generated for each definition in the `definitions` string.
///        self.definitions
///            .split(",")
///            .for_each(|s| {
///                embedder.embed(s.to_string());
///            });
///
///        Ok(())
///     }
/// }
///
/// let fake_definition = WordDefinition {
///    id: "1".to_string(),
///    word: "apple".to_string(),
///    definitions: "a fruit, a tech company".to_string(),
/// };
///
/// assert_eq!(embeddings::to_texts(fake_definition).unwrap(), vec!["a fruit", " a tech company"]);
/// ```
// 为需要转换为向量嵌入的对象派生此 trait
// [Embed::embed] 方法通过将需要嵌入的字符串值添加到 [TextEmbedder] 来累积它们
// 如果发生错误，方法应返回 [EmbedError]
// # 示例
// ```rust
// use std::env;
//
// use serde::{Deserialize, Serialize};
// use rig::{Embed, embeddings::{TextEmbedder, EmbedError}};
//
// struct WordDefinition {
//     id: String,
//     word: String,
//     definitions: String,
// }
//
// impl Embed for WordDefinition {
//     fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
//        // 只需要为 `definition` 字段生成嵌入
//        // 按逗号分割定义并收集到字符串向量中
//        // 这样可以为 `definitions` 字符串中的每个定义生成不同的嵌入
//        self.definitions
//            .split(",")
//            .for_each(|s| {
//                embedder.embed(s.to_string());
//            });
//
//        Ok(())
//     }
// }
//
// let fake_definition = WordDefinition {
//    id: "1".to_string(),
//    word: "apple".to_string(),
//    definitions: "a fruit, a tech company".to_string(),
// };
//
// assert_eq!(embeddings::to_texts(fake_definition).unwrap(), vec!["a fruit", " a tech company"]);
// ```
// 嵌入 trait，定义如何将对象转换为可嵌入的文本
pub trait Embed {
    // 将对象嵌入到文本嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError>;
}

/// Accumulates string values that need to be embedded.
/// Used by the [Embed] trait.
// 累积需要嵌入的字符串值
// 由 [Embed] trait 使用
#[derive(Default)]
pub struct TextEmbedder {
    // 需要嵌入的文本列表（crate 内部可见）
    pub(crate) texts: Vec<String>,
}

// 为 TextEmbedder 实现方法
impl TextEmbedder {
    /// Adds input `text` string to the list of texts in the [TextEmbedder] that need to be embedded.
    // 将输入的 `text` 字符串添加到 [TextEmbedder] 中需要嵌入的文本列表中
    pub fn embed(&mut self, text: String) {
        // 将文本添加到文本列表中
        self.texts.push(text);
    }
}

/// Utility function that returns a vector of strings that need to be embedded for a
/// given object that implements the [Embed] trait.
// 实用函数，返回实现 [Embed] trait 的给定对象需要嵌入的字符串向量
pub fn to_texts(item: impl Embed) -> Result<Vec<String>, EmbedError> {
    // 创建默认的文本嵌入器
    let mut embedder = TextEmbedder::default();
    // 将对象嵌入到嵌入器中
    item.embed(&mut embedder)?;
    // 返回嵌入的文本列表
    Ok(embedder.texts)
}

// ================================================================
// 常见类型的 Embed 实现
// ================================================================

// 为 String 类型实现 Embed trait
impl Embed for String {
    // 将字符串嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 克隆字符串并添加到嵌入器中
        embedder.embed(self.clone());
        // 返回成功
        Ok(())
    }
}

// 为 &str 类型实现 Embed trait
impl Embed for &str {
    // 将字符串切片嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将字符串切片转换为 String 并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 i8 类型实现 Embed trait
impl Embed for i8 {
    // 将 8 位有符号整数嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将整数转换为字符串并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 i16 类型实现 Embed trait
impl Embed for i16 {
    // 将 16 位有符号整数嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将整数转换为字符串并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 i32 类型实现 Embed trait
impl Embed for i32 {
    // 将 32 位有符号整数嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将整数转换为字符串并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 i64 类型实现 Embed trait
impl Embed for i64 {
    // 将 64 位有符号整数嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将整数转换为字符串并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 i128 类型实现 Embed trait
impl Embed for i128 {
    // 将 128 位有符号整数嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将整数转换为字符串并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 f32 类型实现 Embed trait
impl Embed for f32 {
    // 将 32 位浮点数嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将浮点数转换为字符串并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 f64 类型实现 Embed trait
impl Embed for f64 {
    // 将 64 位浮点数嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将浮点数转换为字符串并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 bool 类型实现 Embed trait
impl Embed for bool {
    // 将布尔值嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将布尔值转换为字符串并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 char 类型实现 Embed trait
impl Embed for char {
    // 将字符嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将字符转换为字符串并添加到嵌入器中
        embedder.embed(self.to_string());
        // 返回成功
        Ok(())
    }
}

// 为 serde_json::Value 类型实现 Embed trait
impl Embed for serde_json::Value {
    // 将 JSON 值嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 将 JSON 值序列化为字符串并添加到嵌入器中
        embedder.embed(serde_json::to_string(self).map_err(EmbedError::new)?);
        // 返回成功
        Ok(())
    }
}

// 为实现了 Embed 的类型的引用实现 Embed trait
impl<T: Embed> Embed for &T {
    // 将引用对象嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 解引用并调用嵌入方法
        (*self).embed(embedder)
    }
}

// 为包含实现了 Embed 的类型的 Vec 实现 Embed trait
impl<T: Embed> Embed for Vec<T> {
    // 将向量中的每个元素嵌入到嵌入器中
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // 遍历向量中的每个元素
        for item in self {
            // 将每个元素嵌入到嵌入器中，如果出错则转换为 EmbedError
            item.embed(embedder).map_err(EmbedError::new)?;
        }
        // 返回成功
        Ok(())
    }
}
