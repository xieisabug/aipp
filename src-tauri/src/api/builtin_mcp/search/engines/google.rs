use super::base::SearchEngineBase;
use crate::api::builtin_mcp::search::types::{SearchItem, SearchResults};

/// Google搜索引擎实现
pub struct GoogleEngine;

impl GoogleEngine {
    pub fn display_name() -> &'static str {
        "Google"
    }
    
    pub fn homepage_url() -> &'static str {
        "https://www.google.com"
    }
    
    pub fn search_input_selectors() -> Vec<&'static str> {
        vec![
            "textarea[name='q']",
            "input[name='q']",
            "textarea[title='搜索']",
            "input[title='搜索']",
            "textarea[title='Search']", 
            "input[title='Search']",
            "#APjFqb",
            ".gLFyf",
            ".a4bIc input",
            "form[role='search'] textarea",
            "form[role='search'] input",
        ]
    }
    
    pub fn search_button_selectors() -> Vec<&'static str> {
        vec![
            "input[name='btnK']",
            "button[type='submit']",
            ".FPdoLc input[name='btnK']",
            ".tfB0Bf input[name='btnK']",
        ]
    }
    
    pub fn default_wait_selectors() -> Vec<String> {
        vec![
            "#search".to_string(),
            "#main".to_string(),
            "#rcnt".to_string(),
            "#center_col".to_string(),
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
    
    /// 解析Google搜索结果HTML，提取结构化信息
    pub fn parse_search_results(html: &str, query: &str) -> SearchResults {
        let mut items = Vec::new();
        
        // 使用简单的字符串匹配来解析HTML
        // Google搜索结果通常包含在具有特定类名的div中
        
        // 查找所有搜索结果项，Google通常使用 .g 类或 [data-ved] 属性
        let result_patterns = [
            // 标准搜索结果模式
            r#"<div[^>]*class="[^"]*\bg\b[^"]*"[^>]*>(.*?)</div>"#,
            // 带data-ved的搜索结果
            r#"<div[^>]*data-ved="[^"]*"[^>]*class="[^"]*\bg\b[^"]*"[^>]*>(.*?)</div>"#,
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
                            
                            // 限制结果数量，避免过多结果
                            if items.len() >= 20 {
                                break;
                            }
                        }
                    }
                }
                
                if !items.is_empty() {
                    break; // 如果找到结果就不再尝试其他模式
                }
            }
        }
        
        // 如果使用正则表达式失败，尝试更简单的文本搜索方法
        if items.is_empty() {
            items = Self::fallback_parse_results(html);
        }
        
        // 提取总结果数量（如果可获取）
        let total_results = Self::extract_total_results(html);
        
        SearchResults {
            query: query.to_string(),
            search_engine: Self::display_name().to_string(),
            engine_id: "google".to_string(),
            homepage_url: Self::homepage_url().to_string(),
            items,
            total_results,
            search_time_ms: None, // 可以在后续版本中添加计时功能
        }
    }
    
    /// 解析单个搜索结果
    fn parse_single_result(html: &str, rank: usize) -> Option<SearchItem> {
        // 提取标题 - Google通常使用 h3 标签，可能在 a 标签内
        let title = Self::extract_text_by_patterns(html, &[
            r#"<h3[^>]*>(.*?)</h3>"#,
            r#"<a[^>]*><h3[^>]*>(.*?)</h3></a>"#,
            r#"<div[^>]*role="heading"[^>]*>(.*?)</div>"#,
        ]).unwrap_or_else(|| format!("Search Result {}", rank));
        
        // 提取URL - 通常在 a 标签的 href 属性中
        let url = Self::extract_url_from_html(html).unwrap_or_default();
        
        // 提取摘要/描述 - 通常在span或div中，Google使用特定的类名
        let snippet = Self::extract_text_by_patterns(html, &[
            r#"<span[^>]*class="[^"]*\bst\b[^"]*"[^>]*>(.*?)</span>"#,
            r#"<div[^>]*class="[^"]*\bVwiC3b\b[^"]*"[^>]*>(.*?)</div>"#,
            r#"<span[^>]*data-ved="[^"]*"[^>]*>(.*?)</span>"#,
        ]).unwrap_or_default();
        
        // 提取显示URL（绿色URL显示）
        let display_url = Self::extract_text_by_patterns(html, &[
            r#"<cite[^>]*>(.*?)</cite>"#,
            r#"<span[^>]*class="[^"]*\bdDKKM\b[^"]*"[^>]*>(.*?)</span>"#,
        ]);
        
        // 只有在有基本信息的情况下才创建结果项
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
    
    /// 备用解析方法，使用更简单的文本搜索
    fn fallback_parse_results(html: &str) -> Vec<SearchItem> {
        let mut items = Vec::new();
        
        // 查找常见的Google搜索结果标记
        let lines: Vec<&str> = html.lines().collect();
        let mut current_title: Option<String> = None;
        let mut current_url: Option<String> = None;
        let mut current_snippet: Option<String> = None;
        let mut rank = 1;
        
        for line in lines {
            // 简单匹配，寻找包含链接和标题的行
            if line.contains("<h3") && line.contains("href=") {
                if let Some(url) = Self::extract_url_from_html(line) {
                    if let Some(title) = Self::extract_text_by_patterns(line, &[r#"<h3[^>]*>(.*?)</h3>"#]) {
                        current_title = Some(Self::clean_html_text(&title));
                        current_url = Some(url);
                    }
                }
            }
            
            // 如果我们有标题和URL，寻找描述
            if current_title.is_some() && current_url.is_some() && line.contains("span") {
                if let Some(desc) = Self::extract_text_by_patterns(line, &[r#"<span[^>]*>(.*?)</span>"#]) {
                    current_snippet = Some(Self::clean_html_text(&desc));
                }
            }
            
            // 如果我们收集到了完整的信息，创建一个结果项
            if let (Some(title), Some(url)) = (&current_title, &current_url) {
                items.push(SearchItem {
                    title: title.clone(),
                    url: url.clone(),
                    snippet: current_snippet.clone().unwrap_or_default(),
                    rank,
                    display_url: None,
                });
                
                // 重置当前项
                current_title = None;
                current_url = None;
                current_snippet = None;
                rank += 1;
                
                if items.len() >= 10 {
                    break;
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
        // Google搜索结果中的URL可能是重定向URL，我们需要解析实际URL
        let url_patterns = [
            r#"href="(/url\?q=[^"]*)"#,
            r#"href="(https?://[^"]*)"#,
            r#"href="([^"]*)"#,
        ];
        
        for pattern in &url_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(cap) = re.captures(html) {
                    if let Some(url_match) = cap.get(1) {
                        let url = url_match.as_str();
                        
                        // 如果是Google的重定向URL，尝试解析实际URL
                        if url.starts_with("/url?q=") {
                            if let Some(actual_url) = Self::decode_google_url(url) {
                                return Some(actual_url);
                            }
                        } else if url.starts_with("http") {
                            return Some(url.to_string());
                        }
                    }
                }
            }
        }
        None
    }
    
    /// 解码Google重定向URL
    fn decode_google_url(url: &str) -> Option<String> {
        if let Some(q_start) = url.find("q=") {
            let q_part = &url[q_start + 2..];
            if let Some(end) = q_part.find('&') {
                let encoded_url = &q_part[..end];
                return urlencoding::decode(encoded_url).ok().map(|s| s.into_owned());
            } else {
                return urlencoding::decode(q_part).ok().map(|s| s.into_owned());
            }
        }
        None
    }
    
    /// 清理HTML文本
    fn clean_html_text(text: &str) -> String {
        // 移除HTML标签和多余的空白字符
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
        
        // 清理多余的空白字符
        let re = regex::Regex::new(r"\s+").unwrap_or_else(|_| regex::Regex::new(r"").unwrap());
        re.replace_all(&cleaned, " ").trim().to_string()
    }
    
    /// 提取搜索结果总数
    fn extract_total_results(html: &str) -> Option<u64> {
        let patterns = [
            r"About ([\d,]+) results",
            r"大约 ([\d,]+) 条结果",
            r"(\d+) 个结果",
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