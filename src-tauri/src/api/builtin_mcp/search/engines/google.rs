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
            // 主要搜索框选择器（按优先级排序）
            "textarea[name='q']",           // 新版 Google 主搜索框
            "input[name='q']",              // 传统搜索框
            "textarea[title='搜索']",        // 中文界面搜索框
            "input[title='搜索']",           
            "textarea[title='Search']",      // 英文界面搜索框
            "input[title='Search']",
            "#APjFqb",                      // Google 特定 ID
            ".gLFyf",                       // Google 特定类名
            ".a4bIc input",                 // 容器内的输入框
            ".a4bIc textarea",              // 容器内的文本域
            "form[role='search'] textarea", // 表单内搜索框
            "form[role='search'] input[type='text']",
            "form[role='search'] input[type='search']",
            // 备用选择器（更广泛的匹配）
            "input[aria-label*='搜索']",
            "textarea[aria-label*='搜索']",
            "input[aria-label*='Search']",
            "textarea[aria-label*='Search']",
            "input[autocomplete='off'][name='q']",
            "textarea[autocomplete='off'][name='q']",
        ]
    }
    
    pub fn search_button_selectors() -> Vec<&'static str> {
        vec![
            // 主要搜索按钮选择器
            "input[name='btnK']",              // 标准 Google 搜索按钮
            "button[type='submit']",           // 通用提交按钮
            "input[value='Google 搜索']",       // 中文搜索按钮
            "input[value='Google Search']",    // 英文搜索按钮
            ".FPdoLc input[name='btnK']",      // 容器内的搜索按钮
            ".tfB0Bf input[name='btnK']",      // 另一个容器
            // 高级选择器
            "center input[name='btnK']",       // 居中容器内的按钮
            "form input[type='submit'][name='btnK']",
            "form button[aria-label*='搜索']",
            "form button[aria-label*='Search']",
            // 备用按钮选择器
            "input[type='submit'][value*='搜索']",
            "input[type='submit'][value*='Search']",
            "button[data-ved]:not([disabled])", // Google 特有的按钮
        ]
    }
    
    pub fn default_wait_selectors() -> Vec<String> {
        vec![
            // 主要搜索结果容器
            "#search".to_string(),             // Google 主搜索结果容器
            "#main".to_string(),               // 主内容区域
            "#rcnt".to_string(),               // 结果计数容器
            "#center_col".to_string(),         // 中心列
            // 具体结果选择器
            "[data-ved]".to_string(),          // Google 结果项标识
            ".g".to_string(),                  // Google 搜索结果项类名
            ".tF2Cxc".to_string(),            // 新版结果项类名
            ".yuRUbf".to_string(),            // 结果标题容器
            // 其他有效容器
            "#rso".to_string(),                // 搜索结果区域
            ".srp".to_string(),                // 搜索结果页面
            "#topads".to_string(),             // 广告区域（也表明页面已加载）
            "#bottomads".to_string(),          // 底部广告
            // 错误页面或特殊情况
            ".med".to_string(),                // 消息区域
            "#errorPageContainer".to_string(), // 错误页面
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