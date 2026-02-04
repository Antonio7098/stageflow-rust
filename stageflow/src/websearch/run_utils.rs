//! Utility functions for running web searches and content extraction.
//!
//! These utilities provide high-level functions for common web search operations.

use super::models::{ExtractedLink, WebPage};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Progress information for fetch operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FetchProgress {
    /// Number of completed fetches.
    pub completed: usize,
    /// Total number of fetches.
    pub total: usize,
    /// Currently fetching URL.
    pub current_url: Option<String>,
    /// Number of successful fetches.
    pub success_count: usize,
    /// Number of failed fetches.
    pub error_count: usize,
    /// Elapsed time in milliseconds.
    pub elapsed_ms: f64,
}

impl FetchProgress {
    /// Creates new progress tracker.
    #[must_use]
    pub fn new(total: usize) -> Self {
        Self {
            total,
            ..Default::default()
        }
    }

    /// Returns the completion percentage.
    #[must_use]
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.completed as f64 / self.total as f64) * 100.0
        }
    }

    /// Updates progress with a successful fetch.
    pub fn record_success(&mut self, url: &str, elapsed_ms: f64) {
        self.completed += 1;
        self.success_count += 1;
        self.current_url = Some(url.to_string());
        self.elapsed_ms = elapsed_ms;
    }

    /// Updates progress with a failed fetch.
    pub fn record_error(&mut self, url: &str, elapsed_ms: f64) {
        self.completed += 1;
        self.error_count += 1;
        self.current_url = Some(url.to_string());
        self.elapsed_ms = elapsed_ms;
    }
}

/// Result of a search and extraction operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The search query.
    pub query: String,
    /// All fetched pages.
    pub pages: Vec<WebPage>,
    /// Pages matching the relevance threshold.
    pub relevant_pages: Vec<WebPage>,
    /// Total word count across relevant pages.
    pub total_words: usize,
    /// Total duration in milliseconds.
    pub duration_ms: f64,
}

impl SearchResult {
    /// Creates a new search result.
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            pages: Vec::new(),
            relevant_pages: Vec::new(),
            total_words: 0,
            duration_ms: 0.0,
        }
    }

    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        dict.insert("query".to_string(), serde_json::json!(self.query));
        dict.insert("pages_fetched".to_string(), serde_json::json!(self.pages.len()));
        dict.insert("relevant_pages".to_string(), serde_json::json!(self.relevant_pages.len()));
        dict.insert("total_words".to_string(), serde_json::json!(self.total_words));
        dict.insert("duration_ms".to_string(), serde_json::json!(self.duration_ms));
        dict
    }
}

/// Result of a site mapping operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteMap {
    /// The starting URL.
    pub start_url: String,
    /// All crawled pages.
    pub pages: Vec<WebPage>,
    /// Internal links discovered.
    pub internal_links: Vec<ExtractedLink>,
    /// External links discovered.
    pub external_links: Vec<ExtractedLink>,
    /// Maximum depth reached.
    pub depth_reached: usize,
    /// Total duration in milliseconds.
    pub duration_ms: f64,
}

impl SiteMap {
    /// Creates a new site map.
    #[must_use]
    pub fn new(start_url: impl Into<String>) -> Self {
        Self {
            start_url: start_url.into(),
            pages: Vec::new(),
            internal_links: Vec::new(),
            external_links: Vec::new(),
            depth_reached: 0,
            duration_ms: 0.0,
        }
    }

    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        dict.insert("start_url".to_string(), serde_json::json!(self.start_url));
        dict.insert("pages_crawled".to_string(), serde_json::json!(self.pages.len()));
        dict.insert("internal_links".to_string(), serde_json::json!(self.internal_links.len()));
        dict.insert("external_links".to_string(), serde_json::json!(self.external_links.len()));
        dict.insert("depth_reached".to_string(), serde_json::json!(self.depth_reached));
        dict.insert("duration_ms".to_string(), serde_json::json!(self.duration_ms));
        dict
    }
}

/// Calculates relevance score for a page against a query.
#[must_use]
pub fn calculate_relevance_score(page: &WebPage, query: &str) -> f64 {
    let query_terms: HashSet<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();

    if query_terms.is_empty() {
        return 0.0;
    }

    let title = page.metadata.title.as_deref().unwrap_or("");
    let content = format!("{} {}", title, page.plain_text).to_lowercase();

    let matches: usize = query_terms
        .iter()
        .filter(|term| content.contains(term.as_str()))
        .count();

    matches as f64 / query_terms.len() as f64
}

/// Filters pages by relevance threshold.
#[must_use]
pub fn filter_relevant_pages(pages: &[WebPage], query: &str, threshold: f64) -> Vec<WebPage> {
    let mut relevant: Vec<(f64, WebPage)> = pages
        .iter()
        .filter_map(|page| {
            let score = calculate_relevance_score(page, query);
            if score >= threshold {
                Some((score, page.clone()))
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending
    relevant.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    relevant.into_iter().map(|(_, page)| page).collect()
}

/// Extracts unique links from multiple pages.
#[must_use]
pub fn extract_unique_links(
    pages: &[WebPage],
    internal_only: bool,
    external_only: bool,
) -> Vec<ExtractedLink> {
    let mut seen_urls: HashSet<String> = HashSet::new();
    let mut links = Vec::new();

    for page in pages {
        for link in &page.links {
            if seen_urls.contains(&link.url) {
                continue;
            }

            if internal_only && !link.is_internal {
                continue;
            }

            if external_only && link.is_internal {
                continue;
            }

            seen_urls.insert(link.url.clone());
            links.push(link.clone());
        }
    }

    links
}

/// Calculates exponential backoff delay.
#[must_use]
pub fn calculate_retry_delay(attempt: usize, base_delay: f64, max_delay: f64) -> f64 {
    let delay = base_delay * (attempt + 1) as f64;
    delay.min(max_delay)
}

/// Creates an error result for a failed fetch.
#[must_use]
pub fn create_error_result(url: &str, error: &str, duration_ms: f64) -> WebPage {
    WebPage {
        url: url.to_string(),
        status_code: 0,
        error: Some(error.to_string()),
        fetch_duration_ms: duration_ms,
        fetched_at: Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.6f+00:00").to_string()),
        ..Default::default()
    }
}

/// Extracts domain from URL.
#[must_use]
pub fn extract_domain(url: &str) -> Option<String> {
    let start = url.find("://").map(|i| i + 3)?;
    let rest = &url[start..];
    let end = rest.find('/').unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// Checks if two URLs are on the same domain.
#[must_use]
pub fn same_domain(url1: &str, url2: &str) -> bool {
    match (extract_domain(url1), extract_domain(url2)) {
        (Some(d1), Some(d2)) => d1 == d2,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_progress() {
        let mut progress = FetchProgress::new(10);
        assert_eq!(progress.percent(), 0.0);

        progress.record_success("https://example.com", 100.0);
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.success_count, 1);
        assert_eq!(progress.percent(), 10.0);

        progress.record_error("https://failed.com", 50.0);
        assert_eq!(progress.completed, 2);
        assert_eq!(progress.error_count, 1);
        assert_eq!(progress.percent(), 20.0);
    }

    #[test]
    fn test_fetch_progress_empty() {
        let progress = FetchProgress::new(0);
        assert_eq!(progress.percent(), 0.0);
    }

    #[test]
    fn test_search_result_to_dict() {
        let result = SearchResult {
            query: "test query".to_string(),
            pages: vec![WebPage::default()],
            relevant_pages: vec![WebPage::default()],
            total_words: 100,
            duration_ms: 500.0,
        };

        let dict = result.to_dict();
        assert_eq!(dict.get("query"), Some(&serde_json::json!("test query")));
        assert_eq!(dict.get("pages_fetched"), Some(&serde_json::json!(1)));
        assert_eq!(dict.get("relevant_pages"), Some(&serde_json::json!(1)));
    }

    #[test]
    fn test_site_map_to_dict() {
        let map = SiteMap::new("https://example.com");
        let dict = map.to_dict();
        assert_eq!(dict.get("start_url"), Some(&serde_json::json!("https://example.com")));
        assert_eq!(dict.get("pages_crawled"), Some(&serde_json::json!(0)));
    }

    #[test]
    fn test_calculate_relevance_score() {
        let page = WebPage {
            plain_text: "This is a test page about rust programming".to_string(),
            metadata: super::super::models::PageMetadata {
                title: Some("Rust Guide".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let score = calculate_relevance_score(&page, "rust programming");
        assert!(score > 0.5);

        let score_none = calculate_relevance_score(&page, "python java");
        assert_eq!(score_none, 0.0);
    }

    #[test]
    fn test_calculate_relevance_empty_query() {
        let page = WebPage::default();
        let score = calculate_relevance_score(&page, "");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_filter_relevant_pages() {
        let pages = vec![
            WebPage {
                url: "https://example.com/1".to_string(),
                plain_text: "rust programming language".to_string(),
                ..Default::default()
            },
            WebPage {
                url: "https://example.com/2".to_string(),
                plain_text: "completely unrelated content".to_string(),
                ..Default::default()
            },
        ];

        let relevant = filter_relevant_pages(&pages, "rust programming", 0.5);
        assert_eq!(relevant.len(), 1);
        assert_eq!(relevant[0].url, "https://example.com/1");
    }

    #[test]
    fn test_extract_unique_links() {
        let pages = vec![
            WebPage {
                links: vec![
                    ExtractedLink { url: "https://example.com/a".to_string(), is_internal: true, ..Default::default() },
                    ExtractedLink { url: "https://other.com/b".to_string(), is_internal: false, ..Default::default() },
                ],
                ..Default::default()
            },
            WebPage {
                links: vec![
                    ExtractedLink { url: "https://example.com/a".to_string(), is_internal: true, ..Default::default() },
                    ExtractedLink { url: "https://example.com/c".to_string(), is_internal: true, ..Default::default() },
                ],
                ..Default::default()
            },
        ];

        let all = extract_unique_links(&pages, false, false);
        assert_eq!(all.len(), 3);

        let internal = extract_unique_links(&pages, true, false);
        assert_eq!(internal.len(), 2);

        let external = extract_unique_links(&pages, false, true);
        assert_eq!(external.len(), 1);
    }

    #[test]
    fn test_calculate_retry_delay() {
        assert_eq!(calculate_retry_delay(0, 1.0, 30.0), 1.0);
        assert_eq!(calculate_retry_delay(1, 1.0, 30.0), 2.0);
        assert_eq!(calculate_retry_delay(2, 1.0, 30.0), 3.0);
        assert_eq!(calculate_retry_delay(100, 1.0, 30.0), 30.0);
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/path"), Some("example.com".to_string()));
        assert_eq!(extract_domain("http://sub.example.com"), Some("sub.example.com".to_string()));
        assert_eq!(extract_domain("invalid"), None);
    }

    #[test]
    fn test_same_domain() {
        assert!(same_domain("https://example.com/a", "https://example.com/b"));
        assert!(!same_domain("https://example.com", "https://other.com"));
        assert!(!same_domain("invalid", "https://example.com"));
    }

    #[test]
    fn test_create_error_result() {
        let result = create_error_result("https://example.com", "timeout", 1000.0);
        assert_eq!(result.url, "https://example.com");
        assert_eq!(result.error, Some("timeout".to_string()));
        assert!(!result.success());
    }
}
