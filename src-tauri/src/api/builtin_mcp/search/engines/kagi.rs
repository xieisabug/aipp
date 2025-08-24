use super::base::SearchEngineBase;

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
}