use super::base::SearchEngineBase;
use crate::api::builtin_mcp::search::types::{SearchItem, SearchResults};

/// Bing搜索引擎实现
pub struct BingEngine;

impl BingEngine {
    pub fn display_name() -> &'static str {
        "Bing"
    }
    
    pub fn homepage_url() -> &'static str {
        "https://www.bing.com"
    }
    
    pub fn search_input_selectors() -> Vec<&'static str> {
        vec![
            "#sb_form_q",
            "input[name='q']",
            "textarea[name='q']",
            ".b_searchbox",
            "#searchboxinput",
            "input[placeholder*='搜索']",
            "input[placeholder*='Search']",
        ]
    }
    
    pub fn search_button_selectors() -> Vec<&'static str> {
        vec![
            "#search_icon",
            "input[type='submit']",
            ".b_searchboxSubmit",
            "#sb_form_go",
        ]
    }
    
    pub fn default_wait_selectors() -> Vec<String> {
        vec![
            "#b_content".to_string(),
            "#b_content > main".to_string(),
            ".b_algo".to_string(),
            "#b_results".to_string(),
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
    
    /// 解析Bing搜索结果HTML，提取结构化信息
    pub fn parse_search_results(html: &str, query: &str) -> SearchResults {
        let mut items = Vec::new();
        
        // Bing搜索结果通常使用 .b_algo 类
        let result_patterns = [
            // 标准Bing搜索结果模式
            r#"<li[^>]*class="[^"]*\bb_algo\b[^"]*"[^>]*>(.*?)</li>"#,
            // 备用模式
            r#"<div[^>]*class="[^"]*\bb_algo\b[^"]*"[^>]*>(.*?)</div>"#,
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
        
        // 提取总结果数量
        let total_results = Self::extract_total_results(html);
        
        SearchResults {
            query: query.to_string(),
            search_engine: Self::display_name().to_string(),
            engine_id: "bing".to_string(),
            homepage_url: Self::homepage_url().to_string(),
            items,
            total_results,
            search_time_ms: None,
        }
    }
    
    /// 解析单个Bing搜索结果
    fn parse_single_result(html: &str, rank: usize) -> Option<SearchItem> {
        // 提取标题 - Bing通常使用带有特定类的h2标签
        let title = Self::extract_text_by_patterns(html, &[
            r#"<h2[^>]*><a[^>]*>(.*?)</a></h2>"#,
            r#"<a[^>]*class="[^"]*\bb_title\b[^"]*"[^>]*>(.*?)</a>"#,
            r#"<h2[^>]*>(.*?)</h2>"#,
        ]).unwrap_or_else(|| format!("Bing Result {}", rank));
        
        // 提取URL
        let url = Self::extract_url_from_html(html).unwrap_or_default();
        
        // 提取摘要 - Bing通常使用 .b_caption 或 .b_snippetBigText 类
        let snippet = Self::extract_text_by_patterns(html, &[
            r#"<p[^>]*class="[^"]*\bb_caption\b[^"]*"[^>]*>(.*?)</p>"#,
            r#"<div[^>]*class="[^"]*\bb_caption\b[^"]*"[^>]*>(.*?)</div>"#,
            r#"<span[^>]*class="[^"]*\bb_snippetBigText\b[^"]*"[^>]*>(.*?)</span>"#,
        ]).unwrap_or_default();
        
        // 提取显示URL
        let display_url = Self::extract_text_by_patterns(html, &[
            r#"<cite[^>]*>(.*?)</cite>"#,
            r#"<span[^>]*class="[^"]*\bb_attribution\b[^"]*"[^>]*>(.*?)</span>"#,
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
            // 寻找包含Bing搜索结果的行
            if line.contains("b_title") || (line.contains("<h2") && line.contains("href=")) {
                if let Some(url) = Self::extract_url_from_html(line) {
                    if let Some(title) = Self::extract_text_by_patterns(line, &[
                        r#"<h2[^>]*><a[^>]*>(.*?)</a></h2>"#,
                        r#"<a[^>]*>(.*?)</a>"#,
                    ]) {
                        // 寻找可能的描述
                        let snippet = if let Some(next_line) = lines.get(index + 1) {
                            if next_line.contains("b_caption") {
                                Self::extract_text_by_patterns(next_line, &[
                                    r#"<p[^>]*>(.*?)</p>"#,
                                    r#"<span[^>]*>(.*?)</span>"#,
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
    
    /// 提取搜索结果总数
    fn extract_total_results(html: &str) -> Option<u64> {
        let patterns = [
            r"(\d+(?:,\d+)*)\s*结果",
            r"(\d+(?:,\d+)*)\s*results",
            r"of\s+(\d+(?:,\d+)*)\s+results",
        ];
        
        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(cap) = re.captures(html) {
                    if let Some(num_str) = cap.get(1) {
                        let num_clean = num_str.as_str().replace(',', "");
                        if let Ok(num) = num_clean.parse::<u64>() {
                            return Some(num);
                        }
                    }
                }
            }
        }
        None
    }
}