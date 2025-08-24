use super::base::SearchEngineBase;

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
}