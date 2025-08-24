use super::base::SearchEngineBase;
use crate::api::builtin_mcp::search::types::{SearchItem, SearchResults};

/// Kagi搜索引擎实现
pub struct KagiEngine;

impl KagiEngine {
    pub fn display_name() -> &'static str {
        "Kagi"
    }
    
    pub fn homepage_url() -> &'static str {
        "https://kagi.com"
    }
    
    pub fn search_input_selectors() -> Vec<&'static str> {
        vec![
            "input[name='q']",
            "#searchInput",
            ".search-input",
            "input[placeholder*='搜索']",
            "input[placeholder*='Search']",
        ]
    }
    
    pub fn search_button_selectors() -> Vec<&'static str> {
        vec![
            "button[type='submit']",
            ".search-button",
            "input[type='submit']",
        ]
    }
    
    pub fn default_wait_selectors() -> Vec<String> {
        vec![
            "#search-content".to_string(),
            ".search-result".to_string(),
            "#main".to_string(),
            ".search-container".to_string(),
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
    
    /// 解析Kagi搜索结果HTML，提取结构化信息
    pub fn parse_search_results(html: &str, query: &str) -> SearchResults {
        let mut items = Vec::new();
        
        // Kagi搜索结果通常使用较为标准的HTML结构
        let result_patterns = [
            // 标准搜索结果模式
            r#"<div[^>]*class="[^"]*\bresult\b[^"]*"[^>]*>(.*?)</div>"#,
            // 搜索结果条目
            r#"<article[^>]*class="[^"]*\bsearch-result\b[^"]*"[^>]*>(.*?)</article>"#,
            // 通用结果容器
            r#"<div[^>]*class="[^"]*\bsearch-item\b[^"]*"[^>]*>(.*?)</div>"#,
        ];
        
        let mut rank = 1;
        
        // 尝试不同的匹配模式
        for pattern in &result_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for cap in re.captures_iter(html) {
                    if let Some(result_html) = cap.get(1) {
                        if let Some(item) = Self::parse_single_result(result_html.as_str(), rank) {
                            items.push(item);
                            rank += 1;
                            
                            // 限制结果数量
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
        
        // 如果使用正则表达式失败，尝试备用解析方法
        if items.is_empty() {
            items = Self::fallback_parse_results(html);
        }
        
        SearchResults {
            query: query.to_string(),
            search_engine: Self::display_name().to_string(),
            engine_id: "kagi".to_string(),
            homepage_url: Self::homepage_url().to_string(),
            items,
            total_results: None, // Kagi可能不显示总数
            search_time_ms: None,
        }
    }
    
    /// 解析单个Kagi搜索结果
    fn parse_single_result(html: &str, rank: usize) -> Option<SearchItem> {
        // 提取标题 - Kagi通常使用标准的h2或h3标签结构
        let title = Self::extract_text_by_patterns(html, &[
            r#"<h2[^>]*><a[^>]*>(.*?)</a></h2>"#,
            r#"<h3[^>]*><a[^>]*>(.*?)</a></h3>"#,
            r#"<a[^>]*class="[^"]*\btitle\b[^"]*"[^>]*>(.*?)</a>"#,
            r#"<div[^>]*class="[^"]*\btitle\b[^"]*"[^>]*>(.*?)</div>"#,
        ]).unwrap_or_else(|| format!("Kagi Result {}", rank));
        
        // 提取URL
        let url = Self::extract_url_from_html(html).unwrap_or_default();
        
        // 提取摘要 - Kagi使用简洁的描述结构
        let snippet = Self::extract_text_by_patterns(html, &[
            r#"<p[^>]*class="[^"]*\bsnippet\b[^"]*"[^>]*>(.*?)</p>"#,
            r#"<div[^>]*class="[^"]*\bdescription\b[^"]*"[^>]*>(.*?)</div>"#,
            r#"<span[^>]*class="[^"]*\bsnippet\b[^"]*"[^>]*>(.*?)</span>"#,
            r#"<p[^>]*>(.*?)</p>"#,
        ]).unwrap_or_default();
        
        // 提取显示URL
        let display_url = Self::extract_text_by_patterns(html, &[
            r#"<cite[^>]*>(.*?)</cite>"#,
            r#"<span[^>]*class="[^"]*\burl\b[^"]*"[^>]*>(.*?)</span>"#,
            r#"<div[^>]*class="[^"]*\burl\b[^"]*"[^>]*>(.*?)</div>"#,
        ]);
        
        if !title.is_empty() && !url.is_empty() {
            Some(SearchItem {
                title: Self::clean_html_text(&title),
                url,
                snippet: Self::clean_html_text(&snippet),
                rank,
                display_url: display_url.map(|s| Self::clean_html_text(&s)),
            })
        } else {
            None
        }
    }
    
    /// 备用解析方法
    fn fallback_parse_results(html: &str) -> Vec<SearchItem> {
        let mut items = Vec::new();
        let lines: Vec<&str> = html.lines().collect();
        let mut rank = 1;
        
        for (index, line) in lines.iter().enumerate() {
            // 寻找包含链接和标题的行
            if (line.contains("<h2") || line.contains("<h3")) && line.contains("href=") {
                if let Some(url) = Self::extract_url_from_html(line) {
                    if let Some(title) = Self::extract_text_by_patterns(line, &[
                        r#"<h2[^>]*><a[^>]*>(.*?)</a></h2>"#,
                        r#"<h3[^>]*><a[^>]*>(.*?)</a></h3>"#,
                        r#"<a[^>]*>(.*?)</a>"#,
                    ]) {
                        // 寻找可能的描述
                        let snippet = if let Some(next_line) = lines.get(index + 1) {
                            if next_line.contains("<p") || next_line.contains("snippet") {
                                Self::extract_text_by_patterns(next_line, &[
                                    r#"<p[^>]*>(.*?)</p>"#,
                                    r#"<div[^>]*>(.*?)</div>"#,
                                ]).unwrap_or_default()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        
                        items.push(SearchItem {
                            title: Self::clean_html_text(&title),
                            url,
                            snippet: Self::clean_html_text(&snippet),
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
    
    /// 使用多个模式提取文本
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
    
    /// 从HTML中提取URL
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
    
    /// 清理HTML文本
    fn clean_html_text(text: &str) -> String {
        // 移除HTML标签
        let re = regex::Regex::new(r"<[^>]*>").unwrap_or_else(|_| regex::Regex::new(r"").unwrap());
        let cleaned = re.replace_all(text, "");
        
        // 解码HTML实体
        let cleaned = cleaned
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ");
        
        // 清理多余空白
        let re = regex::Regex::new(r"\s+").unwrap_or_else(|_| regex::Regex::new(r"").unwrap());
        re.replace_all(&cleaned, " ").trim().to_string()
    }
}