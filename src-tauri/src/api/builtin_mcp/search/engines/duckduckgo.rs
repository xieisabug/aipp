use super::base::SearchEngineBase;
use crate::api::builtin_mcp::search::types::{SearchItem, SearchResults};

/// DuckDuckGo搜索引擎实现
pub struct DuckDuckGoEngine;

impl DuckDuckGoEngine {
    pub fn display_name() -> &'static str {
        "DuckDuckGo"
    }
    
    pub fn homepage_url() -> &'static str {
        "https://duckduckgo.com"
    }
    
    pub fn search_input_selectors() -> Vec<&'static str> {
        vec![
            "#search_form_input",
            "input[name='q']",
            "#searchbox_input", 
            ".js-search-input",
            "input[placeholder*='搜索']",
            "input[placeholder*='Search']",
        ]
    }
    
    pub fn search_button_selectors() -> Vec<&'static str> {
        vec![
            "input[type='submit']",
            "#search_button_homepage",
            ".search-wrap__button",
        ]
    }
    
    pub fn default_wait_selectors() -> Vec<String> {
        vec![
            "#links".to_string(),
            ".results".to_string(),
            ".result".to_string(),
            "#web_content".to_string(),
        ]
    }
    
    pub async fn perform_search(page: &playwright::api::Page, query: &str) -> Result<String, String> {
        SearchEngineBase::perform_search(
            page,
            query,
            Self::display_name(),
            Self::homepage_url(),
            &Self::search_input_selectors(),
            &Self::search_button_selectors(),
            &Self::default_wait_selectors(),
        ).await
    }
    
    /// 解析DuckDuckGo搜索结果HTML，提取结构化信息
    pub fn parse_search_results(html: &str, query: &str) -> SearchResults {
        let mut items = Vec::new();
        
        // DuckDuckGo搜索结果通常使用 .result 类
        let result_patterns = [
            r#"<div[^>]*class="[^"]*\bresult\b[^"]*"[^>]*>(.*?)</div>"#,
            r#"<article[^>]*class="[^"]*\bresult\b[^"]*"[^>]*>(.*?)</article>"#,
        ];
        
        let mut rank = 1;
        
        for pattern in &result_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for cap in re.captures_iter(html) {
                    if let Some(result_html) = cap.get(1) {
                        if let Some(item) = Self::parse_single_result(result_html.as_str(), rank) {
                            items.push(item);
                            rank += 1;
                            
                            if items.len() >= 20 {
                                break;
                            }
                        }
                    }
                }
                
                if !items.is_empty() {
                    break;
                }
            }
        }
        
        if items.is_empty() {
            items = Self::fallback_parse_results(html);
        }
        
        SearchResults {
            query: query.to_string(),
            search_engine: Self::display_name().to_string(),
            engine_id: "duckduckgo".to_string(),
            homepage_url: Self::homepage_url().to_string(),
            items,
            total_results: None, // DuckDuckGo通常不显示结果总数
            search_time_ms: None,
        }
    }
    
    fn parse_single_result(html: &str, rank: usize) -> Option<SearchItem> {
        let title = Self::extract_text_by_patterns(html, &[
            r#"<h2[^>]*><a[^>]*>(.*?)</a></h2>"#,
            r#"<a[^>]*class="[^"]*\bresult__title\b[^"]*"[^>]*>(.*?)</a>"#,
            r#"<h3[^>]*><a[^>]*>(.*?)</a></h3>"#,
        ]).unwrap_or_else(|| format!("DuckDuckGo Result {}", rank));
        
        let url = Self::extract_url_from_html(html).unwrap_or_default();
        
        let snippet = Self::extract_text_by_patterns(html, &[
            r#"<span[^>]*class="[^"]*\bresult__snippet\b[^"]*"[^>]*>(.*?)</span>"#,
            r#"<div[^>]*class="[^"]*\bresult__snippet\b[^"]*"[^>]*>(.*?)</div>"#,
        ]).unwrap_or_default();
        
        if !title.is_empty() && !url.is_empty() {
            Some(SearchItem {
                title: Self::clean_html_text(&title),
                url,
                snippet: Self::clean_html_text(&snippet),
                rank,
                display_url: None,
            })
        } else {
            None
        }
    }
    
    fn fallback_parse_results(html: &str) -> Vec<SearchItem> {
        let mut items = Vec::new();
        let lines: Vec<&str> = html.lines().collect();
        let mut rank = 1;
        
        for line in lines {
            if line.contains("result__title") || (line.contains("<h") && line.contains("href=")) {
                if let Some(url) = Self::extract_url_from_html(line) {
                    if let Some(title) = Self::extract_text_by_patterns(line, &[
                        r#"<a[^>]*>(.*?)</a>"#,
                        r#"<h[^>]*>(.*?)</h[^>]*>"#,
                    ]) {
                        items.push(SearchItem {
                            title: Self::clean_html_text(&title),
                            url,
                            snippet: String::new(),
                            rank,
                            display_url: None,
                        });
                        
                        rank += 1;
                        if items.len() >= 10 {
                            break;
                        }
                    }
                }
            }
        }
        
        items
    }
    
    fn extract_text_by_patterns(html: &str, patterns: &[&str]) -> Option<String> {
        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(cap) = re.captures(html) {
                    if let Some(matched) = cap.get(1) {
                        let text = matched.as_str().trim();
                        if !text.is_empty() {
                            return Some(text.to_string());
                        }
                    }
                }
            }
        }
        None
    }
    
    fn extract_url_from_html(html: &str) -> Option<String> {
        let url_patterns = [
            r#"href="(https?://[^"]*)"#,
            r#"href="([^"]*)"#,
        ];
        
        for pattern in &url_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(cap) = re.captures(html) {
                    if let Some(url_match) = cap.get(1) {
                        let url = url_match.as_str();
                        if url.starts_with("http") {
                            return Some(url.to_string());
                        }
                    }
                }
            }
        }
        None
    }
    
    fn clean_html_text(text: &str) -> String {
        let re = regex::Regex::new(r"<[^>]*>").unwrap_or_else(|_| regex::Regex::new(r"").unwrap());
        let cleaned = re.replace_all(text, "");
        
        let cleaned = cleaned
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ");
        
        let re = regex::Regex::new(r"\s+").unwrap_or_else(|_| regex::Regex::new(r"").unwrap());
        re.replace_all(&cleaned, " ").trim().to_string()
    }
}