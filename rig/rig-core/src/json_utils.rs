// 导入反序列化相关类型
use serde::Deserialize;
// 导入反序列化器的相关类型和 trait
use serde::de::{self, Deserializer, SeqAccess, Visitor};
// 导入不可失败错误类型
use std::convert::Infallible;
// 导入格式化相关类型
use std::fmt;
// 导入类型标记类型
use std::marker::PhantomData;
// 导入字符串解析 trait
use std::str::FromStr;

// 合并两个 JSON 值，返回合并后的值
pub fn merge(a: serde_json::Value, b: serde_json::Value) -> serde_json::Value {
    // 匹配两个 JSON 值的类型
    match (a, b) {
        // 如果两个值都是对象，则合并它们的键值对
        (serde_json::Value::Object(mut a_map), serde_json::Value::Object(b_map)) => {
            // 遍历 b 的所有键值对并插入到 a 中
            b_map.into_iter().for_each(|(key, value)| {
                a_map.insert(key, value);
            });
            // 返回合并后的对象
            serde_json::Value::Object(a_map)
        }
        // 如果 a 不是对象或 b 不是对象，则返回 a
        (a, _) => a,
    }
}

// 就地合并两个 JSON 值，修改第一个值
pub fn merge_inplace(a: &mut serde_json::Value, b: serde_json::Value) {
    // 如果两个值都是对象，则合并它们的键值对
    if let (serde_json::Value::Object(a_map), serde_json::Value::Object(b_map)) = (a, b) {
        // 遍历 b 的所有键值对并插入到 a 中
        b_map.into_iter().for_each(|(key, value)| {
            a_map.insert(key, value);
        });
    }
}

/// 此模块在原始 JSON 对象被序列化和反序列化为
/// 字符串（如 `"{\"key\": \"value\"}"`）的情况下很有用。这可能看起来很奇怪，但实际上这就是某些
/// 提供商（如 OpenAI）返回函数参数的方式（出于某种原因）。
// 此模块在原始 JSON 对象被序列化和反序列化为字符串的情况下很有用
// 这可能看起来很奇怪，但实际上这就是某些提供商（如 OpenAI）返回函数参数的方式
pub mod stringified_json {
    // 导入 serde 相关类型
    use serde::{self, Deserialize, Deserializer, Serializer};

    // 序列化 JSON 值为字符串
    pub fn serialize<S>(value: &serde_json::Value, serializer: S) -> Result<S::Ok, S::Error>
    where
        // S 必须实现 Serializer trait
        S: Serializer,
    {
        // 将 JSON 值转换为字符串
        let s = value.to_string();
        // 序列化为字符串
        serializer.serialize_str(&s)
    }

    // 从字符串反序列化为 JSON 值
    pub fn deserialize<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
    where
        // D 必须实现 Deserializer trait
        D: Deserializer<'de>,
    {
        // 首先反序列化为字符串
        let s = String::deserialize(deserializer)?;
        // 然后将字符串解析为 JSON 值
        serde_json::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// 反序列化函数，支持字符串或向量
pub fn string_or_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    // T 必须实现 Deserialize 和 FromStr trait
    T: Deserialize<'de> + FromStr<Err = Infallible>,
    // D 必须实现 Deserializer trait
    D: Deserializer<'de>,
{
    // 定义访问者结构体
    struct StringOrVec<T>(PhantomData<fn() -> T>);

    // 为 StringOrVec 实现 Visitor trait
    impl<'de, T> Visitor<'de> for StringOrVec<T>
    where
        // T 必须实现 Deserialize 和 FromStr trait
        T: Deserialize<'de> + FromStr<Err = Infallible>,
    {
        // 返回类型为 Vec<T>
        type Value = Vec<T>;

        // 设置期望的错误消息
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string, sequence, or null")
        }

        // 处理字符串输入
        fn visit_str<E>(self, value: &str) -> Result<Vec<T>, E>
        where
            E: de::Error,
        {
            // 从字符串解析单个项目
            let item = FromStr::from_str(value).map_err(de::Error::custom)?;
            // 返回包含单个项目的向量
            Ok(vec![item])
        }

        // 处理序列输入
        fn visit_seq<A>(self, seq: A) -> Result<Vec<T>, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // 反序列化整个序列
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }

        // 处理 null 值
        fn visit_none<E>(self) -> Result<Vec<T>, E>
        where
            E: de::Error,
        {
            // 返回空向量
            Ok(vec![])
        }

        // 处理单位值
        fn visit_unit<E>(self) -> Result<Vec<T>, E>
        where
            E: de::Error,
        {
            // 返回空向量
            Ok(vec![])
        }
    }

    // 使用访问者进行反序列化
    deserializer.deserialize_any(StringOrVec(PhantomData))
}

// 反序列化函数，支持 null 或向量
pub fn null_or_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    // T 必须实现 Deserialize trait
    T: Deserialize<'de>,
    // D 必须实现 Deserializer trait
    D: Deserializer<'de>,
{
    // 定义访问者结构体
    struct NullOrVec<T>(PhantomData<fn() -> T>);

    // 为 NullOrVec 实现 Visitor trait
    impl<'de, T> Visitor<'de> for NullOrVec<T>
    where
        // T 必须实现 Deserialize trait
        T: Deserialize<'de>,
    {
        // 返回类型为 Vec<T>
        type Value = Vec<T>;

        // 设置期望的错误消息
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence or null")
        }

        // 处理序列输入
        fn visit_seq<A>(self, seq: A) -> Result<Vec<T>, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // 反序列化整个序列
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }

        // 处理 null 值
        fn visit_none<E>(self) -> Result<Vec<T>, E>
        where
            E: de::Error,
        {
            // 返回空向量
            Ok(vec![])
        }

        // 处理单位值
        fn visit_unit<E>(self) -> Result<Vec<T>, E>
        where
            E: de::Error,
        {
            // 返回空向量
            Ok(vec![])
        }
    }

    // 使用访问者进行反序列化
    deserializer.deserialize_any(NullOrVec(PhantomData))
}

// 条件编译：仅在测试时编译
#[cfg(test)]
mod tests {
    // 导入父模块的所有内容
    use super::*;
    // 导入 serde 的序列化和反序列化 trait
    use serde::{Deserialize, Serialize};

    // 派生 Serialize, Deserialize, Debug, PartialEq trait
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    // 测试用的虚拟结构体
    struct Dummy {
        // 使用 stringified_json 模块进行序列化/反序列化
        #[serde(with = "stringified_json")]
        // JSON 数据字段
        data: serde_json::Value,
    }

    // 测试合并函数
    #[test]
    fn test_merge() {
        // 创建第一个 JSON 对象
        let a = serde_json::json!({"key1": "value1"});
        // 创建第二个 JSON 对象
        let b = serde_json::json!({"key2": "value2"});
        // 合并两个对象
        let result = merge(a, b);
        // 期望的结果
        let expected = serde_json::json!({"key1": "value1", "key2": "value2"});
        // 验证结果
        assert_eq!(result, expected);
    }

    // 测试就地合并函数
    #[test]
    fn test_merge_inplace() {
        // 创建可变的第一个 JSON 对象
        let mut a = serde_json::json!({"key1": "value1"});
        // 创建第二个 JSON 对象
        let b = serde_json::json!({"key2": "value2"});
        // 就地合并
        merge_inplace(&mut a, b);
        // 期望的结果
        let expected = serde_json::json!({"key1": "value1", "key2": "value2"});
        // 验证结果
        assert_eq!(a, expected);
    }

    // 测试字符串化 JSON 序列化
    #[test]
    fn test_stringified_json_serialize() {
        // 创建测试对象
        let dummy = Dummy {
            data: serde_json::json!({"key": "value"}),
        };
        // 序列化为字符串
        let serialized = serde_json::to_string(&dummy).unwrap();
        // 期望的序列化结果
        let expected = r#"{"data":"{\"key\":\"value\"}"}"#;
        // 验证结果
        assert_eq!(serialized, expected);
    }

    // 测试字符串化 JSON 反序列化
    #[test]
    fn test_stringified_json_deserialize() {
        // JSON 字符串
        let json_str = r#"{"data":"{\"key\":\"value\"}"}"#;
        // 反序列化
        let dummy: Dummy = serde_json::from_str(json_str).unwrap();
        // 期望的结果
        let expected = Dummy {
            data: serde_json::json!({"key": "value"}),
        };
        // 验证结果
        assert_eq!(dummy, expected);
    }
}
