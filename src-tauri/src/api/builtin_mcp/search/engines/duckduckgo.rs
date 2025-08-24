use super::base::SearchEngineBase;

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
}