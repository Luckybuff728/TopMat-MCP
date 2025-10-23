// 导入反序列化相关类型
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
// 导入序列化相关类型
use serde::ser::{SerializeSeq, Serializer};
// 导入序列化和反序列化 trait
use serde::{Deserialize, Serialize};
// 导入不可失败错误类型
use std::convert::Infallible;
// 导入格式化相关类型
use std::fmt;
// 导入类型标记类型
use std::marker::PhantomData;
// 导入字符串解析 trait
use std::str::FromStr;

/// 包含单个项目或类型 T 的项目列表的结构体。
/// 如果存在单个项目，`first` 将包含它，`rest` 将为空。
/// 如果存在多个项目，`first` 将包含第一个项目，`rest` 将包含其余项目。
/// 重要：此结构体不能使用空向量创建。
/// OneOrMany 对象只能使用 OneOrMany::from() 或 OneOrMany::try_from() 创建。
// 包含单个项目或类型 T 的项目列表的结构体
// 如果存在单个项目，first 将包含它，rest 将为空
// 如果存在多个项目，first 将包含第一个项目，rest 将包含其余项目
// 重要：此结构体不能使用空向量创建
// OneOrMany 对象只能使用 OneOrMany::from() 或 OneOrMany::try_from() 创建
// 派生比较、调试和克隆 trait
#[derive(PartialEq, Eq, Debug, Clone)]
// 定义一或多个容器结构体，支持泛型类型 T
pub struct OneOrMany<T> {
    /// 列表中的第一个项目。
    // 列表中的第一个项目
    first: T,
    /// 列表中其余的项目。
    // 列表中其余的项目
    rest: Vec<T>,
}

/// 尝试使用空向量创建 OneOrMany 对象时的错误类型。
// 尝试使用空向量创建 OneOrMany 对象时的错误类型
// 派生调试和错误 trait
#[derive(Debug, thiserror::Error)]
// 定义空列表错误，当尝试用空向量创建 OneOrMany 时抛出
#[error("Cannot create OneOrMany with an empty vector.")]
pub struct EmptyListError;

// 为 OneOrMany<T> 实现方法，要求 T 实现 Clone trait
impl<T: Clone> OneOrMany<T> {
    /// 获取列表中的第一个项目。
    // 获取列表中的第一个项目
    pub fn first(&self) -> T {
        // 克隆并返回第一个元素
        self.first.clone()
    }

    /// 获取列表中其余的项目（不包括第一个）。
    // 获取列表中其余的项目（不包括第一个）
    pub fn rest(&self) -> Vec<T> {
        // 克隆并返回其余元素的向量
        self.rest.clone()
    }

    /// 创建 `OneOrMany<T>` 后，将类型 T 的项目添加到 `rest`。
    // 创建 OneOrMany<T> 后，将类型 T 的项目添加到 rest
    pub fn push(&mut self, item: T) {
        // 将项目添加到 rest 向量中
        self.rest.push(item);
    }

    /// 创建 `OneOrMany<T>` 后，在索引处插入类型 T 的项目。
    // 创建 OneOrMany<T> 后，在索引处插入类型 T 的项目
    pub fn insert(&mut self, index: usize, item: T) {
        // 如果索引为 0，则插入到第一个位置
        if index == 0 {
            // 用新项目替换第一个元素，并将旧的第一个元素插入到 rest 的开头
            let old_first = std::mem::replace(&mut self.first, item);
            self.rest.insert(0, old_first);
        } else {
            // 否则插入到 rest 中的相应位置（索引减 1）
            self.rest.insert(index - 1, item);
        }
    }

    /// `OneOrMany<T>` 中所有项目的长度。
    // OneOrMany<T> 中所有项目的长度
    pub fn len(&self) -> usize {
        // 返回 1（第一个元素）加上 rest 的长度
        1 + self.rest.len()
    }

    /// 如果 `OneOrMany<T>` 为空。这将始终为 false，因为您无法创建空的 `OneOrMany<T>`。
    /// 当存在 `len` 方法时需要此方法。
    // 如果 OneOrMany<T> 为空
    // 这将始终为 false，因为您无法创建空的 OneOrMany<T>
    // 当存在 len 方法时需要此方法
    pub fn is_empty(&self) -> bool {
        // 始终返回 false，因为 OneOrMany 不能为空
        false
    }

    /// Create a `OneOrMany` object with a single item of any type.
    // 创建包含单个项目的 OneOrMany 对象
    pub fn one(item: T) -> Self {
        // 创建新的 OneOrMany 对象，first 为传入的项目，rest 为空向量
        OneOrMany {
            first: item,
            rest: vec![],
        }
    }

    /// Create a `OneOrMany` object with a vector of items of any type.
    // 创建包含多个项目的 OneOrMany 对象
    pub fn many<I>(items: I) -> Result<Self, EmptyListError>
    where
        // I 必须实现 IntoIterator<Item = T>
        I: IntoIterator<Item = T>,
    {
        // 将 items 转换为迭代器
        let mut iter = items.into_iter();
        // 创建 OneOrMany 对象
        Ok(OneOrMany {
            // 获取第一个元素，如果没有则返回错误
            first: match iter.next() {
                Some(item) => item,
                None => return Err(EmptyListError),
            },
            // 收集剩余的元素
            rest: iter.collect(),
        })
    }

    /// Merge a list of OneOrMany items into a single OneOrMany item.
    // 将 OneOrMany 项目列表合并为单个 OneOrMany 项目
    pub fn merge<I>(one_or_many_items: I) -> Result<Self, EmptyListError>
    where
        // I 必须实现 IntoIterator<Item = OneOrMany<T>>
        I: IntoIterator<Item = OneOrMany<T>>,
    {
        // 将所有 OneOrMany 项目展平为单个项目列表
        let items = one_or_many_items
            .into_iter()
            .flat_map(|one_or_many| one_or_many.into_iter())
            .collect::<Vec<_>>();

        // 使用 many 方法创建新的 OneOrMany 对象
        OneOrMany::many(items)
    }

    /// Specialized map function for OneOrMany objects.
    ///
    /// Since OneOrMany objects have *atleast* 1 item, using `.collect::<Vec<_>>()` and
    /// `OneOrMany::many()` is fallible resulting in unergonomic uses of `.expect` or `.unwrap`.
    /// This function bypasses those hurdles by directly constructing the `OneOrMany` struct.
    // 专门为 OneOrMany 对象设计的 map 函数
    // 由于 OneOrMany 对象至少有 1 个项目，使用 .collect::<Vec<_>>() 和 OneOrMany::many() 
    // 可能会失败，导致不优雅的 .expect 或 .unwrap 使用
    // 此函数通过直接构造 OneOrMany 结构体来绕过这些障碍
    pub(crate) fn map<U, F: FnMut(T) -> U>(self, mut op: F) -> OneOrMany<U> {
        // 直接构造新的 OneOrMany 对象
        OneOrMany {
            // 对第一个元素应用操作
            first: op(self.first),
            // 对其余元素应用操作并收集
            rest: self.rest.into_iter().map(op).collect(),
        }
    }

    /// Specialized try map function for OneOrMany objects.
    ///
    /// Same as `OneOrMany::map` but fallible.
    // 专门为 OneOrMany 对象设计的 try_map 函数
    // 与 OneOrMany::map 相同，但可能会失败
    pub(crate) fn try_map<U, E, F>(self, mut op: F) -> Result<OneOrMany<U>, E>
    where
        // F 必须实现 FnMut(T) -> Result<U, E>
        F: FnMut(T) -> Result<U, E>,
    {
        // 返回成功的结果，包含转换后的 OneOrMany 对象
        Ok(OneOrMany {
            // 对第一个元素应用操作，如果失败则传播错误
            first: op(self.first)?,
            // 对其余元素应用操作并收集，如果任何操作失败则传播错误
            rest: self
                .rest
                .into_iter()
                .map(op)
                .collect::<Result<Vec<_>, E>>()?,
        })
    }

    // 返回对 OneOrMany 元素的不可变引用的迭代器
    pub fn iter(&self) -> Iter<'_, T> {
        // 创建 Iter 结构体，包含第一个元素的引用和其余元素的迭代器
        Iter {
            first: Some(&self.first),
            rest: self.rest.iter(),
        }
    }

    // 返回对 OneOrMany 元素的可变引用的迭代器
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        // 创建 IterMut 结构体，包含第一个元素的可变引用和其余元素的可变迭代器
        IterMut {
            first: Some(&mut self.first),
            rest: self.rest.iter_mut(),
        }
    }
}

// ================================================================
// OneOrMany 的迭代器实现
//   - OneOrMany<T>::iter() -> 迭代 T 对象的引用
//   - OneOrMany<T>::into_iter() -> 迭代拥有的 T 对象
//   - OneOrMany<T>::iter_mut() -> 迭代 T 对象的可变引用
// ================================================================

/// 由 `OneOrMany::iter()` 调用返回的结构体。
// 由 OneOrMany::iter() 调用返回的结构体
pub struct Iter<'a, T> {
    // 引用。
    // 第一个元素的引用
    first: Option<&'a T>,
    // 其余元素的切片迭代器
    rest: std::slice::Iter<'a, T>,
}

/// 为 `Iter<T>` 实现 `Iterator`。
/// `Iterator` trait 的 Item 类型是 `T` 的引用。
// 为 Iter<T> 实现 Iterator trait
// Iterator trait 的 Item 类型是 T 的引用
impl<'a, T> Iterator for Iter<'a, T> {
    // 迭代器项目的类型是 T 的引用
    type Item = &'a T;

    // 获取下一个元素
    fn next(&mut self) -> Option<Self::Item> {
        // 如果还有第一个元素，则返回它
        if let Some(first) = self.first.take() {
            Some(first)
        } else {
            // 否则返回 rest 中的下一个元素
            self.rest.next()
        }
    }

    // 返回迭代器的大小提示
    fn size_hint(&self) -> (usize, Option<usize>) {
        // 计算第一个元素是否还存在
        let first = if self.first.is_some() { 1 } else { 0 };
        // 计算最大可能的大小
        let max = self.rest.size_hint().1.unwrap_or(0) + first;
        // 根据最大大小返回适当的大小提示
        if max > 0 {
            (1, Some(max))
        } else {
            (0, Some(0))
        }
    }
}

/// 由 `OneOrMany::into_iter()` 调用返回的结构体。
// 由 OneOrMany::into_iter() 调用返回的结构体
pub struct IntoIter<T> {
    // 拥有的。
    // 第一个元素（拥有的）
    first: Option<T>,
    // 其余元素的向量迭代器
    rest: std::vec::IntoIter<T>,
}

/// 为 `IntoIter<T>` 实现 `Iterator`。
// 为 OneOrMany<T> 实现 IntoIterator trait
impl<T> IntoIterator for OneOrMany<T>
where
    // T 必须实现 Clone trait
    T: Clone,
{
    // 迭代器项目的类型是 T
    type Item = T;
    // IntoIter 的类型是 IntoIter<T>
    type IntoIter = IntoIter<T>;

    // 将 OneOrMany 转换为迭代器
    fn into_iter(self) -> Self::IntoIter {
        // 创建 IntoIter 结构体
        IntoIter {
            // 将第一个元素包装在 Some 中
            first: Some(self.first),
            // 将 rest 转换为迭代器
            rest: self.rest.into_iter(),
        }
    }
}

/// 为 `IntoIter<T>` 实现 `Iterator`。
/// `Iterator` trait 的 Item 类型是拥有的 `T`。
// 为 IntoIter<T> 实现 Iterator trait
// Iterator trait 的 Item 类型是拥有的 T
impl<T> Iterator for IntoIter<T>
where
    // T 必须实现 Clone trait
    T: Clone,
{
    // 迭代器项目的类型是 T
    type Item = T;

    // 获取下一个元素
    fn next(&mut self) -> Option<Self::Item> {
        // 匹配第一个元素的处理
        match self.first.take() {
            // 如果还有第一个元素，则返回它
            Some(first) => Some(first),
            // 否则返回 rest 中的下一个元素
            _ => self.rest.next(),
        }
    }

    // 返回迭代器的大小提示
    fn size_hint(&self) -> (usize, Option<usize>) {
        // 计算第一个元素是否还存在
        let first = if self.first.is_some() { 1 } else { 0 };
        // 计算最大可能的大小
        let max = self.rest.size_hint().1.unwrap_or(0) + first;
        // 根据最大大小返回适当的大小提示
        if max > 0 {
            (1, Some(max))
        } else {
            (0, Some(0))
        }
    }
}

/// 由 `OneOrMany::iter_mut()` 调用返回的结构体。
// 由 OneOrMany::iter_mut() 调用返回的结构体
pub struct IterMut<'a, T> {
    // 可变引用。
    // 第一个元素的可变引用
    first: Option<&'a mut T>,
    // 其余元素的可变切片迭代器
    rest: std::slice::IterMut<'a, T>,
}

// 为 `IterMut<T>` 实现 `Iterator`。
// `Iterator` trait 的 Item 类型是 `OneOrMany<T>` 的可变引用。
// 为 IterMut<T> 实现 Iterator trait
// Iterator trait 的 Item 类型是 OneOrMany<T> 的可变引用
impl<'a, T> Iterator for IterMut<'a, T> {
    // 迭代器项目的类型是 T 的可变引用
    type Item = &'a mut T;

    // 获取下一个元素
    fn next(&mut self) -> Option<Self::Item> {
        // 如果还有第一个元素，则返回它
        if let Some(first) = self.first.take() {
            Some(first)
        } else {
            // 否则返回 rest 中的下一个元素
            self.rest.next()
        }
    }

    // 返回迭代器的大小提示
    fn size_hint(&self) -> (usize, Option<usize>) {
        // 计算第一个元素是否还存在
        let first = if self.first.is_some() { 1 } else { 0 };
        // 计算最大可能的大小
        let max = self.rest.size_hint().1.unwrap_or(0) + first;
        // 根据最大大小返回适当的大小提示
        if max > 0 {
            (1, Some(max))
        } else {
            (0, Some(0))
        }
    }
}

// 将 `OneOrMany<T>` 序列化为 json 序列（类似于 `Vec<T>`）
// 将 OneOrMany<T> 序列化为 json 序列（类似于 Vec<T>）
impl<T> Serialize for OneOrMany<T>
where
    // T 必须实现 Serialize 和 Clone trait
    T: Serialize + Clone,
{
    // 序列化方法
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        // S 必须实现 Serializer trait
        S: Serializer,
    {
        // 创建一个序列序列化器，长度为 OneOrMany 对象的长度
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        // 序列化 OneOrMany 对象中的每个元素
        for e in self.iter() {
            seq.serialize_element(e)?;
        }
        // 结束序列序列化
        seq.end()
    }
}

// 将 json 序列反序列化为 `OneOrMany<T>`（类似于 `Vec<T>`）。
// 此外，使用 `OneOrMany::one` 将单个元素（类型为 `T`）反序列化为 `OneOrMany<T>`，
// 这有助于避免在 serde 结构体中使用 `Either<T, OneOrMany<T>>` 类型。
// 将 json 序列反序列化为 OneOrMany<T>（类似于 Vec<T>）
// 此外，使用 OneOrMany::one 将单个元素（类型为 T）反序列化为 OneOrMany<T>
// 这有助于避免在 serde 结构体中使用 Either<T, OneOrMany<T>> 类型
impl<'de, T> Deserialize<'de> for OneOrMany<T>
where
    // T 必须实现 Deserialize 和 Clone trait
    T: Deserialize<'de> + Clone,
{
    // 反序列化方法
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        // D 必须实现 Deserializer trait
        D: Deserializer<'de>,
    {
        // 用于处理反序列化的访问者结构体
        struct OneOrManyVisitor<T>(std::marker::PhantomData<T>);

        // 为 OneOrManyVisitor<T> 实现 Visitor trait
        impl<'de, T> Visitor<'de> for OneOrManyVisitor<T>
        where
            // T 必须实现 Deserialize 和 Clone trait
            T: Deserialize<'de> + Clone,
        {
            // 访问者返回的类型是 OneOrMany<T>
            type Value = OneOrMany<T>;

            // 设置期望的错误消息
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of at least one element")
            }

            // 访问序列并将其反序列化为 OneOrMany
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                // A 必须实现 SeqAccess trait
                A: SeqAccess<'de>,
            {
                // 获取第一个元素
                let first = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                // 收集其余的元素
                let mut rest = Vec::new();
                // 循环获取剩余元素
                while let Some(value) = seq.next_element()? {
                    rest.push(value);
                }

                // 返回反序列化的 OneOrMany 对象
                Ok(OneOrMany { first, rest })
            }
        }

        // 使用访问者将任何类型反序列化为 OneOrMany
        deserializer.deserialize_any(OneOrManyVisitor(std::marker::PhantomData))
    }
}

// 用于 `OneOrMany<T: FromStr>` 字段的特殊反序列化函数
//
// 用法：
// #[derive(Deserialize)]
// struct MyStruct {
//     #[serde(deserialize_with = "string_or_one_or_many")]
//     field: OneOrMany<String>,
// }
// 用于 OneOrMany<T: FromStr> 字段的特殊反序列化函数
// 用法：
// #[derive(Deserialize)]
// struct MyStruct {
//     #[serde(deserialize_with = "string_or_one_or_many")]
//     field: OneOrMany<String>,
// }
pub fn string_or_one_or_many<'de, T, D>(deserializer: D) -> Result<OneOrMany<T>, D::Error>
where
    // T 必须实现 Deserialize、FromStr 和 Clone trait
    T: Deserialize<'de> + FromStr<Err = Infallible> + Clone,
    // D 必须实现 Deserializer trait
    D: Deserializer<'de>,
{
    // 访问者结构体
    struct StringOrOneOrMany<T>(PhantomData<fn() -> T>);

    // 为 StringOrOneOrMany<T> 实现 Visitor trait
    impl<'de, T> Visitor<'de> for StringOrOneOrMany<T>
    where
        // T 必须实现 Deserialize、FromStr 和 Clone trait
        T: Deserialize<'de> + FromStr<Err = Infallible> + Clone,
    {
        // 访问者返回的类型是 OneOrMany<T>
        type Value = OneOrMany<T>;

        // 设置期望的错误消息
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or sequence")
        }

        // 处理字符串输入
        fn visit_str<E>(self, value: &str) -> Result<OneOrMany<T>, E>
        where
            E: de::Error,
        {
            // 从字符串解析项目
            let item = FromStr::from_str(value).map_err(de::Error::custom)?;
            // 返回包含单个项目的 OneOrMany
            Ok(OneOrMany::one(item))
        }

        // 处理序列输入
        fn visit_seq<A>(self, seq: A) -> Result<OneOrMany<T>, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // 反序列化整个序列
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }

        // 处理映射输入
        fn visit_map<M>(self, map: M) -> Result<OneOrMany<T>, M::Error>
        where
            M: MapAccess<'de>,
        {
            // 从映射反序列化单个项目
            let item = Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
            // 返回包含单个项目的 OneOrMany
            Ok(OneOrMany::one(item))
        }
    }

    // 使用访问者进行反序列化
    deserializer.deserialize_any(StringOrOneOrMany(PhantomData))
}

// `string_or_one_or_many` 函数的变体，返回 `Option<OneOrMany<T>>`。
//
// 用法：
// #[derive(Deserialize)]
// struct MyStruct {
//     #[serde(deserialize_with = "string_or_option_one_or_many")]
//     field: Option<OneOrMany<String>>,
// }
// string_or_one_or_many 函数的变体，返回 Option<OneOrMany<T>>
// 用法：
// #[derive(Deserialize)]
// struct MyStruct {
//     #[serde(deserialize_with = "string_or_option_one_or_many")]
//     field: Option<OneOrMany<String>>,
// }
pub fn string_or_option_one_or_many<'de, T, D>(
    deserializer: D,
) -> Result<Option<OneOrMany<T>>, D::Error>
where
    // T 必须实现 Deserialize、FromStr 和 Clone trait
    T: Deserialize<'de> + FromStr<Err = Infallible> + Clone,
    // D 必须实现 Deserializer trait
    D: Deserializer<'de>,
{
    // 访问者结构体
    struct StringOrOptionOneOrMany<T>(PhantomData<fn() -> T>);

    // 为 StringOrOptionOneOrMany<T> 实现 Visitor trait
    impl<'de, T> Visitor<'de> for StringOrOptionOneOrMany<T>
    where
        // T 必须实现 Deserialize、FromStr 和 Clone trait
        T: Deserialize<'de> + FromStr<Err = Infallible> + Clone,
    {
        // 访问者返回的类型是 Option<OneOrMany<T>>
        type Value = Option<OneOrMany<T>>;

        // 设置期望的错误消息
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null, a string, or a sequence")
        }

        // 处理 null 值
        fn visit_none<E>(self) -> Result<Option<OneOrMany<T>>, E>
        where
            E: de::Error,
        {
            // 返回 None
            Ok(None)
        }

        // 处理单位值
        fn visit_unit<E>(self) -> Result<Option<OneOrMany<T>>, E>
        where
            E: de::Error,
        {
            // 返回 None
            Ok(None)
        }

        // 处理 Some 值
        fn visit_some<D>(self, deserializer: D) -> Result<Option<OneOrMany<T>>, D::Error>
        where
            D: Deserializer<'de>,
        {
            // 使用 string_or_one_or_many 进行反序列化并包装在 Some 中
            string_or_one_or_many(deserializer).map(Some)
        }
    }

    // 使用访问者进行选项反序列化
    deserializer.deserialize_option(StringOrOptionOneOrMany(PhantomData))
}

// 条件编译：仅在测试时编译
#[cfg(test)]
mod test {
    // 导入 serde 相关类型
    use serde::{self, Deserialize};
    // 导入 serde_json 的 json 宏
    use serde_json::json;

    // 导入父模块的所有内容
    use super::*;

    // 测试单个元素的 OneOrMany
    #[test]
    fn test_single() {
        // 创建一个包含单个字符串的 OneOrMany
        let one_or_many = OneOrMany::one("hello".to_string());

        // 验证迭代器计数为 1
        assert_eq!(one_or_many.iter().count(), 1);

        // 验证每个元素都是 "hello"
        one_or_many.iter().for_each(|i| {
            assert_eq!(i, "hello");
        });
    }

    // 测试多个元素的 OneOrMany
    #[test]
    fn test() {
        // 创建一个包含多个字符串的 OneOrMany
        let one_or_many = OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        // 验证迭代器计数为 2
        assert_eq!(one_or_many.iter().count(), 2);

        // 验证每个元素的内容
        one_or_many.iter().enumerate().for_each(|(i, item)| {
            // 验证第一个元素
            if i == 0 {
                assert_eq!(item, "hello");
            }
            // 验证第二个元素
            if i == 1 {
                assert_eq!(item, "word");
            }
        });
    }

    // 测试迭代器的大小提示
    #[test]
    fn test_size_hint() {
        // 创建单个元素的 OneOrMany
        let foo = "bar".to_string();
        let one_or_many = OneOrMany::one(foo);
        // 获取迭代器的大小提示
        let size_hint = one_or_many.iter().size_hint();
        // 验证最小大小为 1
        assert_eq!(size_hint.0, 1);
        // 验证最大大小为 Some(1)
        assert_eq!(size_hint.1, Some(1));

        // 创建多个元素的 OneOrMany
        let vec = vec!["foo".to_string(), "bar".to_string(), "baz".to_string()];
        let mut one_or_many = OneOrMany::many(vec).expect("this should never fail");
        // 获取迭代器的大小提示
        let size_hint = one_or_many.iter().size_hint();
        // 验证最小大小为 1
        assert_eq!(size_hint.0, 1);
        // 验证最大大小为 Some(3)
        assert_eq!(size_hint.1, Some(3));

        // 测试 into_iter 的大小提示
        let size_hint = one_or_many.clone().into_iter().size_hint();
        assert_eq!(size_hint.0, 1);
        assert_eq!(size_hint.1, Some(3));

        // 测试 iter_mut 的大小提示
        let size_hint = one_or_many.iter_mut().size_hint();
        assert_eq!(size_hint.0, 1);
        assert_eq!(size_hint.1, Some(3));
    }

    // 测试单个元素的 into_iter
    #[test]
    fn test_one_or_many_into_iter_single() {
        // 创建单个元素的 OneOrMany
        let one_or_many = OneOrMany::one("hello".to_string());

        // 验证 into_iter 计数为 1
        assert_eq!(one_or_many.clone().into_iter().count(), 1);

        // 验证每个元素都是 "hello"
        one_or_many.into_iter().for_each(|i| {
            assert_eq!(i, "hello".to_string());
        });
    }

    // 测试多个元素的 into_iter
    #[test]
    fn test_one_or_many_into_iter() {
        // 创建多个元素的 OneOrMany
        let one_or_many = OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        // 验证 into_iter 计数为 2
        assert_eq!(one_or_many.clone().into_iter().count(), 2);

        // 验证每个元素的内容
        one_or_many.into_iter().enumerate().for_each(|(i, item)| {
            // 验证第一个元素
            if i == 0 {
                assert_eq!(item, "hello".to_string());
            }
            // 验证第二个元素
            if i == 1 {
                assert_eq!(item, "word".to_string());
            }
        });
    }

    // 测试 OneOrMany 的合并功能
    #[test]
    fn test_one_or_many_merge() {
        // 创建第一个 OneOrMany
        let one_or_many_1 = OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        // 创建第二个 OneOrMany
        let one_or_many_2 = OneOrMany::one("sup".to_string());

        // 合并两个 OneOrMany
        let merged = OneOrMany::merge(vec![one_or_many_1, one_or_many_2]).unwrap();

        // 验证合并后的计数为 3
        assert_eq!(merged.iter().count(), 3);

        // 验证合并后的元素内容
        merged.iter().enumerate().for_each(|(i, item)| {
            // 验证第一个元素
            if i == 0 {
                assert_eq!(item, "hello");
            }
            // 验证第二个元素
            if i == 1 {
                assert_eq!(item, "word");
            }
            // 验证第三个元素
            if i == 2 {
                assert_eq!(item, "sup");
            }
        });
    }

    // 测试单个元素的可变迭代器
    #[test]
    fn test_mut_single() {
        // 创建单个元素的可变 OneOrMany
        let mut one_or_many = OneOrMany::one("hello".to_string());

        // 验证可变迭代器计数为 1
        assert_eq!(one_or_many.iter_mut().count(), 1);

        // 验证每个元素都是 "hello"
        one_or_many.iter_mut().for_each(|i| {
            assert_eq!(i, "hello");
        });
    }

    // 测试多个元素的可变迭代器
    #[test]
    fn test_mut() {
        // 创建多个元素的可变 OneOrMany
        let mut one_or_many =
            OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        // 验证可变迭代器计数为 2
        assert_eq!(one_or_many.iter_mut().count(), 2);

        // 修改第一个元素并验证
        one_or_many.iter_mut().enumerate().for_each(|(i, item)| {
            // 修改第一个元素
            if i == 0 {
                item.push_str(" world");
                assert_eq!(item, "hello world");
            }
            // 验证第二个元素
            if i == 1 {
                assert_eq!(item, "word");
            }
        });
    }

    // 测试空向量错误
    #[test]
    fn test_one_or_many_error() {
        // 验证使用空向量创建 OneOrMany 会返回错误
        assert!(OneOrMany::<String>::many(vec![]).is_err())
    }

    // 测试单个元素的长度
    #[test]
    fn test_len_single() {
        // 创建单个元素的 OneOrMany
        let one_or_many = OneOrMany::one("hello".to_string());

        // 验证长度为 1
        assert_eq!(one_or_many.len(), 1);
    }

    // 测试多个元素的长度
    #[test]
    fn test_len_many() {
        // 创建多个元素的 OneOrMany
        let one_or_many = OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        // 验证长度为 2
        assert_eq!(one_or_many.len(), 2);
    }

    // 测试反序列化
    // 测试列表反序列化
    #[test]
    fn test_deserialize_list() {
        // 创建包含数字数组的 JSON 数据
        let json_data = json!({"field": [1, 2, 3]});
        // 反序列化为 OneOrMany<i32>
        let one_or_many: OneOrMany<i32> =
            serde_json::from_value(json_data["field"].clone()).unwrap();

        // 验证长度为 3
        assert_eq!(one_or_many.len(), 3);
        // 验证第一个元素为 1
        assert_eq!(one_or_many.first(), 1);
        // 验证其余元素为 [2, 3]
        assert_eq!(one_or_many.rest(), vec![2, 3]);
    }

    // 测试映射列表反序列化
    #[test]
    fn test_deserialize_list_of_maps() {
        // 创建包含映射数组的 JSON 数据
        let json_data = json!({"field": [{"key": "value1"}, {"key": "value2"}]});
        // 反序列化为 OneOrMany<serde_json::Value>
        let one_or_many: OneOrMany<serde_json::Value> =
            serde_json::from_value(json_data["field"].clone()).unwrap();

        // 验证长度为 2
        assert_eq!(one_or_many.len(), 2);
        // 验证第一个元素
        assert_eq!(one_or_many.first(), json!({"key": "value1"}));
        // 验证其余元素
        assert_eq!(one_or_many.rest(), vec![json!({"key": "value2"})]);
    }

    // 派生调试、反序列化和相等比较 trait
    #[derive(Debug, Deserialize, PartialEq)]
    // 测试用的虚拟结构体
    struct DummyStruct {
        // 使用 string_or_one_or_many 进行反序列化
        #[serde(deserialize_with = "string_or_one_or_many")]
        // OneOrMany<DummyString> 字段
        field: OneOrMany<DummyString>,
    }

    // 派生调试、反序列化和相等比较 trait
    #[derive(Debug, Deserialize, PartialEq)]
    // 测试用的可选虚拟结构体
    struct DummyStructOption {
        // 使用 string_or_option_one_or_many 进行反序列化
        #[serde(deserialize_with = "string_or_option_one_or_many")]
        // Option<OneOrMany<DummyString>> 字段
        field: Option<OneOrMany<DummyString>>,
    }

    // 派生调试、克隆、反序列化和相等比较 trait
    #[derive(Debug, Clone, Deserialize, PartialEq)]
    // 测试用的虚拟字符串结构体
    struct DummyString {
        // 字符串字段
        pub string: String,
    }

    // 为 DummyString 实现 FromStr trait
    impl FromStr for DummyString {
        // 错误类型为 Infallible（不可失败）
        type Err = Infallible;

        // 从字符串创建 DummyString
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            // 返回新的 DummyString 实例
            Ok(DummyString {
                string: s.to_string(),
            })
        }
    }

    // 派生调试、反序列化和相等比较 trait
    #[derive(Debug, Deserialize, PartialEq)]
    // 使用标签和重命名策略
    #[serde(tag = "role", rename_all = "lowercase")]
    // 测试用的虚拟消息枚举
    enum DummyMessage {
        // 助手消息变体
        Assistant {
            // 使用 string_or_option_one_or_many 进行反序列化
            #[serde(deserialize_with = "string_or_option_one_or_many")]
            // 可选内容字段
            content: Option<OneOrMany<DummyString>>,
        },
    }

    // 测试反序列化单位值
    #[test]
    fn test_deserialize_unit() {
        // JSON 字符串，包含 null 内容
        let raw_json = r#"
        {
            "role": "assistant",
            "content": null
        }
        "#;
        // 反序列化为 DummyMessage
        let dummy: DummyMessage = serde_json::from_str(raw_json).unwrap();

        // 验证反序列化结果
        assert_eq!(dummy, DummyMessage::Assistant { content: None });
    }

    // 测试反序列化字符串
    #[test]
    fn test_deserialize_string() {
        // 创建包含字符串的 JSON 数据
        let json_data = json!({"field": "hello"});
        // 反序列化为 DummyStruct
        let dummy: DummyStruct = serde_json::from_value(json_data).unwrap();

        // 验证字段长度为 1
        assert_eq!(dummy.field.len(), 1);
        // 验证第一个元素
        assert_eq!(dummy.field.first(), DummyString::from_str("hello").unwrap());
    }

    // 测试反序列化可选字符串
    #[test]
    fn test_deserialize_string_option() {
        // 创建包含字符串的 JSON 数据
        let json_data = json!({"field": "hello"});
        // 反序列化为 DummyStructOption
        let dummy: DummyStructOption = serde_json::from_value(json_data).unwrap();

        // 验证字段存在
        assert!(dummy.field.is_some());
        // 获取字段值
        let field = dummy.field.unwrap();
        // 验证字段长度为 1
        assert_eq!(field.len(), 1);
        // 验证第一个元素
        assert_eq!(field.first(), DummyString::from_str("hello").unwrap());
    }

    // 测试反序列化可选列表
    #[test]
    fn test_deserialize_list_option() {
        // 创建包含对象数组的 JSON 数据
        let json_data = json!({"field": [{"string": "hello"}, {"string": "world"}]});
        // 反序列化为 DummyStructOption
        let dummy: DummyStructOption = serde_json::from_value(json_data).unwrap();

        // 验证字段存在
        assert!(dummy.field.is_some());
        // 获取字段值
        let field = dummy.field.unwrap();
        // 验证字段长度为 2
        assert_eq!(field.len(), 2);
        // 验证第一个元素
        assert_eq!(field.first(), DummyString::from_str("hello").unwrap());
        // 验证其余元素
        assert_eq!(field.rest(), vec![DummyString::from_str("world").unwrap()]);
    }

    // 测试反序列化 null 选项
    #[test]
    fn test_deserialize_null_option() {
        // 创建包含 null 的 JSON 数据
        let json_data = json!({"field": null});
        // 反序列化为 DummyStructOption
        let dummy: DummyStructOption = serde_json::from_value(json_data).unwrap();

        // 验证字段为 None
        assert!(dummy.field.is_none());
    }
}
