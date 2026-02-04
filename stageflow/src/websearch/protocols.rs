//! Protocol traits for websearch components.
//!
//! These traits define the interfaces for fetching, extraction, and navigation
//! components, allowing for pluggable implementations.

use async_trait::async_trait;
use std::collections::HashMap;

use super::config::{ExtractionConfig, FetchConfig, NavigationConfig};
use super::models::{ExtractedLink, NavigationAction, PageMetadata, PaginationInfo};
use crate::errors::StageflowError;

/// Result of a fetch operation.
#[derive(Debug, Clone)]
pub struct FetchResult {
    /// HTTP status code.
    pub status_code: u16,
    /// Response headers.
    pub headers: HashMap<String, String>,
    /// Response body as text.
    pub text: String,
    /// Final URL after redirects.
    pub final_url: String,
    /// Content type from headers.
    pub content_type: Option<String>,
    /// Time taken to fetch in milliseconds.
    pub duration_ms: f64,
}

impl FetchResult {
    /// Whether the response is HTML.
    #[must_use]
    pub fn is_html(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.contains("text/html") || ct.contains("application/xhtml"))
            .unwrap_or(false)
    }

    /// Whether the fetch was successful (2xx status).
    #[must_use]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }
}

/// Result of content extraction.
#[derive(Debug, Clone, Default)]
pub struct ExtractionResult {
    /// Extracted markdown content.
    pub markdown: String,
    /// Plain text content.
    pub plain_text: String,
    /// Page metadata.
    pub metadata: PageMetadata,
    /// Extracted links.
    pub links: Vec<ExtractedLink>,
    /// Word count.
    pub word_count: usize,
    /// Heading outline.
    pub heading_outline: Vec<HeadingOutline>,
}

/// A heading in the document outline.
#[derive(Debug, Clone)]
pub struct HeadingOutline {
    /// Heading level (1-6).
    pub level: u8,
    /// Heading text.
    pub text: String,
    /// Anchor ID if available.
    pub id: Option<String>,
}

impl ExtractionResult {
    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        dict.insert("markdown".to_string(), serde_json::json!(self.markdown));
        dict.insert("plain_text".to_string(), serde_json::json!(self.plain_text));
        dict.insert("metadata".to_string(), serde_json::json!(self.metadata.to_dict()));
        dict.insert("links".to_string(), serde_json::json!(
            self.links.iter().map(|l| l.to_dict()).collect::<Vec<_>>()
        ));
        dict.insert("word_count".to_string(), serde_json::json!(self.word_count));
        dict.insert("heading_outline".to_string(), serde_json::json!(
            self.heading_outline.iter().map(|h| {
                serde_json::json!({
                    "level": h.level,
                    "text": h.text,
                    "id": h.id,
                })
            }).collect::<Vec<_>>()
        ));
        dict
    }
}

/// Result of navigation analysis.
#[derive(Debug, Clone, Default)]
pub struct NavigationResult {
    /// Available navigation actions.
    pub actions: Vec<NavigationAction>,
    /// Pagination info if detected.
    pub pagination: Option<PaginationInfo>,
    /// Main content selector if detected.
    pub main_content_selector: Option<String>,
    /// Navigation links.
    pub nav_links: Vec<ExtractedLink>,
    /// Breadcrumb links.
    pub breadcrumbs: Vec<ExtractedLink>,
}

impl NavigationResult {
    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        dict.insert("actions".to_string(), serde_json::json!(
            self.actions.iter().map(|a| a.to_dict()).collect::<Vec<_>>()
        ));
        if let Some(ref p) = self.pagination {
            dict.insert("pagination".to_string(), serde_json::json!(p.to_dict()));
        }
        if let Some(ref s) = self.main_content_selector {
            dict.insert("main_content_selector".to_string(), serde_json::json!(s));
        }
        dict.insert("nav_links".to_string(), serde_json::json!(
            self.nav_links.iter().map(|l| l.to_dict()).collect::<Vec<_>>()
        ));
        dict.insert("breadcrumbs".to_string(), serde_json::json!(
            self.breadcrumbs.iter().map(|l| l.to_dict()).collect::<Vec<_>>()
        ));
        dict
    }
}

/// Protocol for HTTP fetching.
#[async_trait]
pub trait Fetcher: Send + Sync {
    /// Fetches a URL and returns the result.
    async fn fetch(
        &self,
        url: &str,
        timeout: Option<f64>,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<FetchResult, StageflowError>;

    /// Gets the configuration.
    fn config(&self) -> &FetchConfig;
}

/// Protocol for content extraction.
pub trait ContentExtractor: Send + Sync {
    /// Extracts content from HTML.
    fn extract(
        &self,
        html: &str,
        base_url: Option<&str>,
        selector: Option<&str>,
    ) -> ExtractionResult;

    /// Extracts only metadata from HTML.
    fn extract_metadata(&self, html: &str) -> PageMetadata;

    /// Extracts only links from HTML.
    fn extract_links(
        &self,
        html: &str,
        base_url: Option<&str>,
        selector: Option<&str>,
    ) -> Vec<ExtractedLink>;

    /// Gets the configuration.
    fn config(&self) -> &ExtractionConfig;
}

/// Protocol for page navigation analysis.
pub trait Navigator: Send + Sync {
    /// Analyzes a page for navigation options.
    fn analyze(&self, html: &str, base_url: Option<&str>) -> NavigationResult;

    /// Gets the configuration.
    fn config(&self) -> &NavigationConfig;
}

/// Observability callbacks for fetch operations.
pub trait FetchObserver: Send + Sync {
    /// Called when a fetch starts.
    fn on_fetch_start(&self, url: &str, request_id: &str);

    /// Called when a fetch completes.
    fn on_fetch_complete(&self, url: &str, request_id: &str, duration_ms: f64, status_code: u16);

    /// Called when a fetch fails.
    fn on_fetch_error(&self, url: &str, request_id: &str, error: &str);

    /// Called when extraction completes.
    fn on_extract_complete(
        &self,
        url: &str,
        request_id: &str,
        duration_ms: f64,
        markdown_len: usize,
        links_count: usize,
    );
}

/// No-op implementation of FetchObserver.
#[derive(Debug, Clone, Default)]
pub struct NoOpFetchObserver;

impl FetchObserver for NoOpFetchObserver {
    fn on_fetch_start(&self, _url: &str, _request_id: &str) {}
    fn on_fetch_complete(&self, _url: &str, _request_id: &str, _duration_ms: f64, _status_code: u16) {}
    fn on_fetch_error(&self, _url: &str, _request_id: &str, _error: &str) {}
    fn on_extract_complete(&self, _url: &str, _request_id: &str, _duration_ms: f64, _markdown_len: usize, _links_count: usize) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_result_is_html() {
        let html_result = FetchResult {
            status_code: 200,
            headers: HashMap::new(),
            text: "<html></html>".to_string(),
            final_url: "https://example.com".to_string(),
            content_type: Some("text/html; charset=utf-8".to_string()),
            duration_ms: 100.0,
        };
        assert!(html_result.is_html());

        let json_result = FetchResult {
            content_type: Some("application/json".to_string()),
            ..html_result.clone()
        };
        assert!(!json_result.is_html());
    }

    #[test]
    fn test_fetch_result_is_success() {
        let success = FetchResult {
            status_code: 200,
            headers: HashMap::new(),
            text: String::new(),
            final_url: String::new(),
            content_type: None,
            duration_ms: 0.0,
        };
        assert!(success.is_success());

        let not_found = FetchResult {
            status_code: 404,
            ..success.clone()
        };
        assert!(!not_found.is_success());

        let redirect = FetchResult {
            status_code: 301,
            ..success
        };
        assert!(!redirect.is_success());
    }

    #[test]
    fn test_extraction_result_to_dict() {
        let result = ExtractionResult {
            markdown: "# Hello".to_string(),
            plain_text: "Hello".to_string(),
            word_count: 1,
            ..Default::default()
        };

        let dict = result.to_dict();
        assert_eq!(dict.get("markdown"), Some(&serde_json::json!("# Hello")));
        assert_eq!(dict.get("word_count"), Some(&serde_json::json!(1)));
    }

    #[test]
    fn test_navigation_result_to_dict() {
        let result = NavigationResult {
            pagination: Some(PaginationInfo {
                current_page: 1,
                next_url: Some("https://example.com/page/2".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let dict = result.to_dict();
        assert!(dict.contains_key("pagination"));
        assert!(dict.contains_key("actions"));
    }

    #[test]
    fn test_noop_observer() {
        let observer = NoOpFetchObserver;
        observer.on_fetch_start("https://example.com", "req-1");
        observer.on_fetch_complete("https://example.com", "req-1", 100.0, 200);
        observer.on_fetch_error("https://example.com", "req-1", "error");
        observer.on_extract_complete("https://example.com", "req-1", 50.0, 1000, 10);
        // Should not panic
    }
}
