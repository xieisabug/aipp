use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SearchEngine {
    Google,
    Bing,
    DuckDuckGo,
    Kagi,
}

impl SearchEngine {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "google" => Some(SearchEngine::Google),
            "bing" => Some(SearchEngine::Bing),
            "duckduckgo" | "ddg" => Some(SearchEngine::DuckDuckGo),
            "kagi" => Some(SearchEngine::Kagi),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SearchEngine::Google => "google",
            SearchEngine::Bing => "bing",
            SearchEngine::DuckDuckGo => "duckduckgo",
            SearchEngine::Kagi => "kagi",
        }
    }

    /// 获取默认的等待选择器
    pub fn default_wait_selectors(&self) -> Vec<String> {
        match self {
            SearchEngine::Google => super::engines::google::GoogleEngine::default_wait_selectors(),
            SearchEngine::Bing => super::engines::bing::BingEngine::default_wait_selectors(),
            SearchEngine::DuckDuckGo => super::engines::duckduckgo::DuckDuckGoEngine::default_wait_selectors(),
            SearchEngine::Kagi => super::engines::kagi::KagiEngine::default_wait_selectors(),
        }
    }

    /// 获取搜索引擎的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            SearchEngine::Google => super::engines::google::GoogleEngine::display_name(),
            SearchEngine::Bing => super::engines::bing::BingEngine::display_name(),
            SearchEngine::DuckDuckGo => super::engines::duckduckgo::DuckDuckGoEngine::display_name(),
            SearchEngine::Kagi => super::engines::kagi::KagiEngine::display_name(),
        }
    }

    /// 获取搜索引擎的首页URL
    pub fn homepage_url(&self) -> &'static str {
        match self {
            SearchEngine::Google => super::engines::google::GoogleEngine::homepage_url(),
            SearchEngine::Bing => super::engines::bing::BingEngine::homepage_url(),
            SearchEngine::DuckDuckGo => super::engines::duckduckgo::DuckDuckGoEngine::homepage_url(),
            SearchEngine::Kagi => super::engines::kagi::KagiEngine::homepage_url(),
        }
    }

    /// 获取搜索框选择器（优先级从高到低）
    pub fn search_input_selectors(&self) -> Vec<&'static str> {
        match self {
            SearchEngine::Google => super::engines::google::GoogleEngine::search_input_selectors(),
            SearchEngine::Bing => super::engines::bing::BingEngine::search_input_selectors(),
            SearchEngine::DuckDuckGo => super::engines::duckduckgo::DuckDuckGoEngine::search_input_selectors(),
            SearchEngine::Kagi => super::engines::kagi::KagiEngine::search_input_selectors(),
        }
    }

    /// 获取搜索按钮选择器（优先级从高到低）
    pub fn search_button_selectors(&self) -> Vec<&'static str> {
        match self {
            SearchEngine::Google => super::engines::google::GoogleEngine::search_button_selectors(),
            SearchEngine::Bing => super::engines::bing::BingEngine::search_button_selectors(),
            SearchEngine::DuckDuckGo => super::engines::duckduckgo::DuckDuckGoEngine::search_button_selectors(),
            SearchEngine::Kagi => super::engines::kagi::KagiEngine::search_button_selectors(),
        }
    }

}

pub struct SearchEngineManager {
    preferred_engine: Option<SearchEngine>,
}

impl SearchEngineManager {
    pub fn new(engine_config: Option<&str>) -> Self {
        let preferred_engine = engine_config
            .and_then(|s| SearchEngine::from_str(s));
        
        Self { preferred_engine }
    }

    /// 获取可用的搜索引擎，使用降级策略：Google -> Bing
    pub fn get_search_engine(&self) -> SearchEngine {
        // 先尝试用户配置的搜索引擎（或默认Google）
        let primary_engine = self.preferred_engine
            .as_ref()
            .unwrap_or(&SearchEngine::Google);

        // TODO: 这里可以添加搜索引擎可用性检测
        // 现在先直接返回主选引擎，如果需要降级逻辑可以在这里添加
        primary_engine.clone()
    }

    /// 获取搜索引擎的等待选择器（用户配置优先，否则使用默认值）
    pub fn get_wait_selectors(&self, engine: &SearchEngine, custom_selectors: Option<&str>) -> Vec<String> {
        if let Some(custom) = custom_selectors {
            custom.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            engine.default_wait_selectors()
        }
    }

    /// 尝试降级到备用搜索引擎
    pub fn get_fallback_engine(&self, current: &SearchEngine) -> Option<SearchEngine> {
        match current {
            SearchEngine::Google => Some(SearchEngine::Bing),
            SearchEngine::Bing => None, // Bing是最后的降级选项
            SearchEngine::DuckDuckGo => Some(SearchEngine::Bing),
            SearchEngine::Kagi => Some(SearchEngine::Google),
        }
    }
}
