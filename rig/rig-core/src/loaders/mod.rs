//! 此模块提供用于加载和预处理文件的实用结构体。
//!
//! `FileLoader` 结构体可用于定义从磁盘加载任何类型文件的通用接口，
//! 以及对文件执行最小预处理，例如读取其内容、忽略错误
//! 并跟踪文件路径及其内容。
//!
//! `PdfFileLoader` 的工作方式类似于 [FileLoader]，但专门设计用于加载 PDF
//! 文件。此加载器还提供 PDF 特定的预处理方法，用于将 PDF 拆分为页面
//! 并跟踪页码及其内容。
//!
//! 注意：`PdfFileLoader` 需要在 `Cargo.toml` 文件中启用 `pdf` 功能。
//!
//! `EpubFileLoader` 的工作方式类似于 `FileLoader`，但专门设计用于加载 EPUB
//! 文件。此加载器还提供 EPUB 特定的预处理方法，用于将 EPUB 拆分为章节
//! 并跟踪章节号及其内容。
//!
//! 注意：EpubFileLoader 需要在 `Cargo.toml` 文件中启用 `epub` 功能。

pub mod file;

pub use file::FileLoader;

#[cfg(feature = "pdf")]
#[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
pub mod pdf;

#[cfg(feature = "pdf")]
pub use pdf::PdfFileLoader;

#[cfg(feature = "epub")]
#[cfg_attr(docsrs, doc(cfg(feature = "epub")))]
pub mod epub;

#[cfg(feature = "epub")]
pub use epub::{EpubFileLoader, RawTextProcessor, StripXmlProcessor, TextProcessor};
