//! 向量存储的内存实现。
// 向量存储的内存实现
use std::{
    // 导入 Reverse 用于反向排序
    cmp::Reverse,
    // 导入 BinaryHeap 和 HashMap 集合
    collections::{BinaryHeap, HashMap},
};

// 导入 OrderedFloat 用于浮点数排序
use ordered_float::OrderedFloat;
// 导入 serde 的序列化和反序列化 trait
use serde::{Deserialize, Serialize};

// 导入父模块的 VectorStoreError, VectorStoreIndex 和 VectorSearchRequest
use super::{VectorStoreError, VectorStoreIndex, request::VectorSearchRequest};
// 导入 crate 根模块的 OneOrMany 和嵌入相关类型
use crate::{
    // 一个或多个类型的包装器
    OneOrMany,
    // 嵌入相关的类型：Embedding, EmbeddingModel, VectorDistance
    embeddings::{Embedding, EmbeddingModel, distance::VectorDistance},
};

/// [InMemoryVectorStore] 是一个简单的内存向量存储，使用 HashMap 在内存中存储嵌入。
// [InMemoryVectorStore] 是一个简单的内存向量存储，使用 HashMap 在内存中存储嵌入
// 派生 Clone 和 Default trait
#[derive(Clone, Default)]
pub struct InMemoryVectorStore<D: Serialize> {
    /// 嵌入存储在 HashMap 中。
    /// HashMap 键是文档 ID。
    /// HashMap 值是可序列化文档及其对应嵌入的元组。
    // 嵌入存储在 HashMap 中
    // HashMap 键是文档 ID
    // HashMap 值是可序列化文档及其对应嵌入的元组
    embeddings: HashMap<String, (D, OneOrMany<Embedding>)>,
}

// 为 InMemoryVectorStore 实现方法（要求 D 实现 Serialize 和 Eq）
impl<D: Serialize + Eq> InMemoryVectorStore<D> {
    /// 从文档及其对应的嵌入创建新的 [InMemoryVectorStore]。
    /// ID 会自动生成，格式为 `"doc{n}"`，其中 `n` 是文档的索引。
    // 从文档及其对应的嵌入创建新的 [InMemoryVectorStore]
    // ID 会自动生成，格式为 `"doc{n}"`，其中 `n` 是文档的索引
    pub fn from_documents(documents: impl IntoIterator<Item = (D, OneOrMany<Embedding>)>) -> Self {
        // 创建新的 HashMap
        let mut store = HashMap::new();
        // 遍历文档，为每个文档生成 ID
        documents
            .into_iter()
            .enumerate()
            .for_each(|(i, (doc, embeddings))| {
                // 插入文档，ID 格式为 "doc{索引}"
                store.insert(format!("doc{i}"), (doc, embeddings));
            });

        // 返回新的 InMemoryVectorStore 实例
        Self { embeddings: store }
    }

    /// 从具有 ID 的文档及其对应的嵌入创建新的 [InMemoryVectorStore]。
    // 从具有 ID 的文档及其对应的嵌入创建新的 [InMemoryVectorStore]
    pub fn from_documents_with_ids(
        documents: impl IntoIterator<Item = (impl ToString, D, OneOrMany<Embedding>)>,
    ) -> Self {
        // 创建新的 HashMap
        let mut store = HashMap::new();
        // 遍历文档，使用提供的 ID
        documents.into_iter().for_each(|(i, doc, embeddings)| {
            // 插入文档，使用提供的 ID
            store.insert(i.to_string(), (doc, embeddings));
        });

        // 返回新的 InMemoryVectorStore 实例
        Self { embeddings: store }
    }

    /// Create a new [InMemoryVectorStore] from documents and their corresponding embeddings.
    /// Document ids are generated using the provided function.
    // 从文档及其对应的嵌入创建新的 [InMemoryVectorStore]
    // 文档 ID 使用提供的函数生成
    pub fn from_documents_with_id_f(
        documents: impl IntoIterator<Item = (D, OneOrMany<Embedding>)>,
        f: fn(&D) -> String,
    ) -> Self {
        // 创建新的 HashMap
        let mut store = HashMap::new();
        // 遍历文档，使用函数生成 ID
        documents.into_iter().for_each(|(doc, embeddings)| {
            // 使用提供的函数生成 ID 并插入文档
            store.insert(f(&doc), (doc, embeddings));
        });

        // 返回新的 InMemoryVectorStore 实例
        Self { embeddings: store }
    }

    /// Implement vector search on [InMemoryVectorStore].
    /// To be used by implementations of [VectorStoreIndex::top_n] and [VectorStoreIndex::top_n_ids] methods.
    // 在 [InMemoryVectorStore] 上实现向量搜索
    // 用于 [VectorStoreIndex::top_n] 和 [VectorStoreIndex::top_n_ids] 方法的实现
    fn vector_search(&self, prompt_embedding: &Embedding, n: usize) -> EmbeddingRanking<'_, D> {
        // Sort documents by best embedding distance
        // 按最佳嵌入距离排序文档
        let mut docs = BinaryHeap::new();

        // 遍历所有嵌入
        for (id, (doc, embeddings)) in self.embeddings.iter() {
            // Get the best context for the document given the prompt
            // 获取给定提示的文档的最佳上下文
            if let Some((distance, embed_doc)) = embeddings
                .iter()
                .map(|embedding| {
                    (
                        OrderedFloat(embedding.cosine_similarity(prompt_embedding, false)),
                        &embedding.document,
                    )
                })
                .max_by(|a, b| a.0.cmp(&b.0))
            {
                // 将排序项推入堆
                docs.push(Reverse(RankingItem(distance, id, doc, embed_doc)));
            };

            // If the heap size exceeds n, pop the least old element.
            // 如果堆大小超过 n，弹出最不相关的元素
            if docs.len() > n {
                docs.pop();
            }
        }

        // Log selected tools with their distances
        // 记录选中的文档及其距离
        tracing::info!(target: "rig",
            "Selected documents: {}",
            docs.iter()
                .map(|Reverse(RankingItem(distance, id, _, _))| format!("{id} ({distance})"))
                .collect::<Vec<String>>()
                .join(", ")
        );

        // 返回排序结果
        docs
    }

    /// Add documents and their corresponding embeddings to the store.
    /// Ids are automatically generated have will have the form `"doc{n}"` where `n`
    /// is the index of the document.
    // 将文档及其对应的嵌入添加到存储中
    // ID 会自动生成，格式为 `"doc{n}"`，其中 `n` 是文档的索引
    pub fn add_documents(
        &mut self,
        documents: impl IntoIterator<Item = (D, OneOrMany<Embedding>)>,
    ) {
        // 获取当前存储的文档数量
        let current_index = self.embeddings.len();
        // 遍历文档并添加到存储中
        documents
            .into_iter()
            .enumerate()
            .for_each(|(index, (doc, embeddings))| {
                // 插入文档，ID 格式为 "doc{当前索引 + 文档索引}"
                self.embeddings
                    .insert(format!("doc{}", index + current_index), (doc, embeddings));
            });
    }

    /// Add documents and their corresponding embeddings to the store with ids.
    // 将具有 ID 的文档及其对应的嵌入添加到存储中
    pub fn add_documents_with_ids(
        &mut self,
        documents: impl IntoIterator<Item = (impl ToString, D, OneOrMany<Embedding>)>,
    ) {
        // 遍历文档并添加到存储中
        documents.into_iter().for_each(|(id, doc, embeddings)| {
            // 使用提供的 ID 插入文档
            self.embeddings.insert(id.to_string(), (doc, embeddings));
        });
    }

    /// Add documents and their corresponding embeddings to the store.
    /// Document ids are generated using the provided function.
    // 将文档及其对应的嵌入添加到存储中
    // 文档 ID 使用提供的函数生成
    pub fn add_documents_with_id_f(
        &mut self,
        documents: Vec<(D, OneOrMany<Embedding>)>,
        f: fn(&D) -> String,
    ) {
        // 遍历文档
        for (doc, embeddings) in documents {
            // 使用提供的函数生成 ID
            let id = f(&doc);
            // 插入文档
            self.embeddings.insert(id, (doc, embeddings));
        }
    }

    /// Get the document by its id and deserialize it into the given type.
    // 根据 ID 获取文档并将其反序列化为给定类型
    pub fn get_document<T: for<'a> Deserialize<'a>>(
        &self,
        id: &str,
    ) -> Result<Option<T>, VectorStoreError> {
        // 获取文档并反序列化
        Ok(self
            .embeddings
            .get(id)
            .map(|(doc, _)| serde_json::from_str(&serde_json::to_string(doc)?))
            .transpose()?)
    }
}

/// RankingItem(distance, document_id, serializable document, embeddings document)
// RankingItem(距离, 文档ID, 可序列化文档, 嵌入文档)
// 派生 Eq 和 PartialEq trait
#[derive(Eq, PartialEq)]
struct RankingItem<'a, D: Serialize>(OrderedFloat<f64>, &'a String, &'a D, &'a String);

// 为 RankingItem 实现 Ord trait
impl<D: Serialize + Eq> Ord for RankingItem<'_, D> {
    // 比较两个 RankingItem
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 基于距离进行比较
        self.0.cmp(&other.0)
    }
}

// 为 RankingItem 实现 PartialOrd trait
impl<D: Serialize + Eq> PartialOrd for RankingItem<'_, D> {
    // 部分比较两个 RankingItem
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // 返回完整的比较结果
        Some(self.cmp(other))
    }
}

// 嵌入排序类型别名
type EmbeddingRanking<'a, D> = BinaryHeap<Reverse<RankingItem<'a, D>>>;

// 为 InMemoryVectorStore 实现其他方法（要求 D 实现 Serialize）
impl<D: Serialize> InMemoryVectorStore<D> {
    // 创建向量索引
    pub fn index<M: EmbeddingModel>(self, model: M) -> InMemoryVectorIndex<M, D> {
        // 创建新的向量索引
        InMemoryVectorIndex::new(model, self)
    }

    // 获取迭代器
    pub fn iter(&self) -> impl Iterator<Item = (&String, &(D, OneOrMany<Embedding>))> {
        // 返回嵌入的迭代器
        self.embeddings.iter()
    }

    // 获取长度
    pub fn len(&self) -> usize {
        // 返回嵌入的数量
        self.embeddings.len()
    }

    // 检查是否为空
    pub fn is_empty(&self) -> bool {
        // 检查嵌入是否为空
        self.embeddings.is_empty()
    }
}

// 内存向量索引结构体
pub struct InMemoryVectorIndex<M: EmbeddingModel, D: Serialize> {
    // 嵌入模型
    model: M,
    // 公共的向量存储
    pub store: InMemoryVectorStore<D>,
}

// 为 InMemoryVectorIndex 实现方法
impl<M: EmbeddingModel, D: Serialize> InMemoryVectorIndex<M, D> {
    // 创建新的向量索引
    pub fn new(model: M, store: InMemoryVectorStore<D>) -> Self {
        // 返回新的索引实例
        Self { model, store }
    }

    // 获取迭代器
    pub fn iter(&self) -> impl Iterator<Item = (&String, &(D, OneOrMany<Embedding>))> {
        // 返回存储的迭代器
        self.store.iter()
    }

    // 获取长度
    pub fn len(&self) -> usize {
        // 返回存储的长度
        self.store.len()
    }

    // 检查是否为空
    pub fn is_empty(&self) -> bool {
        // 检查存储是否为空
        self.store.is_empty()
    }
}

// 为 InMemoryVectorIndex 实现 VectorStoreIndex trait
impl<M: EmbeddingModel + Sync, D: Serialize + Sync + Send + Eq> VectorStoreIndex
    for InMemoryVectorIndex<M, D>
{
    // 获取前 n 个最相似的文档
    async fn top_n<T: for<'a> Deserialize<'a>>(
        &self,
        req: VectorSearchRequest,
    ) -> Result<Vec<(f64, String, T)>, VectorStoreError> {
        // 获取查询的嵌入向量
        let prompt_embedding = &self.model.embed_text(req.query()).await?;

        // 执行向量搜索
        let docs = self
            .store
            .vector_search(prompt_embedding, req.samples() as usize);

        // Return n best
        // 返回前 n 个最佳结果
        docs.into_iter()
            // The distance should always be between 0 and 1, so distance should be fine to use as an absolute value
            // 距离应该总是在 0 和 1 之间，所以距离可以直接用作绝对值
            .map(|Reverse(RankingItem(distance, id, doc, _))| {
                // 反序列化文档并返回结果
                Ok((
                    distance.0,
                    id.clone(),
                    serde_json::from_str(
                        &serde_json::to_string(doc).map_err(VectorStoreError::JsonError)?,
                    )
                    .map_err(VectorStoreError::JsonError)?,
                ))
            })
            .collect::<Result<Vec<_>, _>>()
    }

    // 获取前 n 个最相似文档的 ID
    async fn top_n_ids(
        &self,
        req: VectorSearchRequest,
    ) -> Result<Vec<(f64, String)>, VectorStoreError> {
        // 获取查询的嵌入向量
        let prompt_embedding = &self.model.embed_text(req.query()).await?;

        // 执行向量搜索
        let docs = self
            .store
            .vector_search(prompt_embedding, req.samples() as usize);

        // 返回距离和 ID 的元组
        docs.into_iter()
            .map(|Reverse(RankingItem(distance, id, _, _))| Ok((distance.0, id.clone())))
            .collect::<Result<Vec<_>, _>>()
    }
}

// 测试模块，只在测试时编译
#[cfg(test)]
mod tests {
    // 导入 Reverse 用于反向排序
    use std::cmp::Reverse;

    // 导入 OneOrMany 和 Embedding
    use crate::{OneOrMany, embeddings::embedding::Embedding};

    // 导入父模块的 InMemoryVectorStore 和 RankingItem
    use super::{InMemoryVectorStore, RankingItem};

    // 测试自动生成的 ID
    #[test]
    fn test_auto_ids() {
        // 创建向量存储并添加初始文档
        let mut vector_store = InMemoryVectorStore::from_documents(vec![
            (
                "glarb-garb",
                OneOrMany::one(Embedding {
                    document: "glarb-garb".to_string(),
                    vec: vec![0.1, 0.1, 0.5],
                }),
            ),
            (
                "marble-marble",
                OneOrMany::one(Embedding {
                    document: "marble-marble".to_string(),
                    vec: vec![0.7, -0.3, 0.0],
                }),
            ),
            (
                "flumb-flumb",
                OneOrMany::one(Embedding {
                    document: "flumb-flumb".to_string(),
                    vec: vec![0.3, 0.7, 0.1],
                }),
            ),
        ]);

        // 添加更多文档
        vector_store.add_documents(vec![
            (
                "brotato",
                OneOrMany::one(Embedding {
                    document: "brotato".to_string(),
                    vec: vec![0.3, 0.7, 0.1],
                }),
            ),
            (
                "ping-pong",
                OneOrMany::one(Embedding {
                    document: "ping-pong".to_string(),
                    vec: vec![0.7, -0.3, 0.0],
                }),
            ),
        ]);

        // 收集并排序存储项以进行验证
        let mut store = vector_store.embeddings.into_iter().collect::<Vec<_>>();
        store.sort_by_key(|(id, _)| id.clone());

        // 验证存储的内容和 ID 生成
        assert_eq!(
            store,
            vec![
                (
                    "doc0".to_string(),
                    (
                        "glarb-garb",
                        OneOrMany::one(Embedding {
                            document: "glarb-garb".to_string(),
                            vec: vec![0.1, 0.1, 0.5],
                        })
                    )
                ),
                (
                    "doc1".to_string(),
                    (
                        "marble-marble",
                        OneOrMany::one(Embedding {
                            document: "marble-marble".to_string(),
                            vec: vec![0.7, -0.3, 0.0],
                        })
                    )
                ),
                (
                    "doc2".to_string(),
                    (
                        "flumb-flumb",
                        OneOrMany::one(Embedding {
                            document: "flumb-flumb".to_string(),
                            vec: vec![0.3, 0.7, 0.1],
                        })
                    )
                ),
                (
                    "doc3".to_string(),
                    (
                        "brotato",
                        OneOrMany::one(Embedding {
                            document: "brotato".to_string(),
                            vec: vec![0.3, 0.7, 0.1],
                        })
                    )
                ),
                (
                    "doc4".to_string(),
                    (
                        "ping-pong",
                        OneOrMany::one(Embedding {
                            document: "ping-pong".to_string(),
                            vec: vec![0.7, -0.3, 0.0],
                        })
                    )
                )
            ]
        );
    }

    // 测试单个嵌入的向量搜索
    #[test]
    fn test_single_embedding() {
        let vector_store = InMemoryVectorStore::from_documents_with_ids(vec![
            (
                "doc1",
                "glarb-garb",
                OneOrMany::one(Embedding {
                    document: "glarb-garb".to_string(),
                    vec: vec![0.1, 0.1, 0.5],
                }),
            ),
            (
                "doc2",
                "marble-marble",
                OneOrMany::one(Embedding {
                    document: "marble-marble".to_string(),
                    vec: vec![0.7, -0.3, 0.0],
                }),
            ),
            (
                "doc3",
                "flumb-flumb",
                OneOrMany::one(Embedding {
                    document: "flumb-flumb".to_string(),
                    vec: vec![0.3, 0.7, 0.1],
                }),
            ),
        ]);

        let ranking = vector_store.vector_search(
            &Embedding {
                document: "glarby-glarble".to_string(),
                vec: vec![0.0, 0.1, 0.6],
            },
            1,
        );

        assert_eq!(
            ranking
                .into_iter()
                .map(|Reverse(RankingItem(distance, id, doc, _))| {
                    (
                        distance.0,
                        id.clone(),
                        serde_json::from_str(&serde_json::to_string(doc).unwrap()).unwrap(),
                    )
                })
                .collect::<Vec<(_, _, String)>>(),
            vec![(
                0.9807965956109156,
                "doc1".to_string(),
                "glarb-garb".to_string()
            )]
        )
    }

    // 测试多个嵌入的向量搜索
    #[test]
    fn test_multiple_embeddings() {
        let vector_store = InMemoryVectorStore::from_documents_with_ids(vec![
            (
                "doc1",
                "glarb-garb",
                OneOrMany::many(vec![
                    Embedding {
                        document: "glarb-garb".to_string(),
                        vec: vec![0.1, 0.1, 0.5],
                    },
                    Embedding {
                        document: "don't-choose-me".to_string(),
                        vec: vec![-0.5, 0.9, 0.1],
                    },
                ])
                .unwrap(),
            ),
            (
                "doc2",
                "marble-marble",
                OneOrMany::many(vec![
                    Embedding {
                        document: "marble-marble".to_string(),
                        vec: vec![0.7, -0.3, 0.0],
                    },
                    Embedding {
                        document: "sandwich".to_string(),
                        vec: vec![0.5, 0.5, -0.7],
                    },
                ])
                .unwrap(),
            ),
            (
                "doc3",
                "flumb-flumb",
                OneOrMany::many(vec![
                    Embedding {
                        document: "flumb-flumb".to_string(),
                        vec: vec![0.3, 0.7, 0.1],
                    },
                    Embedding {
                        document: "banana".to_string(),
                        vec: vec![0.1, -0.5, -0.5],
                    },
                ])
                .unwrap(),
            ),
        ]);

        let ranking = vector_store.vector_search(
            &Embedding {
                document: "glarby-glarble".to_string(),
                vec: vec![0.0, 0.1, 0.6],
            },
            1,
        );

        assert_eq!(
            ranking
                .into_iter()
                .map(|Reverse(RankingItem(distance, id, doc, _))| {
                    (
                        distance.0,
                        id.clone(),
                        serde_json::from_str(&serde_json::to_string(doc).unwrap()).unwrap(),
                    )
                })
                .collect::<Vec<(_, _, String)>>(),
            vec![(
                0.9807965956109156,
                "doc1".to_string(),
                "glarb-garb".to_string()
            )]
        )
    }
}
