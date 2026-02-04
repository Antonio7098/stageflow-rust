//! Configuration types for web search and fetching.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

/// Configuration for HTTP fetching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchConfig {
    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: f64,
    /// Maximum number of redirects to follow.
    #[serde(default = "default_max_redirects")]
    pub max_redirects: usize,
    /// User agent string.
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    /// Whether to verify SSL certificates.
    #[serde(default = "default_verify_ssl")]
    pub verify_ssl: bool,
    /// Maximum response size in bytes.
    #[serde(default = "default_max_size")]
    pub max_response_size: usize,
    /// Additional headers to include.
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    /// Retry configuration.
    #[serde(default)]
    pub retry: RetryConfig,
}

fn default_timeout() -> f64 {
    30.0
}

fn default_max_redirects() -> usize {
    10
}

fn default_user_agent() -> String {
    "stageflow-websearch/0.1".to_string()
}

fn default_verify_ssl() -> bool {
    true
}

fn default_max_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: default_timeout(),
            max_redirects: default_max_redirects(),
            user_agent: default_user_agent(),
            verify_ssl: default_verify_ssl(),
            max_response_size: default_max_size(),
            headers: std::collections::HashMap::new(),
            retry: RetryConfig::default(),
        }
    }
}

impl FetchConfig {
    /// Creates a new fetch configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the timeout.
    #[must_use]
    pub fn with_timeout(mut self, seconds: f64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Sets the user agent.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Adds a header.
    #[must_use]
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Gets timeout as Duration.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        Duration::from_secs_f64(self.timeout_seconds)
    }
}

/// Retry configuration for failed requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    /// Initial delay between retries in seconds.
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: f64,
    /// Backoff multiplier.
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
    /// Maximum delay between retries.
    #[serde(default = "default_max_delay")]
    pub max_delay_seconds: f64,
    /// Status codes that should trigger a retry.
    #[serde(default = "default_retry_status_codes")]
    pub retry_status_codes: HashSet<u16>,
}

fn default_max_retries() -> usize {
    3
}

fn default_retry_delay() -> f64 {
    1.0
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_max_delay() -> f64 {
    30.0
}

fn default_retry_status_codes() -> HashSet<u16> {
    [429, 500, 502, 503, 504].into_iter().collect()
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            retry_delay_seconds: default_retry_delay(),
            backoff_multiplier: default_backoff_multiplier(),
            max_delay_seconds: default_max_delay(),
            retry_status_codes: default_retry_status_codes(),
        }
    }
}

impl RetryConfig {
    /// Calculates the delay for a given attempt.
    #[must_use]
    pub fn delay_for_attempt(&self, attempt: usize) -> Duration {
        let delay = self.retry_delay_seconds * self.backoff_multiplier.powi(attempt as i32);
        let capped = delay.min(self.max_delay_seconds);
        Duration::from_secs_f64(capped)
    }

    /// Whether a status code should trigger a retry.
    #[must_use]
    pub fn should_retry_status(&self, status: u16) -> bool {
        self.retry_status_codes.contains(&status)
    }
}

/// Configuration for content extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Whether to preserve headings in markdown.
    #[serde(default = "default_true")]
    pub preserve_headings: bool,
    /// Whether to preserve lists in markdown.
    #[serde(default = "default_true")]
    pub preserve_lists: bool,
    /// Whether to preserve links in markdown.
    #[serde(default = "default_true")]
    pub preserve_links: bool,
    /// Whether to preserve emphasis in markdown.
    #[serde(default = "default_true")]
    pub preserve_emphasis: bool,
    /// Whether to preserve code blocks in markdown.
    #[serde(default = "default_true")]
    pub preserve_code: bool,
    /// Whether to preserve blockquotes in markdown.
    #[serde(default = "default_true")]
    pub preserve_blockquotes: bool,
    /// Whether to preserve tables in markdown.
    #[serde(default = "default_true")]
    pub preserve_tables: bool,
    /// Maximum length of link text.
    #[serde(default = "default_max_link_text")]
    pub max_link_text_length: usize,
    /// Maximum length of headings.
    #[serde(default = "default_max_heading")]
    pub max_heading_length: usize,
    /// Whether to include link URLs in markdown.
    #[serde(default = "default_true")]
    pub include_link_urls: bool,
    /// Minimum text length to consider.
    #[serde(default = "default_min_text")]
    pub min_text_length: usize,
    /// CSS selectors for elements to remove.
    #[serde(default = "default_remove_selectors")]
    pub remove_selectors: Vec<String>,
    /// CSS selectors for main content.
    #[serde(default = "default_content_selectors")]
    pub main_content_selectors: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_max_link_text() -> usize {
    100
}

fn default_max_heading() -> usize {
    200
}

fn default_min_text() -> usize {
    1
}

fn default_remove_selectors() -> Vec<String> {
    vec![
        "script".to_string(),
        "style".to_string(),
        "noscript".to_string(),
        "iframe".to_string(),
        "svg".to_string(),
        "nav".to_string(),
        "footer".to_string(),
        "header".to_string(),
        "aside".to_string(),
        ".ad".to_string(),
        ".ads".to_string(),
        ".advertisement".to_string(),
        ".sidebar".to_string(),
        ".cookie-banner".to_string(),
        ".cookie-notice".to_string(),
        "#cookie-banner".to_string(),
    ]
}

fn default_content_selectors() -> Vec<String> {
    vec![
        "article".to_string(),
        "main".to_string(),
        "[role=\"main\"]".to_string(),
        "#content".to_string(),
        ".content".to_string(),
        ".post-content".to_string(),
        ".article-content".to_string(),
        ".entry-content".to_string(),
    ]
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            preserve_headings: true,
            preserve_lists: true,
            preserve_links: true,
            preserve_emphasis: true,
            preserve_code: true,
            preserve_blockquotes: true,
            preserve_tables: true,
            max_link_text_length: default_max_link_text(),
            max_heading_length: default_max_heading(),
            include_link_urls: true,
            min_text_length: default_min_text(),
            remove_selectors: default_remove_selectors(),
            main_content_selectors: default_content_selectors(),
        }
    }
}

impl ExtractionConfig {
    /// Creates a new extraction configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a selector to remove.
    #[must_use]
    pub fn with_remove_selector(mut self, selector: impl Into<String>) -> Self {
        self.remove_selectors.push(selector.into());
        self
    }

    /// Adds a main content selector.
    #[must_use]
    pub fn with_content_selector(mut self, selector: impl Into<String>) -> Self {
        self.main_content_selectors.push(selector.into());
        self
    }
}

/// Configuration for page navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationConfig {
    /// CSS selectors for pagination containers.
    #[serde(default = "default_pagination_selectors")]
    pub pagination_selectors: Vec<String>,
    /// Regex patterns for pagination URLs.
    #[serde(default = "default_pagination_patterns")]
    pub pagination_link_patterns: Vec<String>,
    /// Text patterns for next page links.
    #[serde(default = "default_next_texts")]
    pub next_link_texts: Vec<String>,
    /// Text patterns for previous page links.
    #[serde(default = "default_prev_texts")]
    pub prev_link_texts: Vec<String>,
    /// CSS selectors for navigation links.
    #[serde(default = "default_nav_selectors")]
    pub nav_link_selectors: Vec<String>,
    /// CSS selectors for main content areas.
    #[serde(default = "default_content_selectors")]
    pub content_selectors: Vec<String>,
    /// Minimum number of links to consider as navigation.
    #[serde(default = "default_min_nav")]
    pub min_nav_links: usize,
    /// Maximum number of navigation actions to return.
    #[serde(default = "default_max_actions")]
    pub max_actions: usize,
}

fn default_pagination_selectors() -> Vec<String> {
    vec![
        ".pagination".to_string(),
        ".pager".to_string(),
        ".page-nav".to_string(),
        "[role=\"navigation\"]".to_string(),
        "nav.pagination".to_string(),
    ]
}

fn default_pagination_patterns() -> Vec<String> {
    vec![
        r"page=\d+".to_string(),
        r"p=\d+".to_string(),
        r"/page/\d+".to_string(),
        r"offset=\d+".to_string(),
    ]
}

fn default_next_texts() -> Vec<String> {
    vec![
        "next".to_string(),
        "→".to_string(),
        "»".to_string(),
        ">".to_string(),
        "older".to_string(),
        "more".to_string(),
    ]
}

fn default_prev_texts() -> Vec<String> {
    vec![
        "prev".to_string(),
        "previous".to_string(),
        "←".to_string(),
        "«".to_string(),
        "<".to_string(),
        "newer".to_string(),
        "back".to_string(),
    ]
}

fn default_nav_selectors() -> Vec<String> {
    vec![
        "nav a".to_string(),
        ".menu a".to_string(),
        ".nav a".to_string(),
        "[role=\"navigation\"] a".to_string(),
    ]
}

fn default_min_nav() -> usize {
    3
}

fn default_max_actions() -> usize {
    20
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            pagination_selectors: default_pagination_selectors(),
            pagination_link_patterns: default_pagination_patterns(),
            next_link_texts: default_next_texts(),
            prev_link_texts: default_prev_texts(),
            nav_link_selectors: default_nav_selectors(),
            content_selectors: default_content_selectors(),
            min_nav_links: default_min_nav(),
            max_actions: default_max_actions(),
        }
    }
}

impl NavigationConfig {
    /// Creates a new navigation configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Combined configuration for the web search client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    /// Maximum concurrent requests.
    #[serde(default = "default_concurrent")]
    pub max_concurrent: usize,
    /// Whether to automatically extract content.
    #[serde(default = "default_true")]
    pub auto_extract: bool,
    /// Whether to automatically detect navigation.
    #[serde(default = "default_true")]
    pub auto_navigate: bool,
    /// Fetch configuration.
    #[serde(default)]
    pub fetch: FetchConfig,
    /// Extraction configuration.
    #[serde(default)]
    pub extraction: ExtractionConfig,
    /// Navigation configuration.
    #[serde(default)]
    pub navigation: NavigationConfig,
}

fn default_concurrent() -> usize {
    5
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            max_concurrent: default_concurrent(),
            auto_extract: true,
            auto_navigate: true,
            fetch: FetchConfig::default(),
            extraction: ExtractionConfig::default(),
            navigation: NavigationConfig::default(),
        }
    }
}

impl WebSearchConfig {
    /// Creates a new web search configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum concurrent requests.
    #[must_use]
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Disables auto extraction.
    #[must_use]
    pub fn without_auto_extract(mut self) -> Self {
        self.auto_extract = false;
        self
    }

    /// Disables auto navigation.
    #[must_use]
    pub fn without_auto_navigate(mut self) -> Self {
        self.auto_navigate = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_config_defaults() {
        let config = FetchConfig::default();
        assert_eq!(config.timeout_seconds, 30.0);
        assert_eq!(config.max_redirects, 10);
        assert!(config.verify_ssl);
    }

    #[test]
    fn test_fetch_config_builder() {
        let config = FetchConfig::new()
            .with_timeout(60.0)
            .with_user_agent("custom-agent")
            .with_header("Authorization", "Bearer token");

        assert_eq!(config.timeout_seconds, 60.0);
        assert_eq!(config.user_agent, "custom-agent");
        assert_eq!(config.headers.get("Authorization"), Some(&"Bearer token".to_string()));
    }

    #[test]
    fn test_retry_config_delay() {
        let config = RetryConfig::default();
        
        let delay0 = config.delay_for_attempt(0);
        let delay1 = config.delay_for_attempt(1);
        let delay2 = config.delay_for_attempt(2);

        assert_eq!(delay0, Duration::from_secs(1));
        assert_eq!(delay1, Duration::from_secs(2));
        assert_eq!(delay2, Duration::from_secs(4));
    }

    #[test]
    fn test_retry_config_max_delay() {
        let config = RetryConfig {
            max_delay_seconds: 5.0,
            ..Default::default()
        };

        let delay = config.delay_for_attempt(10);
        assert_eq!(delay, Duration::from_secs(5));
    }

    #[test]
    fn test_retry_status_codes() {
        let config = RetryConfig::default();
        
        assert!(config.should_retry_status(429));
        assert!(config.should_retry_status(503));
        assert!(!config.should_retry_status(200));
        assert!(!config.should_retry_status(404));
    }

    #[test]
    fn test_extraction_config_defaults() {
        let config = ExtractionConfig::default();
        
        assert!(config.preserve_headings);
        assert!(config.preserve_links);
        assert!(!config.remove_selectors.is_empty());
        assert!(config.remove_selectors.contains(&"script".to_string()));
    }

    #[test]
    fn test_navigation_config_defaults() {
        let config = NavigationConfig::default();
        
        assert!(config.next_link_texts.contains(&"next".to_string()));
        assert!(config.prev_link_texts.contains(&"prev".to_string()));
        assert_eq!(config.max_actions, 20);
    }

    #[test]
    fn test_web_search_config() {
        let config = WebSearchConfig::new()
            .with_max_concurrent(10)
            .without_auto_extract();

        assert_eq!(config.max_concurrent, 10);
        assert!(!config.auto_extract);
        assert!(config.auto_navigate);
    }
}
