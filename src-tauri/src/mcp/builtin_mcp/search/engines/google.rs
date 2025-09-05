use crate::mcp::builtin_mcp::search::types::{SearchItem, SearchResults};
use scraper::{Html, Selector};

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
            "textarea[name='q']",                     // 新版 Google 主搜索框
            "input[name='q']",                        // 传统搜索框
            "form[action='/search'] input[name='q']", // 带 action 的搜索表单
            "form[action*='google.'][role='search'] input[name='q']", // 更通用的表单匹配
            "textarea[title='搜索']",                 // 中文界面搜索框
            "input[title='搜索']",
            "textarea[title='Search']", // 英文界面搜索框
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
            "input[name='btnK']",           // 标准 Google 搜索按钮
            "button[type='submit']",        // 通用提交按钮
            "input[value='Google 搜索']",   // 中文搜索按钮
            "input[value='Google Search']", // 英文搜索按钮
            ".FPdoLc input[name='btnK']",   // 容器内的搜索按钮
            ".tfB0Bf input[name='btnK']",   // 另一个容器
            // 高级选择器
            "center input[name='btnK']", // 居中容器内的按钮
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
            "#search".to_string(),     // Google 主搜索结果容器
            "#main".to_string(),       // 主内容区域
            "#rcnt".to_string(),       // 结果计数容器
            "#center_col".to_string(), // 中心列
            // 具体结果选择器
            "[data-ved]".to_string(), // Google 结果项标识
            ".g".to_string(),         // Google 搜索结果项类名
            ".tF2Cxc".to_string(),    // 新版结果项类名
            ".yuRUbf".to_string(),    // 结果标题容器
            // 其他有效容器
            "#rso".to_string(),       // 搜索结果区域
            ".srp".to_string(),       // 搜索结果页面
            "#topads".to_string(),    // 广告区域（也表明页面已加载）
            "#bottomads".to_string(), // 底部广告
            // 错误页面或特殊情况
            ".med".to_string(),                // 消息区域
            "#errorPageContainer".to_string(), // 错误页面
        ]
    }

    /// 解析Google搜索结果HTML，提取结构化信息（HTML解析器版）
    pub fn parse_search_results(html: &str, query: &str) -> SearchResults {
        let mut items = Vec::new();
        let document = Html::parse_document(html);

        // 结果卡片选择器（优先新版本，再到通用）
        let selectors = [Selector::parse("div.tF2Cxc").ok(), Selector::parse("div.g").ok()];

        let mut rank = 1usize;
        for sel in selectors.iter().flatten() {
            for card in document.select(sel) {
                if let Some(item) = Self::parse_card_element(card, rank) {
                    items.push(item);
                    rank += 1;
                    if items.len() >= 20 {
                        break;
                    }
                }
            }
            if !items.is_empty() {
                break;
            }
        }

        // 提取搜索结果总数（如果可获取）
        let total_results = Self::extract_total_results(html);

        SearchResults {
            query: query.to_string(),
            search_engine: Self::display_name().to_string(),
            engine_id: "google".to_string(),
            homepage_url: Self::homepage_url().to_string(),
            items,
            total_results,
            search_time_ms: None,
        }
    }

    /// 从结果卡片元素中抽取一个条目
    fn parse_card_element(card: scraper::ElementRef<'_>, rank: usize) -> Option<SearchItem> {
        // 标题：优先 .yuRUbf h3，其次任意 h3 / role=heading
        let title = Self::first_text_in(card, &[".yuRUbf h3", "h3", "[role=heading]"])
            .unwrap_or_else(|| format!("Search Result {}", rank));

        // URL：优先 .yuRUbf a[href]，否则任意 a[href]
        let url = Self::first_href_in(card, &[".yuRUbf a[href]", "a[href]"]).unwrap_or_default();

        // 摘要：兼容多版本类名
        let snippet = Self::first_text_in(card, &["div.VwiC3b", "span.VwiC3b", "span[data-ved]"])
            .unwrap_or_default();

        // 显示 URL（有些页面会有）
        let display_url = Self::first_text_in(card, &["cite", "span.dDKKM"]);

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

    /// 在元素内按选择器列表找到首个文本
    fn first_text_in(root: scraper::ElementRef<'_>, selectors: &[&str]) -> Option<String> {
        for sel in selectors {
            if let Ok(selector) = Selector::parse(sel) {
                if let Some(node) = root.select(&selector).next() {
                    let text = node.text().collect::<String>();
                    let text = text.trim();
                    if !text.is_empty() {
                        return Some(text.to_string());
                    }
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
                        if href.starts_with("/url?q=") {
                            if let Some(actual) = Self::decode_google_url(href) {
                                return Some(actual);
                            }
                        } else if href.starts_with("http") {
                            return Some(href.to_string());
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

    /// 提取搜索结果总数
    fn extract_total_results(html: &str) -> Option<u64> {
        let patterns = [r"About ([\d,]+) results", r"大约 ([\d,]+) 条结果", r"(\d+) 个结果"];

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
