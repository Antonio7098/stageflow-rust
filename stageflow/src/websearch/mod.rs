//! Web search and content extraction utilities.
//!
//! This module provides:
//! - Data models for web pages and links
//! - Content extraction from HTML
//! - Navigation and pagination detection
//! - Configuration for fetching and extraction
//! - Protocol traits for pluggable components

mod config;
mod models;
mod protocols;

pub use config::{
    ExtractionConfig, FetchConfig, NavigationConfig, RetryConfig, WebSearchConfig,
};
pub use models::{
    ExtractedLink, NavigationAction, PageMetadata, PaginationInfo, WebPage,
};
pub use protocols::{
    ContentExtractor, ExtractionResult, FetchObserver, FetchResult, Fetcher,
    HeadingOutline, NavigationResult, Navigator, NoOpFetchObserver,
};
