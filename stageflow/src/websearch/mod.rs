//! Web search and content extraction (requires `websearch` feature).

#[cfg(feature = "websearch")]
mod client;
#[cfg(feature = "websearch")]
mod extractor;
#[cfg(feature = "websearch")]
mod fetcher;
#[cfg(feature = "websearch")]
mod models;
#[cfg(feature = "websearch")]
mod navigator;

#[cfg(feature = "websearch")]
pub use client::WebSearchClient;
#[cfg(feature = "websearch")]
pub use extractor::{ContentExtractor, ExtractionConfig, ExtractionResult};
#[cfg(feature = "websearch")]
pub use fetcher::{FetchConfig, FetchResult, Fetcher};
#[cfg(feature = "websearch")]
pub use models::{ExtractedLink, NavigationAction, PageMetadata, PaginationInfo, WebPage};
#[cfg(feature = "websearch")]
pub use navigator::{NavigationConfig, NavigationResult, PageNavigator};
