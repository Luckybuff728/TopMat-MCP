//! 此模块定义了 [EmbeddingsBuilder] 结构体，它累积要嵌入的对象
//! 并在构建时为每个对象批量生成嵌入。
//! 只有实现 [Embed] trait 的类型才能添加到 [EmbeddingsBuilder]。

// 导入标准库的比较和集合功能
use std::{cmp::max, collections::HashMap};

// 导入 futures 的流处理和流扩展功能
use futures::{StreamExt, stream};

// 导入 crate 中的相关类型和模块
use crate::{
    // OneOrMany 类型，用于表示单个或多个值
    OneOrMany,
    // 嵌入相关的类型和 trait
    embeddings::{
        // Embed trait，用于定义可嵌入的对象
        Embed, 
        // 嵌入错误类型
        EmbedError, 
        // 嵌入向量类型
        Embedding, 
        // 嵌入模型错误类型
        EmbeddingError, 
        // 嵌入模型 trait
        EmbeddingModel, 
        // 文本嵌入器
        embed::TextEmbedder,
    },
};

/// 用于从类型 `T` 的一个或多个文档创建嵌入的构建器。
/// 注意：`T` 可以是实现 [Embed] trait 的任何类型。
///
/// 使用构建器比直接使用 [EmbeddingModel::embed_text] 更可取，因为
/// 它会在单个请求中批处理文档到模型提供商。
///
/// # 示例
/// ```rust
/// use std::env;
///
/// use rig::{
///     embeddings::EmbeddingsBuilder,
///     providers::openai::{Client, TEXT_EMBEDDING_ADA_002},
/// };
/// use serde::{Deserialize, Serialize};
///
/// // Create OpenAI client
/// let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
/// let openai_client = Client::new(&openai_api_key);
///
/// let model = openai_client.embedding_model(TEXT_EMBEDDING_ADA_002);
///
/// let embeddings = EmbeddingsBuilder::new(model.clone())
///     .documents(vec![
///         "1. *flurbo* (noun): A green alien that lives on cold planets.".to_string(),
///         "2. *flurbo* (noun): A fictional digital currency that originated in the animated series Rick and Morty.".to_string()
///         "1. *glarb-glarb* (noun): An ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.".to_string(),
///         "2. *glarb-glarb* (noun): A fictional creature found in the distant, swampy marshlands of the planet Glibbo in the Andromeda galaxy.".to_string()
///         "1. *linlingdong* (noun): A term used by inhabitants of the sombrero galaxy to describe humans.".to_string(),
///         "2. *linlingdong* (noun): A rare, mystical instrument crafted by the ancient monks of the Nebulon Mountain Ranges on the planet Quarm.".to_string()
///     ])?
///     .build()
///     .await?;
/// ```
// 用于从类型 `T` 的一个或多个文档创建嵌入的构建器
// 注意：`T` 可以是实现 [Embed] trait 的任何类型
//
// 使用构建器比直接使用 [EmbeddingModel::embed_text] 更可取，因为
// 它会在单个请求中批处理文档到模型提供商
//
// # 示例
// ```rust
// use std::env;
//
// use rig::{
//     embeddings::EmbeddingsBuilder,
//     providers::openai::{Client, TEXT_EMBEDDING_ADA_002},
// };
// use serde::{Deserialize, Serialize};
//
// // 创建 OpenAI 客户端
// let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
// let openai_client = Client::new(&openai_api_key);
//
// let model = openai_client.embedding_model(TEXT_EMBEDDING_ADA_002);
//
// let embeddings = EmbeddingsBuilder::new(model.clone())
//     .documents(vec![
//         "1. *flurbo* (noun): A green alien that lives on cold planets.".to_string(),
//         "2. *flurbo* (noun): A fictional digital currency that originated in the animated series Rick and Morty.".to_string()
//         "1. *glarb-glarb* (noun): An ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.".to_string(),
//         "2. *glarb-glarb* (noun): A fictional creature found in the distant, swampy marshlands of the planet Glibbo in the Andromeda galaxy.".to_string()
//         "1. *linlingdong* (noun): A term used by inhabitants of the sombrero galaxy to describe humans.".to_string(),
//         "2. *linlingdong* (noun): A rare, mystical instrument crafted by the ancient monks of the Nebulon Mountain Ranges on the planet Quarm.".to_string()
//     ])?
//     .build()
//     .await?;
// ```
// 标记为非穷尽结构体，表示未来可能会添加更多字段
#[non_exhaustive]
pub struct EmbeddingsBuilder<M, T>
where
    // M 必须实现 EmbeddingModel trait
    M: EmbeddingModel,
    // T 必须实现 Embed trait
    T: Embed,
{
    // 嵌入模型实例
    model: M,
    // 文档列表，每个文档包含原始对象和提取的文本列表
    documents: Vec<(T, Vec<String>)>,
}

// 为 EmbeddingsBuilder 实现基本方法
impl<M, T> EmbeddingsBuilder<M, T>
where
    // M 必须实现 EmbeddingModel trait
    M: EmbeddingModel,
    // T 必须实现 Embed trait
    T: Embed,
{
    /// Create a new embedding builder with the given embedding model
    // 使用给定的嵌入模型创建新的嵌入构建器
    pub fn new(model: M) -> Self {
        Self {
            // 设置嵌入模型
            model,
            // 初始化文档列表为空向量
            documents: vec![],
        }
    }

    /// Add a document to be embedded to the builder. `document` must implement the [Embed] trait.
    // 向构建器添加要嵌入的文档。`document` 必须实现 [Embed] trait
    pub fn document(mut self, document: T) -> Result<Self, EmbedError> {
        // 创建默认的文本嵌入器
        let mut embedder = TextEmbedder::default();
        // 调用文档的嵌入方法，提取文本
        document.embed(&mut embedder)?;

        // 将文档和提取的文本添加到文档列表
        self.documents.push((document, embedder.texts));

        // 返回修改后的构建器
        Ok(self)
    }

    /// Add multiple documents to be embedded to the builder. `documents` must be iterable
    /// with items that implement the [Embed] trait.
    // 向构建器添加多个要嵌入的文档。`documents` 必须是可迭代的，
    // 且项目必须实现 [Embed] trait
    pub fn documents(self, documents: impl IntoIterator<Item = T>) -> Result<Self, EmbedError> {
        // 使用 try_fold 逐个添加文档
        let builder = documents
            // 转换为迭代器
            .into_iter()
            // 尝试折叠，逐个添加文档
            .try_fold(self, |builder, doc| builder.document(doc))?;

        // 返回修改后的构建器
        Ok(builder)
    }
}

// 为 EmbeddingsBuilder 实现异步构建方法
impl<M, T> EmbeddingsBuilder<M, T>
where
    // M 必须实现 EmbeddingModel trait
    M: EmbeddingModel,
    // T 必须实现 Embed trait 和 Send trait（用于跨线程发送）
    T: Embed + Send,
{
    /// Generate embeddings for all documents in the builder.
    /// Returns a vector of tuples, where the first element is the document and the second element is the embeddings (either one embedding or many).
    // 为构建器中的所有文档生成嵌入
    // 返回一个元组向量，其中第一个元素是文档，第二个元素是嵌入（一个或多个）
    pub async fn build(self) -> Result<Vec<(T, OneOrMany<Embedding>)>, EmbeddingError> {
        // 导入流扩展 trait
        use stream::TryStreamExt;

        // Store the documents and their texts in a HashMap for easy access.
        // 在 HashMap 中存储文档及其文本，以便于访问
        let mut docs = HashMap::new();
        let mut texts = Vec::new();

        // Iterate over all documents in the builder and insert their docs and texts into the lookup stores.
        // 遍历构建器中的所有文档，并将它们的文档和文本插入到查找存储中
        for (i, (doc, doc_texts)) in self.documents.into_iter().enumerate() {
            // 将文档插入到 HashMap 中，使用索引作为键
            docs.insert(i, doc);
            // 将索引和文本列表添加到文本向量中
            texts.push((i, doc_texts));
        }

        // Compute the embeddings.
        // 计算嵌入向量
        let mut embeddings = stream::iter(texts.into_iter())
            // Merge the texts of each document into a single list of texts.
            // 将每个文档的文本合并为单个文本列表
            .flat_map(|(i, texts)| stream::iter(texts.into_iter().map(move |text| (i, text))))
            // Chunk them into batches. Each batch size is at most the embedding API limit per request.
            // 将它们分块为批次。每个批次大小最多是每次请求的嵌入 API 限制
            .chunks(M::MAX_DOCUMENTS)
            // Generate the embeddings for each batch.
            // 为每个批次生成嵌入
            .map(|text| async {
                // 将文本批次解构为 ID 和文档
                let (ids, docs): (Vec<_>, Vec<_>) = text.into_iter().unzip();

                // 调用模型生成嵌入
                let embeddings = self.model.embed_texts(docs).await?;
                // 将 ID 和嵌入配对并收集为向量
                Ok::<_, EmbeddingError>(ids.into_iter().zip(embeddings).collect::<Vec<_>>())
            })
            // Parallelize the embeddings generation over 10 concurrent requests
            // 并行化嵌入生成，最多 10 个并发请求
            .buffer_unordered(max(1, 1024 / M::MAX_DOCUMENTS))
            // Collect the embeddings into a HashMap.
            // 将嵌入收集到 HashMap 中
            .try_fold(
                // 初始化空的 HashMap
                HashMap::new(),
                // 折叠函数，将嵌入添加到累加器中
                |mut acc: HashMap<_, OneOrMany<Embedding>>, embeddings| async move {
                    // 遍历每个嵌入
                    embeddings.into_iter().for_each(|(i, embedding)| {
                        // 如果键已存在，则添加嵌入；否则创建新的 OneOrMany
                        acc.entry(i)
                            .and_modify(|embeddings| embeddings.push(embedding.clone()))
                            .or_insert(OneOrMany::one(embedding.clone()));
                    });

                    // 返回累加器
                    Ok(acc)
                },
            )
            .await?;

        // Merge the embeddings with their respective documents
        // 将嵌入与它们各自的文档合并
        Ok(docs
            .into_iter()
            .map(|(i, doc)| {
                (
                    // 文档
                    doc,
                    // 从嵌入 HashMap 中移除对应的嵌入
                    embeddings.remove(&i).expect("Document should be present"),
                )
            })
            .collect())
    }
}

// 测试模块
#[cfg(test)]
mod tests {
    // 导入测试所需的类型和 trait
    use crate::{
        // Embed trait
        Embed,
        // 嵌入相关的类型和 trait
        embeddings::{Embedding, EmbeddingModel, embed::EmbedError, embed::TextEmbedder},
    };

    // 导入要测试的 EmbeddingsBuilder
    use super::EmbeddingsBuilder;

    // 测试用的模型结构体
    #[derive(Clone)]
    struct Model;

    // 为 Model 实现 EmbeddingModel trait
    impl EmbeddingModel for Model {
        // 每次请求的最大文档数
        const MAX_DOCUMENTS: usize = 5;

        // 返回嵌入向量的维度
        fn ndims(&self) -> usize {
            // 返回 10 维向量
            10
        }

        // 嵌入文本的异步方法
        async fn embed_texts(
            // 自身引用
            &self,
            // 要嵌入的文档列表
            documents: impl IntoIterator<Item = String> + Send,
        ) -> Result<Vec<crate::embeddings::Embedding>, crate::embeddings::EmbeddingError> {
            // 为每个文档创建模拟的嵌入向量
            Ok(documents
                .into_iter()
                .map(|doc| Embedding {
                    // 文档内容
                    document: doc.to_string(),
                    // 模拟的 10 维向量
                    vec: vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9],
                })
                .collect())
        }
    }

    // 单词定义结构体，用于测试多个文本的嵌入
    #[derive(Clone, Debug)]
    struct WordDefinition {
        // 文档 ID
        id: String,
        // 定义列表
        definitions: Vec<String>,
    }

    // 为 WordDefinition 实现 Embed trait
    impl Embed for WordDefinition {
        // 嵌入方法，将定义添加到嵌入器中
        fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
            // 遍历所有定义
            for definition in &self.definitions {
                // 将每个定义添加到嵌入器中
                embedder.embed(definition.clone());
            }
            // 返回成功
            Ok(())
        }
    }

    // 创建包含多个文本的定义列表
    fn definitions_multiple_text() -> Vec<WordDefinition> {
        vec![
            // 第一个单词定义
            WordDefinition {
                // 文档 ID
                id: "doc0".to_string(),
                // 多个定义
                definitions: vec![
                    "A green alien that lives on cold planets.".to_string(),
                    "A fictional digital currency that originated in the animated series Rick and Morty.".to_string()
                ]
            },
            // 第二个单词定义
            WordDefinition {
                // 文档 ID
                id: "doc1".to_string(),
                // 多个定义
                definitions: vec![
                    "An ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.".to_string(),
                    "A fictional creature found in the distant, swampy marshlands of the planet Glibbo in the Andromeda galaxy.".to_string()
                ]
            }
        ]
    }

    // 创建第二批包含多个文本的定义列表
    fn definitions_multiple_text_2() -> Vec<WordDefinition> {
        vec![
            // 第三个单词定义
            WordDefinition {
                // 文档 ID
                id: "doc2".to_string(),
                // 单个定义
                definitions: vec!["Another fake definitions".to_string()],
            },
            // 第四个单词定义
            WordDefinition {
                // 文档 ID
                id: "doc3".to_string(),
                // 单个定义
                definitions: vec!["Some fake definition".to_string()],
            },
        ]
    }

    // 单词定义结构体，用于测试单个文本的嵌入
    #[derive(Clone, Debug)]
    struct WordDefinitionSingle {
        // 文档 ID
        id: String,
        // 单个定义
        definition: String,
    }

    // 为 WordDefinitionSingle 实现 Embed trait
    impl Embed for WordDefinitionSingle {
        // 嵌入方法，将单个定义添加到嵌入器中
        fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
            // 将定义添加到嵌入器中
            embedder.embed(self.definition.clone());
            // 返回成功
            Ok(())
        }
    }

    // 创建包含单个文本的定义列表
    fn definitions_single_text() -> Vec<WordDefinitionSingle> {
        vec![
            // 第一个单词定义
            WordDefinitionSingle {
                // 文档 ID
                id: "doc0".to_string(),
                // 单个定义
                definition: "A green alien that lives on cold planets.".to_string(),
            },
            // 第二个单词定义
            WordDefinitionSingle {
                // 文档 ID
                id: "doc1".to_string(),
                // 单个定义
                definition: "An ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.".to_string(),
            }
        ]
    }

    // 测试多个文本的嵌入构建
    #[tokio::test]
    async fn test_build_multiple_text() {
        // 获取测试用的多个文本定义
        let fake_definitions = definitions_multiple_text();

        // 创建测试模型
        let fake_model = Model;
        // 构建嵌入并获取结果
        let mut result = EmbeddingsBuilder::new(fake_model)
            .documents(fake_definitions)
            .unwrap()
            .build()
            .await
            .unwrap();

        // 按 ID 排序结果
        result.sort_by(|(fake_definition_1, _), (fake_definition_2, _)| {
            fake_definition_1.id.cmp(&fake_definition_2.id)
        });

        // 验证结果数量
        assert_eq!(result.len(), 2);

        // 验证第一个定义
        let first_definition = &result[0];
        assert_eq!(first_definition.0.id, "doc0");
        assert_eq!(first_definition.1.len(), 2);
        assert_eq!(
            first_definition.1.first().document,
            "A green alien that lives on cold planets.".to_string()
        );

        // 验证第二个定义
        let second_definition = &result[1];
        assert_eq!(second_definition.0.id, "doc1");
        assert_eq!(second_definition.1.len(), 2);
        assert_eq!(
            second_definition.1.rest()[0].document, "A fictional creature found in the distant, swampy marshlands of the planet Glibbo in the Andromeda galaxy.".to_string()
        )
    }

    // 测试单个文本的嵌入构建
    #[tokio::test]
    async fn test_build_single_text() {
        // 获取测试用的单个文本定义
        let fake_definitions = definitions_single_text();

        // 创建测试模型
        let fake_model = Model;
        // 构建嵌入并获取结果
        let mut result = EmbeddingsBuilder::new(fake_model)
            .documents(fake_definitions)
            .unwrap()
            .build()
            .await
            .unwrap();

        // 按 ID 排序结果
        result.sort_by(|(fake_definition_1, _), (fake_definition_2, _)| {
            fake_definition_1.id.cmp(&fake_definition_2.id)
        });

        // 验证结果数量
        assert_eq!(result.len(), 2);

        // 验证第一个定义
        let first_definition = &result[0];
        assert_eq!(first_definition.0.id, "doc0");
        assert_eq!(first_definition.1.len(), 1);
        assert_eq!(
            first_definition.1.first().document,
            "A green alien that lives on cold planets.".to_string()
        );

        // 验证第二个定义
        let second_definition = &result[1];
        assert_eq!(second_definition.0.id, "doc1");
        assert_eq!(second_definition.1.len(), 1);
        assert_eq!(
            second_definition.1.first().document, "An ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.".to_string()
        )
    }

    // 测试多个和单个文本的混合嵌入构建
    #[tokio::test]
    async fn test_build_multiple_and_single_text() {
        // 获取多个文本定义
        let fake_definitions = definitions_multiple_text();
        // 获取单个文本定义
        let fake_definitions_single = definitions_multiple_text_2();

        // 创建测试模型
        let fake_model = Model;
        // 构建嵌入并获取结果，添加两批文档
        let mut result = EmbeddingsBuilder::new(fake_model)
            .documents(fake_definitions)
            .unwrap()
            .documents(fake_definitions_single)
            .unwrap()
            .build()
            .await
            .unwrap();

        // 按 ID 排序结果
        result.sort_by(|(fake_definition_1, _), (fake_definition_2, _)| {
            fake_definition_1.id.cmp(&fake_definition_2.id)
        });

        // 验证结果数量
        assert_eq!(result.len(), 4);

        // 验证第二个定义
        let second_definition = &result[1];
        assert_eq!(second_definition.0.id, "doc1");
        assert_eq!(second_definition.1.len(), 2);
        assert_eq!(
            second_definition.1.first().document, "An ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.".to_string()
        );

        // 验证第三个定义
        let third_definition = &result[2];
        assert_eq!(third_definition.0.id, "doc2");
        assert_eq!(third_definition.1.len(), 1);
        assert_eq!(
            third_definition.1.first().document,
            "Another fake definitions".to_string()
        )
    }

    // 测试字符串向量的嵌入构建
    #[tokio::test]
    async fn test_build_string() {
        // 获取多个文本定义
        let bindings = definitions_multiple_text();
        // 提取定义列表
        let fake_definitions = bindings.iter().map(|def| def.definitions.clone());

        // 创建测试模型
        let fake_model = Model;
        // 构建嵌入并获取结果
        let mut result = EmbeddingsBuilder::new(fake_model)
            .documents(fake_definitions)
            .unwrap()
            .build()
            .await
            .unwrap();

        // 按定义内容排序结果
        result.sort_by(|(fake_definition_1, _), (fake_definition_2, _)| {
            fake_definition_1.cmp(fake_definition_2)
        });

        // 验证结果数量
        assert_eq!(result.len(), 2);

        // 验证第一个定义
        let first_definition = &result[0];
        assert_eq!(first_definition.1.len(), 2);
        assert_eq!(
            first_definition.1.first().document,
            "A green alien that lives on cold planets.".to_string()
        );

        // 验证第二个定义
        let second_definition = &result[1];
        assert_eq!(second_definition.1.len(), 2);
        assert_eq!(
            second_definition.1.rest()[0].document, "A fictional creature found in the distant, swampy marshlands of the planet Glibbo in the Andromeda galaxy.".to_string()
        )
    }
}
