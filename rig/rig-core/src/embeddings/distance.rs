// 向量距离计算 trait，定义各种向量距离计算方法
pub trait VectorDistance {
    /// 获取两个嵌入向量的点积
    // 计算两个向量的点积（内积）
    fn dot_product(&self, other: &Self) -> f64;

    /// 获取两个嵌入向量的余弦相似度。
    /// 如果 `normalized` 为 true，则返回点积。
    // 计算两个向量的余弦相似度
    // 如果 `normalized` 为 true，则直接返回点积（假设向量已归一化）
    fn cosine_similarity(&self, other: &Self, normalized: bool) -> f64;

    /// 获取两个嵌入向量的角度距离。
    // 计算两个向量的角度距离（基于余弦相似度）
    fn angular_distance(&self, other: &Self, normalized: bool) -> f64;

    /// 获取两个嵌入向量的欧几里得距离。
    // 计算两个向量的欧几里得距离（L2 距离）
    fn euclidean_distance(&self, other: &Self) -> f64;

    /// 获取两个嵌入向量的曼哈顿距离。
    // 计算两个向量的曼哈顿距离（L1 距离）
    fn manhattan_distance(&self, other: &Self) -> f64;

    /// 获取两个嵌入向量的切比雪夫距离。
    // 计算两个向量的切比雪夫距离（L∞ 距离）
    fn chebyshev_distance(&self, other: &Self) -> f64;
}

// 当没有启用 rayon 特性时，为 Embedding 实现 VectorDistance trait
#[cfg(not(feature = "rayon"))]
impl VectorDistance for crate::embeddings::Embedding {
    // 计算点积
    fn dot_product(&self, other: &Self) -> f64 {
        // 遍历两个向量的元素
        self.vec
            .iter()
            // 将两个向量配对
            .zip(other.vec.iter())
            // 计算每对元素的乘积
            .map(|(x, y)| x * y)
            // 求和
            .sum()
    }

    // 计算余弦相似度
    fn cosine_similarity(&self, other: &Self, normalized: bool) -> f64 {
        // 计算点积
        let dot_product = self.dot_product(other);

        // 如果向量已归一化，直接返回点积
        if normalized {
            dot_product
        } else {
            // 计算第一个向量的模长
            let magnitude1: f64 = self.vec.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
            // 计算第二个向量的模长
            let magnitude2: f64 = other.vec.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();

            // 返回点积除以两个模长的乘积
            dot_product / (magnitude1 * magnitude2)
        }
    }

    // 计算角度距离
    fn angular_distance(&self, other: &Self, normalized: bool) -> f64 {
        // 计算余弦相似度
        let cosine_sim = self.cosine_similarity(other, normalized);
        // 计算反余弦并除以 π 得到角度距离
        cosine_sim.acos() / std::f64::consts::PI
    }

    // 计算欧几里得距离
    fn euclidean_distance(&self, other: &Self) -> f64 {
        // 遍历两个向量的元素
        self.vec
            .iter()
            // 将两个向量配对
            .zip(other.vec.iter())
            // 计算每对元素差的平方
            .map(|(x, y)| (x - y).powi(2))
            // 求和并开平方根
            .sum::<f64>()
            .sqrt()
    }

    // 计算曼哈顿距离
    fn manhattan_distance(&self, other: &Self) -> f64 {
        // 遍历两个向量的元素
        self.vec
            .iter()
            // 将两个向量配对
            .zip(other.vec.iter())
            // 计算每对元素差的绝对值
            .map(|(x, y)| (x - y).abs())
            // 求和
            .sum()
    }

    // 计算切比雪夫距离
    fn chebyshev_distance(&self, other: &Self) -> f64 {
        // 遍历两个向量的元素
        self.vec
            .iter()
            // 将两个向量配对
            .zip(other.vec.iter())
            // 计算每对元素差的绝对值
            .map(|(x, y)| (x - y).abs())
            // 使用 fold 找到最大值
            .fold(0.0, f64::max)
    }
}

// 当启用 rayon 特性时，使用并行计算的实现
#[cfg(feature = "rayon")]
mod rayon {
    // 导入所需的类型和 trait
    use crate::embeddings::{Embedding, distance::VectorDistance};
    // 导入 rayon 的并行计算功能
    use rayon::prelude::*;

    // 为 Embedding 实现 VectorDistance trait（并行版本）
    impl VectorDistance for Embedding {
        // 计算点积（并行版本）
        fn dot_product(&self, other: &Self) -> f64 {
            // 使用并行迭代器遍历两个向量的元素
            self.vec
                .par_iter()
                // 将两个向量配对
                .zip(other.vec.par_iter())
                // 计算每对元素的乘积
                .map(|(x, y)| x * y)
                // 求和
                .sum()
        }

        // 计算余弦相似度（并行版本）
        fn cosine_similarity(&self, other: &Self, normalized: bool) -> f64 {
            // 计算点积
            let dot_product = self.dot_product(other);

            // 如果向量已归一化，直接返回点积
            if normalized {
                dot_product
            } else {
                // 使用并行迭代器计算第一个向量的模长
                let magnitude1: f64 = self.vec.par_iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
                // 使用并行迭代器计算第二个向量的模长
                let magnitude2: f64 = other.vec.par_iter().map(|x| x.powi(2)).sum::<f64>().sqrt();

                // 返回点积除以两个模长的乘积
                dot_product / (magnitude1 * magnitude2)
            }
        }

        // 计算角度距离（并行版本）
        fn angular_distance(&self, other: &Self, normalized: bool) -> f64 {
            // 计算余弦相似度
            let cosine_sim = self.cosine_similarity(other, normalized);
            // 计算反余弦并除以 π 得到角度距离
            cosine_sim.acos() / std::f64::consts::PI
        }

        // 计算欧几里得距离（并行版本）
        fn euclidean_distance(&self, other: &Self) -> f64 {
            // 使用并行迭代器遍历两个向量的元素
            self.vec
                .par_iter()
                // 将两个向量配对
                .zip(other.vec.par_iter())
                // 计算每对元素差的平方
                .map(|(x, y)| (x - y).powi(2))
                // 求和并开平方根
                .sum::<f64>()
                .sqrt()
        }

        // 计算曼哈顿距离（并行版本）
        fn manhattan_distance(&self, other: &Self) -> f64 {
            // 使用并行迭代器遍历两个向量的元素
            self.vec
                .par_iter()
                // 将两个向量配对
                .zip(other.vec.par_iter())
                // 计算每对元素差的绝对值
                .map(|(x, y)| (x - y).abs())
                // 求和
                .sum()
        }

        // 计算切比雪夫距离（注意：这里仍然使用串行迭代器，因为 fold 操作）
        fn chebyshev_distance(&self, other: &Self) -> f64 {
            // 遍历两个向量的元素
            self.vec
                .iter()
                // 将两个向量配对
                .zip(other.vec.iter())
                // 计算每对元素差的绝对值
                .map(|(x, y)| (x - y).abs())
                // 使用 fold 找到最大值
                .fold(0.0, f64::max)
        }
    }
}

// 测试模块
#[cfg(test)]
mod tests {
    // 导入要测试的 trait
    use super::VectorDistance;
    // 导入要测试的类型
    use crate::embeddings::Embedding;

    // 创建测试用的嵌入向量
    fn embeddings() -> (Embedding, Embedding) {
        // 第一个嵌入向量
        let embedding_1 = Embedding {
            // 文档内容
            document: "test".to_string(),
            // 向量数据：[1.0, 2.0, 3.0]
            vec: vec![1.0, 2.0, 3.0],
        };

        // 第二个嵌入向量
        let embedding_2 = Embedding {
            // 文档内容
            document: "test".to_string(),
            // 向量数据：[1.0, 5.0, 7.0]
            vec: vec![1.0, 5.0, 7.0],
        };

        // 返回两个嵌入向量
        (embedding_1, embedding_2)
    }

    // 测试点积计算
    #[test]
    fn test_dot_product() {
        // 获取测试用的嵌入向量
        let (embedding_1, embedding_2) = embeddings();

        // 验证点积计算：1*1 + 2*5 + 3*7 = 1 + 10 + 21 = 32
        assert_eq!(embedding_1.dot_product(&embedding_2), 32.0)
    }

    // 测试余弦相似度计算
    #[test]
    fn test_cosine_similarity() {
        // 获取测试用的嵌入向量
        let (embedding_1, embedding_2) = embeddings();

        // 验证余弦相似度计算（非归一化）
        assert_eq!(
            embedding_1.cosine_similarity(&embedding_2, false),
            0.9875414397573881
        )
    }

    // 测试角度距离计算
    #[test]
    fn test_angular_distance() {
        // 获取测试用的嵌入向量
        let (embedding_1, embedding_2) = embeddings();

        // 验证角度距离计算（基于余弦相似度）
        assert_eq!(
            embedding_1.angular_distance(&embedding_2, false),
            0.0502980301830343
        )
    }

    // 测试欧几里得距离计算
    #[test]
    fn test_euclidean_distance() {
        // 获取测试用的嵌入向量
        let (embedding_1, embedding_2) = embeddings();

        // 验证欧几里得距离：√((1-1)² + (2-5)² + (3-7)²) = √(0 + 9 + 16) = √25 = 5
        assert_eq!(embedding_1.euclidean_distance(&embedding_2), 5.0)
    }

    // 测试曼哈顿距离计算
    #[test]
    fn test_manhattan_distance() {
        // 获取测试用的嵌入向量
        let (embedding_1, embedding_2) = embeddings();

        // 验证曼哈顿距离：|1-1| + |2-5| + |3-7| = 0 + 3 + 4 = 7
        assert_eq!(embedding_1.manhattan_distance(&embedding_2), 7.0)
    }

    // 测试切比雪夫距离计算
    #[test]
    fn test_chebyshev_distance() {
        // 获取测试用的嵌入向量
        let (embedding_1, embedding_2) = embeddings();

        // 验证切比雪夫距离：max(|1-1|, |2-5|, |3-7|) = max(0, 3, 4) = 4
        assert_eq!(embedding_1.chebyshev_distance(&embedding_2), 4.0)
    }
}
