//! Web search and content extraction utilities.
//!
//! This module provides:
//! - Data models for web pages and links
//! - Content extraction from HTML
//! - Navigation and pagination detection
//! - Configuration for fetching and extraction
//! - Protocol traits for pluggable components
//! - Run utilities for common operations

mod config;
mod models;
mod protocols;
mod run_utils;

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
pub use run_utils::{
    FetchProgress, SearchResult, SiteMap, calculate_relevance_score, calculate_retry_delay,
    create_error_result, extract_domain, extract_unique_links, filter_relevant_pages,
    same_domain,
};
