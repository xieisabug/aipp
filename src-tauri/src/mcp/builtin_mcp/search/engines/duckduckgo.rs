use crate::mcp::builtin_mcp::search::types::{SearchItem, SearchResults};
use scraper::{Html, Selector};

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
    
    
    /// 解析DuckDuckGo搜索结果HTML，提取结构化信息（HTML解析器版）
    pub fn parse_search_results(html: &str, query: &str) -> SearchResults {
        let mut items = Vec::new();
        let document = Html::parse_document(html);

        // 结果卡片选择器（DuckDuckGo通常使用 .result 类）
        let selectors = [
            Selector::parse("div.result").ok(),
            Selector::parse("article.result").ok(),
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
    
    /// 从结果卡片元素中抽取一个条目
    fn parse_card_element(card: scraper::ElementRef<'_>, rank: usize) -> Option<SearchItem> {
        // 标题：DuckDuckGo 通常使用 h2 a 或 .result__title 类
        let title = Self::first_text_in(card, &["h2 a", "h3 a", "a.result__title", "h2", "h3"])
            .unwrap_or_else(|| format!("DuckDuckGo Result {}", rank));

        // URL：寻找标题链接
        let url = Self::first_href_in(card, &["h2 a", "h3 a", "a.result__title", "a[href]"]).unwrap_or_default();

        // 摘要：DuckDuckGo 使用 .result__snippet 类或其他描述元素
        let snippet = Self::first_text_in(card, &["span.result__snippet", "div.result__snippet", "p", "div"]).unwrap_or_default();

        if !title.trim().is_empty() && !url.trim().is_empty() {
            Some(SearchItem {
                title: title.trim().to_string(),
                url,
                snippet: snippet.trim().to_string(),
                rank,
                display_url: None,
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
}
