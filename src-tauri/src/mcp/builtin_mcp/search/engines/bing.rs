use crate::mcp::builtin_mcp::search::types::{SearchItem, SearchResults};
use scraper::{Html, Selector};

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
    
    
    /// 解析Bing搜索结果HTML，提取结构化信息（HTML解析器版）
    pub fn parse_search_results(html: &str, query: &str) -> SearchResults {
        let mut items = Vec::new();
        let document = Html::parse_document(html);

        // 结果卡片选择器（Bing通常使用 .b_algo 类）
        let selectors = [
            Selector::parse("li.b_algo").ok(),
            Selector::parse("div.b_algo").ok(),
        ];

        let mut rank = 1usize;
        for sel in selectors.iter().flatten() {
            for card in document.select(sel) {
                if let Some(item) = Self::parse_card_element(card, rank) {
                    items.push(item);
                    rank += 1;
                    if items.len() >= 20 { break; }
                }
            }
            if !items.is_empty() { break; }
        }

        // 提取总结结果数量
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
    
    
    
    
    /// 从结果卡片元素中抽取一个条目
    fn parse_card_element(card: scraper::ElementRef<'_>, rank: usize) -> Option<SearchItem> {
        // 标题：Bing 通常使用 h2 > a 结构或 .b_title 类
        let title = Self::first_text_in(card, &["h2 a", "a.b_title", "h2"])
            .unwrap_or_else(|| format!("Bing Result {}", rank));

        // URL：寻找标题链接
        let url = Self::first_href_in(card, &["h2 a", "a.b_title", "a[href]"]).unwrap_or_default();

        // 摘要：Bing 使用 .b_caption 类或其他描述元素
        let snippet = Self::first_text_in(card, &["p.b_caption", "div.b_caption", "span.b_snippetBigText", "p", "div"]).unwrap_or_default();

        // 显示 URL
        let display_url = Self::first_text_in(card, &["cite", "span.b_attribution"]);

        if !title.trim().is_empty() && !url.trim().is_empty() {
            Some(SearchItem {
                title: title.trim().to_string(),
                url,
                snippet: snippet.trim().to_string(),
                rank,
                display_url: display_url.map(|s| s.trim().to_string()),
            })
        } else {
            None
        }
    }

    /// 在元素内按给定选择器列表找到首个文本
    fn first_text_in(root: scraper::ElementRef<'_>, selectors: &[&str]) -> Option<String> {
        for sel in selectors {
            if let Ok(selector) = Selector::parse(sel) {
                if let Some(node) = root.select(&selector).next() {
                    let text = node.text().collect::<String>();
                    let text = text.trim();
                    if !text.is_empty() { return Some(text.to_string()); }
                }
            }
        }
        None
    }

    /// 在元素内按选择器列表找到首个链接的真实 URL
    fn first_href_in(root: scraper::ElementRef<'_>, selectors: &[&str]) -> Option<String> {
        for sel in selectors {
            if let Ok(selector) = Selector::parse(sel) {
                for node in root.select(&selector) {
                    if let Some(href) = node.value().attr("href") {
                        if href.starts_with("http") {
                            return Some(href.to_string());
                        }
                    }
                }
            }
        }
        None
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
