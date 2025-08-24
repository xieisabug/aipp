use super::base::SearchEngineBase;

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
}