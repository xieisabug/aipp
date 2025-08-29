use tauri::AppHandle;
use std::collections::HashMap;
use super::browser::BrowserManager;
use super::engine_manager::{SearchEngine, SearchEngineManager};
use super::fetcher::{ContentFetcher, FetchConfig};
use super::types::{SearchRequest, SearchResponse, SearchResultType};
use super::engines::base::SearchEngineBase;

#[derive(Clone)]
pub struct SearchHandler {
    app_handle: AppHandle,
}

impl SearchHandler {
    pub fn new(app_handle: AppHandle) -> Self {
        println!("[SEARCH] Creating SearchHandler");
        Self { app_handle }
    }

    /// 执行带结果类型的网络搜索
    pub async fn search_web_with_type(&self, request: SearchRequest) -> Result<SearchResponse, String> {
        println!("[SEARCH] Starting web search with type {:?} for query: {}", request.result_type, request.query);

        let config = self.load_search_config()?;
        let browser_manager = BrowserManager::new(config.get("BROWSER_TYPE").map(|s| s.as_str()));
        let engine_manager = SearchEngineManager::new(config.get("SEARCH_ENGINE").map(|s| s.as_str()));

        let search_engine = engine_manager.get_search_engine();
        
        println!("[SEARCH] Using {} engine for {:?} results", search_engine.display_name(), request.result_type);

        // 首先获取HTML内容
        match self.fetch_search_html(&request.query, &search_engine, &browser_manager, &config).await {
            Ok(html) => {
                // 根据结果类型处理HTML
                self.process_html_by_type(html, &request, &search_engine)
            }
            Err(e) => {
                // 尝试降级到其他搜索引擎
                if let Some(fallback_engine) = engine_manager.get_fallback_engine(&search_engine) {
                    println!("[SEARCH] Trying fallback engine: {} for typed search", fallback_engine.display_name());
                    
                    match self.fetch_search_html(&request.query, &fallback_engine, &browser_manager, &config).await {
                        Ok(html) => {
                            self.process_html_by_type(html, &request, &fallback_engine)
                        }
                        Err(fallback_error) => {
                            println!("[SEARCH] Fallback also failed: {}", fallback_error);
                            Err(format!("Search failed: {} (fallback also failed: {})", e, fallback_error))
                        }
                    }
                } else {
                    Err(format!("Search failed: {}", e))
                }
            }
        }
    }

    /// 获取搜索HTML内容
    async fn fetch_search_html(
        &self,
        query: &str,
        search_engine: &SearchEngine,
        browser_manager: &BrowserManager,
        config: &std::collections::HashMap<String, String>,
    ) -> Result<String, String> {
        let fetch_config = self.build_fetch_config(config, &SearchEngineManager::new(None), search_engine)?;
        let mut fetcher = ContentFetcher::new(self.app_handle.clone(), fetch_config);
        fetcher.fetch_search_content(query, search_engine, browser_manager).await
    }

    /// 根据结果类型处理HTML
    fn process_html_by_type(
        &self,
        html: String,
        request: &SearchRequest,
        search_engine: &SearchEngine,
    ) -> Result<SearchResponse, String> {
        match request.result_type {
            SearchResultType::Html => {
                Ok(SearchResponse::Html {
                    query: request.query.clone(),
                    homepage_url: search_engine.homepage_url().to_string(),
                    search_engine: search_engine.display_name().to_string(),
                    engine_id: search_engine.as_str().to_string(),
                    html_content: html,
                    message: format!("Successfully retrieved HTML search results from {}", search_engine.display_name()),
                })
            }
            SearchResultType::Markdown => {
                let markdown_content = SearchEngineBase::html_to_markdown(&html);
                Ok(SearchResponse::Markdown {
                    query: request.query.clone(),
                    homepage_url: search_engine.homepage_url().to_string(),
                    search_engine: search_engine.display_name().to_string(),
                    engine_id: search_engine.as_str().to_string(),
                    markdown_content,
                    message: format!("Successfully converted {} search results to Markdown format", search_engine.display_name()),
                })
            }
            SearchResultType::Items => {
                let search_results = match search_engine {
                    SearchEngine::Google => super::engines::google::GoogleEngine::parse_search_results(&html, &request.query),
                    SearchEngine::Bing => super::engines::bing::BingEngine::parse_search_results(&html, &request.query),
                    SearchEngine::DuckDuckGo => super::engines::duckduckgo::DuckDuckGoEngine::parse_search_results(&html, &request.query),
                    SearchEngine::Kagi => super::engines::kagi::KagiEngine::parse_search_results(&html, &request.query),
                };
                // 返回简化格式，仅包含搜索结果项数组
                Ok(SearchResponse::ItemsOnly(search_results.items))
            }
        }
    }

    /// 抓取指定URL的内容，支持多种格式
    pub async fn fetch_url_with_type(&self, url: &str, result_type: &str) -> Result<String, String> {
        println!("[SEARCH] Fetching URL with type {}: {}", result_type, url);

        let config = self.load_search_config()?;
        let browser_manager = BrowserManager::new(config.get("BROWSER_TYPE").map(|s| s.as_str()));
        
        let fetch_config = self.build_general_fetch_config(&config)?;
        let mut fetcher = ContentFetcher::new(self.app_handle.clone(), fetch_config);

        match fetcher.fetch_content(url, &browser_manager).await {
            Ok(html) => {
                println!("[SEARCH] Successfully fetched URL content");
                
                match result_type {
                    "markdown" => {
                        let markdown_content = SearchEngineBase::html_to_markdown(&html);
                        Ok(markdown_content)
                    }
                    "html" | _ => {
                        Ok(html)
                    }
                }
            }
            Err(e) => {
                Err(format!("Failed to fetch URL: {}", e))
            }
        }
    }

    /// 抓取指定URL的内容（保持向后兼容）
    pub async fn fetch_url(&self, url: &str) -> Result<serde_json::Value, String> {
        println!("[SEARCH] Fetching URL: {}", url);

        let config = self.load_search_config()?;
        let browser_manager = BrowserManager::new(config.get("BROWSER_TYPE").map(|s| s.as_str()));
        
        let fetch_config = self.build_general_fetch_config(&config)?;
        let mut fetcher = ContentFetcher::new(self.app_handle.clone(), fetch_config);

        match fetcher.fetch_content(url, &browser_manager).await {
            Ok(html) => {
                println!("[SEARCH] Successfully fetched URL content");
                Ok(serde_json::json!({
                    "url": url,
                    "status": "success",
                    "html_content": html,
                    "message": "URL fetched successfully",
                }))
            }
            Err(e) => {
                Err(format!("Failed to fetch URL: {}", e))
            }
        }
    }

    /// 从数据库加载搜索配置
    fn load_search_config(&self) -> Result<HashMap<String, String>, String> {
    use crate::mcp::mcp_db::MCPDatabase;
        
        let db = MCPDatabase::new(&self.app_handle).map_err(|e| e.to_string())?;
        
        // 查询内置搜索服务器的环境变量
        let mut stmt = db
            .conn
            .prepare("SELECT environment_variables FROM mcp_server WHERE command = ? AND is_builtin = 1 LIMIT 1")
            .map_err(|e| format!("Database prepare error: {}", e))?;

        let env_text: Option<String> = stmt.query_row(["aipp:search"], |row| {
            row.get::<_, Option<String>>(0)
        }).unwrap_or(None);

        let mut config = HashMap::new();
        if let Some(text) = env_text {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    config.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }

        Ok(config)
    }

    /// 构建针对搜索引擎的抓取配置
    fn build_fetch_config(&self, config: &HashMap<String, String>, engine_manager: &SearchEngineManager, search_engine: &SearchEngine) -> Result<FetchConfig, String> {
        let wait_selectors = engine_manager.get_wait_selectors(
            search_engine,
            config.get("WAIT_SELECTORS").map(|s| s.as_str())
        );

        Ok(FetchConfig {
            user_data_dir: config.get("USER_DATA_DIR").cloned(),
            proxy_server: config.get("PROXY_SERVER").cloned(),
            headless: config.get("HEADLESS")
                .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(true),
            user_agent: None,
            bypass_csp: false,
            wait_selectors,
            wait_timeout_ms: config.get("WAIT_TIMEOUT_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(15000),
            wait_poll_ms: config.get("WAIT_POLL_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(250),
        })
    }

    /// 构建通用的抓取配置（用于直接URL抓取）
    fn build_general_fetch_config(&self, config: &HashMap<String, String>) -> Result<FetchConfig, String> {
        let wait_selectors = if let Some(selectors_str) = config.get("WAIT_SELECTORS") {
            selectors_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec!["body".to_string(), "main".to_string(), "#content".to_string()]
        };

        Ok(FetchConfig {
            user_data_dir: config.get("USER_DATA_DIR").cloned(),
            proxy_server: config.get("PROXY_SERVER").cloned(),
            headless: config.get("HEADLESS")
                .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(true),
            user_agent: None,
            bypass_csp: false,
            wait_selectors,
            wait_timeout_ms: config.get("WAIT_TIMEOUT_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(15000),
            wait_poll_ms: config.get("WAIT_POLL_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(250),
        })
    }
}
