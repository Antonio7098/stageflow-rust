//! Web search and content extraction utilities.
//!
//! This module provides:
//! - Data models for web pages and links
//! - Content extraction from HTML
//! - Navigation and pagination detection
//! - Configuration for fetching and extraction

mod config;
mod models;

pub use config::{
    ExtractionConfig, FetchConfig, NavigationConfig, RetryConfig, WebSearchConfig,
};
pub use models::{
    ExtractedLink, NavigationAction, PageMetadata, PaginationInfo, WebPage,
};
