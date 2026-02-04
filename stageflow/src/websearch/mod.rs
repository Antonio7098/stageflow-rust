//! Web search and content extraction utilities.
//!
//! This module provides:
//! - Data models for web pages and links
//! - Content extraction from HTML
//! - Navigation and pagination detection

mod models;

pub use models::{
    ExtractedLink, NavigationAction, PageMetadata, PaginationInfo, WebPage,
};
