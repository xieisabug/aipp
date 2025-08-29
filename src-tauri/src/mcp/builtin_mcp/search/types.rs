use serde::{Deserialize, Serialize};

/// 搜索结果类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SearchResultType {
    /// 返回原始HTML内容
    Html,
    /// 返回转换后的Markdown内容
    Markdown,
    /// 返回结构化的搜索结果项
    Items,
}

impl Default for SearchResultType {
    fn default() -> Self {
        SearchResultType::Html
    }
}

impl SearchResultType {
    pub fn from_str(s: Option<&str>) -> Self {
        match s {
            Some("html") => SearchResultType::Html,
            Some("markdown") => SearchResultType::Markdown,
            Some("items") => SearchResultType::Items,
            _ => SearchResultType::default(),
        }
    }
}

/// 搜索请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// 搜索关键词
    pub query: String,
    /// 期望的结果类型（默认 Html）
    #[serde(default)]
    pub result_type: SearchResultType,
}

/// 单个搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItem {
    /// 结果标题
    pub title: String,
    /// 结果链接
    pub url: String,
    /// 结果摘要/描述
    pub snippet: String,
    /// 搜索结果排名（从1开始）
    pub rank: usize,
    /// 显示的URL（如果与实际URL不同）
    pub display_url: Option<String>,
}

/// 结构化搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// 搜索查询
    pub query: String,
    /// 搜索引擎名称
    pub search_engine: String,
    /// 搜索引擎ID
    pub engine_id: String,
    /// 搜索引擎首页URL
    pub homepage_url: String,
    /// 结果项列表
    pub items: Vec<SearchItem>,
    /// 总结果数量（如果可获取）
    pub total_results: Option<u64>,
    /// 搜索耗时（毫秒）
    pub search_time_ms: Option<u64>,
}

/// 搜索响应统一格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SearchResponse {
    /// HTML内容响应
    Html {
        query: String,
        homepage_url: String,
        search_engine: String,
        engine_id: String,
        html_content: String,
        message: String,
    },
    /// Markdown内容响应
    Markdown {
        query: String,
        homepage_url: String,
        search_engine: String,
        engine_id: String,
        markdown_content: String,
        message: String,
    },
    /// 结构化结果响应（完整对象）
    Items(SearchResults),
    /// 简化的搜索结果响应（仅包含结果项数组）
    ItemsOnly(Vec<SearchItem>),
}
