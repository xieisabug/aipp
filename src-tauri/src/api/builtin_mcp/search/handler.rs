use tauri::AppHandle;
use std::collections::HashMap;
use super::browser::BrowserManager;
use super::engines::{SearchEngine, SearchEngineManager};
use super::fetcher::{ContentFetcher, FetchConfig};

#[derive(Clone)]
pub struct SearchHandler {
    app_handle: AppHandle,
}

impl SearchHandler {
    pub fn new(app_handle: AppHandle) -> Self {
        println!("[SEARCH] Creating SearchHandler");
        Self { app_handle }
    }

    /// 执行网络搜索
    pub async fn search_web(&self, query: &str) -> Result<serde_json::Value, String> {
        println!("[SEARCH] Starting web search for query: {}", query);

        let config = self.load_search_config()?;
        let browser_manager = BrowserManager::new(config.get("BROWSER_TYPE").map(|s| s.as_str()));
        let engine_manager = SearchEngineManager::new(config.get("SEARCH_ENGINE").map(|s| s.as_str()));

        let search_engine = engine_manager.get_search_engine();
        
        println!("[SEARCH] Using {} engine, homepage: {}", search_engine.display_name(), search_engine.homepage_url());

        let fetch_config = self.build_fetch_config(&config, &engine_manager, &search_engine)?;
        let fetcher = ContentFetcher::new(self.app_handle.clone(), fetch_config);

        match fetcher.fetch_search_content(query, &search_engine, &browser_manager).await {
            Ok(html) => {
                println!("[SEARCH] Successfully fetched search results");
                Ok(serde_json::json!({
                    "query": query,
                    "homepage_url": search_engine.homepage_url(),
                    "search_engine": search_engine.display_name(),
                    "engine_id": search_engine.as_str(),
                    "html_content": html,
                    "message": format!("Search completed using {}", search_engine.display_name()),
                }))
            }
            Err(e) => {
                // 尝试降级到其他搜索引擎
                if let Some(fallback_engine) = engine_manager.get_fallback_engine(&search_engine) {
                    println!("[SEARCH] Trying fallback engine: {}", fallback_engine.display_name());
                    let fallback_config = self.build_fetch_config(&config, &engine_manager, &fallback_engine)?;
                    let fallback_fetcher = ContentFetcher::new(self.app_handle.clone(), fallback_config);
                    
                    match fallback_fetcher.fetch_search_content(query, &fallback_engine, &browser_manager).await {
                        Ok(html) => {
                            println!("[SEARCH] Fallback search successful");
                            return Ok(serde_json::json!({
                                "query": query,
                                "homepage_url": fallback_engine.homepage_url(),
                                "search_engine": fallback_engine.display_name(),
                                "engine_id": fallback_engine.as_str(),
                                "html_content": html,
                                "message": format!("Search completed using fallback engine {}", fallback_engine.display_name()),
                            }));
                        }
                        Err(fallback_error) => {
                            println!("[SEARCH] Fallback also failed: {}", fallback_error);
                        }
                    }
                }

                Err(format!("Search failed: {}", e))
            }
        }
    }

    /// 抓取指定URL的内容
    pub async fn fetch_url(&self, url: &str) -> Result<serde_json::Value, String> {
        println!("[SEARCH] Fetching URL: {}", url);

        let config = self.load_search_config()?;
        let browser_manager = BrowserManager::new(config.get("BROWSER_TYPE").map(|s| s.as_str()));
        
        let fetch_config = self.build_general_fetch_config(&config)?;
        let fetcher = ContentFetcher::new(self.app_handle.clone(), fetch_config);

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
        use crate::db::mcp_db::MCPDatabase;
        
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
            user_agent: config.get("USER_AGENT").cloned(),
            bypass_csp: config.get("BYPASS_CSP")
                .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false),
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
            user_agent: config.get("USER_AGENT").cloned(),
            bypass_csp: config.get("BYPASS_CSP")
                .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false),
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