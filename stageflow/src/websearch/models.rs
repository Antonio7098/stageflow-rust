//! Data models for web search and content extraction.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata extracted from a web page.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PageMetadata {
    /// Page title.
    pub title: Option<String>,
    /// Meta description.
    pub description: Option<String>,
    /// Page language.
    pub language: Option<String>,
    /// Author name.
    pub author: Option<String>,
    /// Published date.
    pub published_date: Option<String>,
    /// Canonical URL.
    pub canonical_url: Option<String>,
    /// Open Graph image URL.
    pub og_image: Option<String>,
    /// Content type (e.g., article, website).
    pub content_type: Option<String>,
    /// Keywords.
    #[serde(default)]
    pub keywords: Vec<String>,
}

impl PageMetadata {
    /// Creates new empty metadata.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        if let Some(ref v) = self.title {
            dict.insert("title".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.description {
            dict.insert("description".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.language {
            dict.insert("language".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.author {
            dict.insert("author".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.published_date {
            dict.insert("published_date".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.canonical_url {
            dict.insert("canonical_url".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.og_image {
            dict.insert("og_image".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.content_type {
            dict.insert("content_type".to_string(), serde_json::json!(v));
        }
        dict.insert("keywords".to_string(), serde_json::json!(self.keywords));
        dict
    }

    /// Creates from dictionary.
    pub fn from_dict(dict: &HashMap<String, serde_json::Value>) -> Self {
        Self {
            title: dict.get("title").and_then(|v| v.as_str()).map(String::from),
            description: dict.get("description").and_then(|v| v.as_str()).map(String::from),
            language: dict.get("language").and_then(|v| v.as_str()).map(String::from),
            author: dict.get("author").and_then(|v| v.as_str()).map(String::from),
            published_date: dict.get("published_date").and_then(|v| v.as_str()).map(String::from),
            canonical_url: dict.get("canonical_url").and_then(|v| v.as_str()).map(String::from),
            og_image: dict.get("og_image").and_then(|v| v.as_str()).map(String::from),
            content_type: dict.get("content_type").and_then(|v| v.as_str()).map(String::from),
            keywords: dict
                .get("keywords")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        }
    }
}

/// A link extracted from a web page.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ExtractedLink {
    /// The URL of the link.
    pub url: String,
    /// The link text.
    #[serde(default)]
    pub text: String,
    /// The link title attribute.
    pub title: Option<String>,
    /// The rel attribute.
    pub rel: Option<String>,
    /// Whether this is an internal link.
    #[serde(default)]
    pub is_internal: bool,
    /// Context around the link.
    pub context: Option<String>,
}

impl ExtractedLink {
    /// Creates a new extracted link.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Creates from an element with resolution.
    #[must_use]
    pub fn from_element(
        href: &str,
        text: &str,
        base_url: Option<&str>,
        title: Option<&str>,
        rel: Option<&str>,
        context: Option<&str>,
    ) -> Self {
        let mut url = href.to_string();
        let mut is_internal = false;

        // Resolve relative URLs
        if let Some(base) = base_url {
            if !href.starts_with("http://") && !href.starts_with("https://") && !href.starts_with("//") {
                // Simple URL join
                if href.starts_with('/') {
                    if let Some(domain_end) = base.find("://").map(|i| base[i + 3..].find('/').map(|j| i + 3 + j).unwrap_or(base.len())) {
                        url = format!("{}{}", &base[..domain_end], href);
                    }
                } else {
                    let base_path = base.rfind('/').map(|i| &base[..=i]).unwrap_or(base);
                    url = format!("{}{}", base_path, href);
                }
            } else if href.starts_with("//") {
                url = format!("https:{}", href);
            }

            // Check if internal
            if let (Some(base_domain), Some(url_domain)) = (extract_domain(base), extract_domain(&url)) {
                is_internal = base_domain == url_domain;
            }
        }

        Self {
            url,
            text: text.trim().to_string(),
            title: title.map(String::from),
            rel: rel.map(String::from),
            is_internal,
            context: context.map(String::from),
        }
    }

    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        dict.insert("url".to_string(), serde_json::json!(self.url));
        dict.insert("text".to_string(), serde_json::json!(self.text));
        if let Some(ref v) = self.title {
            dict.insert("title".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.rel {
            dict.insert("rel".to_string(), serde_json::json!(v));
        }
        dict.insert("is_internal".to_string(), serde_json::json!(self.is_internal));
        if let Some(ref v) = self.context {
            dict.insert("context".to_string(), serde_json::json!(v));
        }
        dict
    }
}

/// Extract domain from URL.
fn extract_domain(url: &str) -> Option<String> {
    let start = url.find("://").map(|i| i + 3)?;
    let rest = &url[start..];
    let end = rest.find('/').unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// A navigation action that can be taken on a page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NavigationAction {
    /// Type of action (e.g., "pagination", "nav_link", "content_link").
    pub action_type: String,
    /// Human-readable label.
    pub label: String,
    /// Target URL if applicable.
    pub url: Option<String>,
    /// CSS selector if applicable.
    pub selector: Option<String>,
    /// Priority (lower = higher priority).
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_priority() -> u8 {
    5
}

impl NavigationAction {
    /// Creates a new navigation action.
    #[must_use]
    pub fn new(action_type: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_type: action_type.into(),
            label: label.into(),
            url: None,
            selector: None,
            priority: 5,
            metadata: HashMap::new(),
        }
    }

    /// Sets the URL.
    #[must_use]
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the priority.
    #[must_use]
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        dict.insert("action_type".to_string(), serde_json::json!(self.action_type));
        dict.insert("label".to_string(), serde_json::json!(self.label));
        if let Some(ref v) = self.url {
            dict.insert("url".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.selector {
            dict.insert("selector".to_string(), serde_json::json!(v));
        }
        dict.insert("priority".to_string(), serde_json::json!(self.priority));
        dict.insert("metadata".to_string(), serde_json::json!(self.metadata));
        dict
    }
}

/// Pagination information for a page.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PaginationInfo {
    /// Current page number.
    #[serde(default = "default_page")]
    pub current_page: u32,
    /// Total number of pages if known.
    pub total_pages: Option<u32>,
    /// URL of the next page.
    pub next_url: Option<String>,
    /// URL of the previous page.
    pub prev_url: Option<String>,
    /// URLs of all known pages.
    #[serde(default)]
    pub page_urls: Vec<String>,
}

fn default_page() -> u32 {
    1
}

impl PaginationInfo {
    /// Creates new pagination info.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether there is a next page.
    #[must_use]
    pub fn has_next(&self) -> bool {
        self.next_url.is_some()
    }

    /// Whether there is a previous page.
    #[must_use]
    pub fn has_prev(&self) -> bool {
        self.prev_url.is_some()
    }

    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        dict.insert("current_page".to_string(), serde_json::json!(self.current_page));
        if let Some(v) = self.total_pages {
            dict.insert("total_pages".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.next_url {
            dict.insert("next_url".to_string(), serde_json::json!(v));
        }
        if let Some(ref v) = self.prev_url {
            dict.insert("prev_url".to_string(), serde_json::json!(v));
        }
        dict.insert("page_urls".to_string(), serde_json::json!(self.page_urls));
        dict
    }
}

/// A fetched and processed web page.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebPage {
    /// The original URL requested.
    pub url: String,
    /// The final URL after redirects.
    pub final_url: Option<String>,
    /// HTTP status code.
    #[serde(default)]
    pub status_code: u16,
    /// Extracted markdown content.
    #[serde(default)]
    pub markdown: String,
    /// Plain text content.
    #[serde(default)]
    pub plain_text: String,
    /// Page metadata.
    #[serde(default)]
    pub metadata: PageMetadata,
    /// Extracted links.
    #[serde(default)]
    pub links: Vec<ExtractedLink>,
    /// Navigation actions.
    #[serde(default)]
    pub navigation_actions: Vec<NavigationAction>,
    /// Pagination info.
    pub pagination: Option<PaginationInfo>,
    /// Time to fetch in milliseconds.
    #[serde(default)]
    pub fetch_duration_ms: f64,
    /// Time to extract content in milliseconds.
    #[serde(default)]
    pub extract_duration_ms: f64,
    /// When the page was fetched.
    pub fetched_at: Option<String>,
    /// Word count.
    #[serde(default)]
    pub word_count: usize,
    /// Error message if fetch failed.
    pub error: Option<String>,
}

impl WebPage {
    /// Creates a new web page.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Whether the fetch was successful.
    #[must_use]
    pub fn success(&self) -> bool {
        self.error.is_none() && (200..400).contains(&self.status_code)
    }

    /// Gets the page title.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.metadata.title.as_deref()
    }

    /// Gets the page description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.metadata.description.as_deref()
    }

    /// Gets internal links.
    #[must_use]
    pub fn internal_links(&self) -> Vec<&ExtractedLink> {
        self.links.iter().filter(|l| l.is_internal).collect()
    }

    /// Gets external links.
    #[must_use]
    pub fn external_links(&self) -> Vec<&ExtractedLink> {
        self.links.iter().filter(|l| !l.is_internal).collect()
    }

    /// Extracts links with optional filters.
    #[must_use]
    pub fn extract_links(
        &self,
        internal_only: bool,
        external_only: bool,
        limit: Option<usize>,
    ) -> Vec<&ExtractedLink> {
        let mut links: Vec<_> = self.links.iter()
            .filter(|l| {
                if internal_only && !l.is_internal {
                    return false;
                }
                if external_only && l.is_internal {
                    return false;
                }
                true
            })
            .collect();

        if let Some(limit) = limit {
            links.truncate(limit);
        }

        links
    }

    /// Creates an error result.
    #[must_use]
    pub fn error_result(url: impl Into<String>, error: impl Into<String>, duration_ms: f64) -> Self {
        Self {
            url: url.into(),
            status_code: 0,
            error: Some(error.into()),
            fetch_duration_ms: duration_ms,
            fetched_at: Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.6f+00:00").to_string()),
            ..Default::default()
        }
    }

    /// Truncates content to a maximum length.
    #[must_use]
    pub fn truncate(&self, max_chars: usize) -> Self {
        if self.markdown.len() <= max_chars {
            return self.clone();
        }

        let mut truncated = self.clone();

        // Truncate markdown
        let mut md = self.markdown.chars().take(max_chars).collect::<String>();
        // Try to find a good break point
        if let Some(pos) = md.rfind("\n\n") {
            if pos > max_chars / 2 {
                md.truncate(pos);
            }
        }
        md.push_str("\n\n[Content truncated...]");
        truncated.markdown = md;

        // Truncate plain text
        let mut pt = self.plain_text.chars().take(max_chars).collect::<String>();
        // Try to find a good break point
        for sep in [". ", "! ", "? "] {
            if let Some(pos) = pt.rfind(sep) {
                if pos > max_chars / 2 {
                    pt.truncate(pos + sep.len());
                    break;
                }
            }
        }
        truncated.plain_text = pt;

        truncated
    }

    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        dict.insert("url".to_string(), serde_json::json!(self.url));
        if let Some(ref v) = self.final_url {
            dict.insert("final_url".to_string(), serde_json::json!(v));
        }
        dict.insert("status_code".to_string(), serde_json::json!(self.status_code));
        dict.insert("markdown".to_string(), serde_json::json!(self.markdown));
        dict.insert("plain_text".to_string(), serde_json::json!(self.plain_text));
        dict.insert("metadata".to_string(), serde_json::json!(self.metadata.to_dict()));
        dict.insert("links".to_string(), serde_json::json!(
            self.links.iter().map(|l| l.to_dict()).collect::<Vec<_>>()
        ));
        dict.insert("navigation_actions".to_string(), serde_json::json!(
            self.navigation_actions.iter().map(|a| a.to_dict()).collect::<Vec<_>>()
        ));
        if let Some(ref p) = self.pagination {
            dict.insert("pagination".to_string(), serde_json::json!(p.to_dict()));
        }
        dict.insert("fetch_duration_ms".to_string(), serde_json::json!(self.fetch_duration_ms));
        dict.insert("extract_duration_ms".to_string(), serde_json::json!(self.extract_duration_ms));
        if let Some(ref v) = self.fetched_at {
            dict.insert("fetched_at".to_string(), serde_json::json!(v));
        }
        dict.insert("word_count".to_string(), serde_json::json!(self.word_count));
        if let Some(ref v) = self.error {
            dict.insert("error".to_string(), serde_json::json!(v));
        }
        dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_metadata_roundtrip() {
        let meta = PageMetadata::new()
            .with_title("Test Page")
            .with_description("A test description");

        let dict = meta.to_dict();
        let restored = PageMetadata::from_dict(&dict);

        assert_eq!(meta, restored);
    }

    #[test]
    fn test_extracted_link_from_element() {
        let link = ExtractedLink::from_element(
            "/about",
            "  About Us  ",
            Some("https://example.com/page"),
            None,
            None,
            None,
        );

        assert_eq!(link.url, "https://example.com/about");
        assert_eq!(link.text, "About Us");
        assert!(link.is_internal);
    }

    #[test]
    fn test_extracted_link_external() {
        let link = ExtractedLink::from_element(
            "https://other.com/page",
            "External",
            Some("https://example.com"),
            None,
            None,
            None,
        );

        assert_eq!(link.url, "https://other.com/page");
        assert!(!link.is_internal);
    }

    #[test]
    fn test_pagination_info() {
        let mut pagination = PaginationInfo::new();
        assert!(!pagination.has_next());
        assert!(!pagination.has_prev());

        pagination.next_url = Some("https://example.com/page/2".to_string());
        assert!(pagination.has_next());
    }

    #[test]
    fn test_web_page_success() {
        let page = WebPage {
            url: "https://example.com".to_string(),
            status_code: 200,
            ..Default::default()
        };
        assert!(page.success());

        let error_page = WebPage::error_result("https://example.com", "Failed", 100.0);
        assert!(!error_page.success());
    }

    #[test]
    fn test_web_page_extract_links() {
        let page = WebPage {
            url: "https://example.com".to_string(),
            links: vec![
                ExtractedLink { url: "/internal".to_string(), is_internal: true, ..Default::default() },
                ExtractedLink { url: "https://other.com".to_string(), is_internal: false, ..Default::default() },
                ExtractedLink { url: "/another".to_string(), is_internal: true, ..Default::default() },
            ],
            ..Default::default()
        };

        assert_eq!(page.internal_links().len(), 2);
        assert_eq!(page.external_links().len(), 1);
        assert_eq!(page.extract_links(true, false, Some(1)).len(), 1);
    }

    #[test]
    fn test_web_page_truncate() {
        let page = WebPage {
            url: "https://example.com".to_string(),
            markdown: "A".repeat(20000),
            plain_text: "B".repeat(20000),
            ..Default::default()
        };

        let truncated = page.truncate(10000);
        assert!(truncated.markdown.len() < 15000);
        assert!(truncated.markdown.contains("[Content truncated...]"));
    }

    #[test]
    fn test_navigation_action() {
        let action = NavigationAction::new("pagination", "Next page")
            .with_url("https://example.com/page/2")
            .with_priority(1);

        assert_eq!(action.action_type, "pagination");
        assert_eq!(action.priority, 1);

        let dict = action.to_dict();
        assert_eq!(dict.get("action_type"), Some(&serde_json::json!("pagination")));
    }
}
