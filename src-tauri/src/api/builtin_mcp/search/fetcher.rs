use std::path::PathBuf;
use std::time::Duration;
use std::fs;
use tauri::{AppHandle, Manager};
use tokio::process::Command as TokioCommand;
use playwright::Playwright;
use super::browser::BrowserManager;
use super::engines::SearchEngine;

#[derive(Debug, Clone)]
pub struct FetchConfig {
    pub user_data_dir: Option<String>,
    pub proxy_server: Option<String>,
    pub headless: bool,
    pub user_agent: Option<String>,
    pub bypass_csp: bool,
    pub wait_selectors: Vec<String>,
    pub wait_timeout_ms: u64,
    pub wait_poll_ms: u64,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            user_data_dir: None,
            proxy_server: None,
            headless: true,
            user_agent: None,
            bypass_csp: false,
            wait_selectors: vec![],
            wait_timeout_ms: 15000,
            wait_poll_ms: 250,
        }
    }
}

pub struct ContentFetcher {
    app_handle: AppHandle,
    config: FetchConfig,
    search_engine: Option<SearchEngine>,
}

impl ContentFetcher {
    pub fn new(app_handle: AppHandle, config: FetchConfig) -> Self {
        Self { app_handle, config, search_engine: None }
    }
    
    pub fn with_search_engine(app_handle: AppHandle, config: FetchConfig, search_engine: SearchEngine) -> Self {
        Self { app_handle, config, search_engine: Some(search_engine) }
    }

    /// 主要的内容抓取方法，按优先级尝试不同策略
    pub async fn fetch_content(&self, url: &str, browser_manager: &BrowserManager) -> Result<String, String> {
        println!("[FETCHER] Starting content fetch for: {}", url);

        // 策略1: Playwright（最优，支持复杂动态内容）
        match self.fetch_with_playwright(url, browser_manager).await {
            Ok(html) => {
                println!("[FETCHER][PW] Successfully fetched {} bytes", html.len());
                return Ok(html);
            }
            Err(e) => {
                println!("[FETCHER][PW] Failed: {}", e);
            }
        }

        // 策略2: Headless Browser（次优，轻量级）
        match self.fetch_with_headless_browser(url, browser_manager).await {
            Ok(html) => {
                println!("[FETCHER][HEADLESS] Successfully fetched {} bytes", html.len());
                return Ok(html);
            }
            Err(e) => {
                println!("[FETCHER][HEADLESS] Failed: {}", e);
            }
        }

        // 策略3: HTTP直接请求（兜底，适合静态内容）
        match self.fetch_with_http(url).await {
            Ok(html) => {
                println!("[FETCHER][HTTP] Successfully fetched {} bytes", html.len());
                return Ok(html);
            }
            Err(e) => {
                println!("[FETCHER][HTTP] Failed: {}", e);
            }
        }

        // 策略4: WebView兜底（不提取内容，仅导航）
        self.fallback_webview_navigation(url).await
    }

    /// 为搜索请求定制的获取方法
    pub async fn fetch_search_content(&self, query: &str, search_engine: &SearchEngine, browser_manager: &BrowserManager) -> Result<String, String> {
        println!("[FETCHER] Starting search content fetch for query: {}", query);

        // 使用Playwright执行搜索流程
        match self.fetch_search_with_playwright(query, search_engine, browser_manager).await {
            Ok(html) => {
                println!("[FETCHER][SEARCH] Successfully fetched {} bytes", html.len());
                return Ok(html);
            }
            Err(e) => {
                println!("[FETCHER][SEARCH] Search flow failed: {}", e);
            }
        }

        // 搜索流程失败，不再降级到直接URL访问
        Err(format!("Search flow failed for {} engine: {}", search_engine.display_name(), "All interactive search attempts failed"))
    }

    /// 使用Playwright执行搜索流程
    async fn fetch_search_with_playwright(&self, query: &str, search_engine: &SearchEngine, browser_manager: &BrowserManager) -> Result<String, String> {
        let (_browser_type, browser_path) = browser_manager.get_available_browser()?;

        let user_data_dir = self.get_user_data_dir()?;
        if let Err(e) = fs::create_dir_all(&user_data_dir) {
            println!("[PW] Warning: Failed to create user_data_dir {:?}: {}", user_data_dir, e);
        }

        let playwright = Playwright::initialize()
            .await
            .map_err(|e| format!("Playwright init error: {}", e))?;

        let chromium = playwright.chromium();
        let mut launcher = chromium.persistent_context_launcher(&user_data_dir);

        // 配置浏览器
        launcher = launcher
            .executable(&browser_path)
            .headless(self.config.headless);

        if self.config.bypass_csp {
            launcher = launcher.bypass_csp(true);
        }

        if let Some(ref ua) = self.config.user_agent {
            launcher = launcher.user_agent(ua);
        }

        if let Some(ref proxy) = self.config.proxy_server {
            use playwright::api::ProxySettings;
            let proxy_settings = ProxySettings {
                server: proxy.clone(),
                bypass: None,
                username: None,
                password: None,
            };
            launcher = launcher.proxy(proxy_settings);
        }

        let context = launcher
            .launch()
            .await
            .map_err(|e| format!("Playwright launch error: {}", e))?;

        let page = context
            .new_page()
            .await
            .map_err(|e| format!("Playwright new_page error: {}", e))?;

        // 执行搜索流程
        let html = search_engine.perform_search(&page, query).await?;

        if html.trim().is_empty() {
            return Err("Empty HTML from search flow".to_string());
        }

        Ok(html)
    }

    /// 使用Playwright抓取内容
    async fn fetch_with_playwright(&self, url: &str, browser_manager: &BrowserManager) -> Result<String, String> {
        let (_browser_type, browser_path) = browser_manager.get_available_browser()?;

        let user_data_dir = self.get_user_data_dir()?;
        if let Err(e) = fs::create_dir_all(&user_data_dir) {
            println!("[PW] Warning: Failed to create user_data_dir {:?}: {}", user_data_dir, e);
        }

        let playwright = Playwright::initialize()
            .await
            .map_err(|e| format!("Playwright init error: {}", e))?;

        let chromium = playwright.chromium();
        let mut launcher = chromium.persistent_context_launcher(&user_data_dir);

        // 配置浏览器
        launcher = launcher
            .executable(&browser_path)
            .headless(self.config.headless);

        if self.config.bypass_csp {
            launcher = launcher.bypass_csp(true);
        }

        if let Some(ref ua) = self.config.user_agent {
            launcher = launcher.user_agent(ua);
        }

        if let Some(ref proxy) = self.config.proxy_server {
            use playwright::api::ProxySettings;
            let proxy_settings = ProxySettings {
                server: proxy.clone(),
                bypass: None,
                username: None,
                password: None,
            };
            launcher = launcher.proxy(proxy_settings);
        }

        let context = launcher
            .launch()
            .await
            .map_err(|e| format!("Playwright launch error: {}", e))?;

        let page = context
            .new_page()
            .await
            .map_err(|e| format!("Playwright new_page error: {}", e))?;

        page
            .goto_builder(url)
            .goto()
            .await
            .map_err(|e| format!("Playwright goto error: {}", e))?;

        // 等待页面加载完成
        self.wait_for_content(&page).await?;

        let html: String = page
            .eval("() => document.documentElement.outerHTML")
            .await
            .map_err(|e| format!("Playwright eval error: {}", e))?;

        if html.trim().is_empty() {
            return Err("Empty HTML from Playwright".to_string());
        }

        Ok(html)
    }

    /// 等待页面内容加载
    async fn wait_for_content(&self, page: &playwright::api::Page) -> Result<(), String> {
        if self.config.wait_selectors.is_empty() {
            page.wait_for_timeout(800.0).await;
            return Ok(());
        }

        let start = std::time::Instant::now();
        let selectors_json = serde_json::to_string(&self.config.wait_selectors)
            .unwrap_or("[]".to_string());
        
        let script = format!(
            "() => {{ const sels = {}; for (const s of sels) {{ if (document.querySelector(s)) return s; }} return null; }}",
            selectors_json
        );

        let mut matched: Option<String> = None;
        loop {
            let found: Option<String> = page
                .eval(&script)
                .await
                .map_err(|e| format!("Playwright wait eval error: {}", e))?;

            if let Some(sel) = found {
                matched = Some(sel);
                break;
            }

            if start.elapsed() >= Duration::from_millis(self.config.wait_timeout_ms) {
                break;
            }

            page.wait_for_timeout(self.config.wait_poll_ms as f64).await;
        }

        if let Some(sel) = matched {
            println!("[PW] Waited selector matched: {}", sel);
        } else {
            println!("[PW] Wait timeout after {} ms", self.config.wait_timeout_ms);
        }

        Ok(())
    }

    /// 使用系统浏览器headless模式抓取
    async fn fetch_with_headless_browser(&self, url: &str, browser_manager: &BrowserManager) -> Result<String, String> {
        let (browser_type, browser_path) = browser_manager.get_available_browser()?;
        println!("[HEADLESS] Using {} at {}", browser_type.as_str(), browser_path.display());

        let mut cmd = TokioCommand::new(browser_path);
        
        let user_agent = self.config.user_agent.as_deref().unwrap_or(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
        );

        cmd.arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-extensions")
            .arg("--disable-blink-features=AutomationControlled")
            .arg("--virtual-time-budget=15000")
            .arg("--timeout=45000")
            .arg("--hide-scrollbars")
            .arg("--window-size=1280,800")
            .arg("--dump-dom")
            .arg(format!("--user-agent={}", user_agent))
            .arg(url);

        let output = cmd.output().await
            .map_err(|e| format!("Failed to run headless browser: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Headless browser failed: {}", stderr.trim()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if stdout.trim().is_empty() {
            return Err("Empty DOM output from headless browser".to_string());
        }

        Ok(stdout)
    }

    /// 使用HTTP直接请求
    async fn fetch_with_http(&self, url: &str) -> Result<String, String> {
        let user_agent = self.config.user_agent.as_deref().unwrap_or(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
        );

        let mut client_builder = reqwest::Client::builder()
            .user_agent(user_agent)
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(Duration::from_secs(15));

        if let Some(ref proxy) = self.config.proxy_server {
            let proxy = reqwest::Proxy::all(proxy)
                .map_err(|e| format!("Invalid proxy configuration: {}", e))?;
            client_builder = client_builder.proxy(proxy);
        }

        let client = client_builder
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let resp = client
            .get(url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|e| format!("HTTP request error: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("HTTP status {} when fetching {}", status.as_u16(), url));
        }

        let text = resp.text().await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        if text.trim().is_empty() {
            return Err("Empty response body".to_string());
        }

        Ok(text)
    }

    /// WebView兜底导航（不提取内容）
    async fn fallback_webview_navigation(&self, url: &str) -> Result<String, String> {
        if let Err(e) = crate::window::ensure_hidden_search_window(self.app_handle.clone()).await {
            println!("[WEBVIEW] Failed to create hidden search window: {}", e);
        } else if let Some(window) = self.app_handle.get_webview_window("hidden_search") {
            let _ = window.navigate(url.parse().map_err(|e| format!("Invalid URL: {}", e))?);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        Err("All fetch strategies failed; WebView navigation attempted but no content extracted".to_string())
    }

    /// 获取用户数据目录
    fn get_user_data_dir(&self) -> Result<PathBuf, String> {
        if let Some(ref custom_dir) = self.config.user_data_dir {
            Ok(PathBuf::from(custom_dir))
        } else {
            let base = self.app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            Ok(base.join("playwright_profile"))
        }
    }
}