use super::browser::BrowserManager;
use super::engine_manager::SearchEngine;
use super::fingerprint::{FingerprintConfig, FingerprintManager, TimingConfig};
use playwright::Playwright;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::process::Command as TokioCommand;
use tokio::time::sleep;

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
    fingerprint_manager: FingerprintManager,
    timing_config: TimingConfig,
}

impl ContentFetcher {
    pub fn new(app_handle: AppHandle, config: FetchConfig) -> Self {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default().join("data"));

        let fingerprint_manager = FingerprintManager::new(&app_data_dir);
        let timing_config = FingerprintManager::get_timing_config();

        Self { app_handle, config, fingerprint_manager, timing_config }
    }

    /// 主要的内容抓取方法，按优先级尝试不同策略
    pub async fn fetch_content(
        &mut self,
        url: &str,
        browser_manager: &BrowserManager,
    ) -> Result<String, String> {
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
    pub async fn fetch_search_content(
        &mut self,
        query: &str,
        search_engine: &SearchEngine,
        browser_manager: &BrowserManager,
    ) -> Result<String, String> {
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
        Err(format!(
            "Search flow failed for {} engine: {}",
            search_engine.display_name(),
            "All interactive search attempts failed"
        ))
    }

    /// 使用Playwright执行搜索流程
    async fn fetch_search_with_playwright(
        &mut self,
        query: &str,
        search_engine: &SearchEngine,
        browser_manager: &BrowserManager,
    ) -> Result<String, String> {
        let (_browser_type, browser_path) = browser_manager.get_available_browser()?;

        let user_data_dir = self.get_user_data_dir()?;
        if let Err(e) = fs::create_dir_all(&user_data_dir) {
            println!("[PW] Warning: Failed to create user_data_dir {:?}: {}", user_data_dir, e);
        }

        let playwright =
            Playwright::initialize().await.map_err(|e| format!("Playwright init error: {}", e))?;

        let chromium = playwright.chromium();
        let mut launcher = chromium.persistent_context_launcher(&user_data_dir);

        // 获取稳定的指纹配置（通过单独作用域避免借用冲突）
        let (fingerprint, stealth_args) = {
            let fp = self.fingerprint_manager.get_stable_fingerprint(None).clone();
            let args = FingerprintManager::get_stealth_launch_args();
            (fp, args)
        };

        // 应用指纹配置
        launcher = self.fingerprint_manager.apply_fingerprint_to_context(launcher, &fingerprint);

        // 配置浏览器启动参数
        launcher =
            launcher.executable(&browser_path).headless(self.config.headless).args(&stealth_args);

        if self.config.bypass_csp {
            launcher = launcher.bypass_csp(true);
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

        let context =
            launcher.launch().await.map_err(|e| format!("Playwright launch error: {}", e))?;

        let page =
            context.new_page().await.map_err(|e| format!("Playwright new_page error: {}", e))?;

        // 注入反检测脚本
        self.inject_anti_detection_scripts(&page).await?;

        // 在页面级别设置额外的HTTP头（替代浏览器上下文级别的设置）
        self.set_page_http_headers(&page, &fingerprint).await?;

        // 执行搜索流程（使用人性化的延时）
        let html = self.perform_humanized_search(&page, query, search_engine).await?;

        if html.trim().is_empty() {
            return Err("Empty HTML from search flow".to_string());
        }

        Ok(html)
    }

    /// 使用Playwright抓取内容
    async fn fetch_with_playwright(
        &mut self,
        url: &str,
        browser_manager: &BrowserManager,
    ) -> Result<String, String> {
        let (_browser_type, browser_path) = browser_manager.get_available_browser()?;

        let user_data_dir = self.get_user_data_dir()?;
        if let Err(e) = fs::create_dir_all(&user_data_dir) {
            println!("[PW] Warning: Failed to create user_data_dir {:?}: {}", user_data_dir, e);
        }

        let playwright =
            Playwright::initialize().await.map_err(|e| format!("Playwright init error: {}", e))?;

        let chromium = playwright.chromium();
        let mut launcher = chromium.persistent_context_launcher(&user_data_dir);

        // 获取稳定的指纹配置（通过单独作用域避免借用冲突）
        let (fingerprint, stealth_args) = {
            let fp = self.fingerprint_manager.get_stable_fingerprint(None).clone();
            let args = FingerprintManager::get_stealth_launch_args();
            (fp, args)
        };

        // 应用指纹配置
        launcher = self.fingerprint_manager.apply_fingerprint_to_context(launcher, &fingerprint);

        // 配置浏览器启动参数
        launcher =
            launcher.executable(&browser_path).headless(self.config.headless).args(&stealth_args);

        if self.config.bypass_csp {
            launcher = launcher.bypass_csp(true);
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

        let context =
            launcher.launch().await.map_err(|e| format!("Playwright launch error: {}", e))?;

        let page =
            context.new_page().await.map_err(|e| format!("Playwright new_page error: {}", e))?;

        // 注入反检测脚本
        self.inject_anti_detection_scripts(&page).await?;

        // 在页面级别设置额外的HTTP头（替代浏览器上下文级别的设置）
        self.set_page_http_headers(&page, &fingerprint).await?;

        page.goto_builder(url).goto().await.map_err(|e| format!("Playwright goto error: {}", e))?;

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
        let selectors_json =
            serde_json::to_string(&self.config.wait_selectors).unwrap_or("[]".to_string());

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
    async fn fetch_with_headless_browser(
        &self,
        url: &str,
        browser_manager: &BrowserManager,
    ) -> Result<String, String> {
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

        let output =
            cmd.output().await.map_err(|e| format!("Failed to run headless browser: {}", e))?;

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

        let client =
            client_builder.build().map_err(|e| format!("Failed to build HTTP client: {}", e))?;

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

        let text = resp.text().await.map_err(|e| format!("Failed to read response body: {}", e))?;

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

        Err("All fetch strategies failed; WebView navigation attempted but no content extracted"
            .to_string())
    }

    /// 获取用户数据目录
    fn get_user_data_dir(&self) -> Result<PathBuf, String> {
        if let Some(ref custom_dir) = self.config.user_data_dir {
            Ok(PathBuf::from(custom_dir))
        } else {
            let base = self
                .app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            Ok(base.join("playwright_profile"))
        }
    }

    /// 注入反检测脚本
    async fn inject_anti_detection_scripts(
        &self,
        page: &playwright::api::Page,
    ) -> Result<(), String> {
        // 移除webdriver标识
        let anti_detection_script = r#"
            // 移除webdriver属性
            Object.defineProperty(navigator, 'webdriver', {
                get: () => false,
            });

            // 覆盖Chrome对象
            window.chrome = {
                runtime: {},
                loadTimes: function() {
                    return {
                        commitLoadTime: Date.now() / 1000 - Math.random() * 100,
                        finishDocumentLoadTime: Date.now() / 1000 - Math.random() * 50,
                        finishLoadTime: Date.now() / 1000 - Math.random() * 20,
                        firstPaintAfterLoadTime: 0,
                        firstPaintTime: Date.now() / 1000 - Math.random() * 30,
                        navigationType: "Other",
                        npnNegotiatedProtocol: "http/1.1",
                        requestTime: Date.now() / 1000 - Math.random() * 200,
                        startLoadTime: Date.now() / 1000 - Math.random() * 300,
                        connectionInfo: "http/1.1",
                        wasFetchedViaSpdy: false,
                        wasNpnNegotiated: false
                    };
                },
                csi: function() {
                    return {
                        startE: Date.now() - Math.random() * 1000,
                        onloadT: Date.now() - Math.random() * 500,
                        pageT: Date.now() - Math.random() * 300,
                        tran: Math.floor(Math.random() * 20)
                    };
                }
            };

            // 模拟真实的插件信息
            Object.defineProperty(navigator, 'plugins', {
                get: () => [
                    {
                        0: {type: "application/x-google-chrome-pdf", suffixes: "pdf", description: "Portable Document Format"},
                        description: "Portable Document Format",
                        filename: "internal-pdf-viewer",
                        length: 1,
                        name: "Chrome PDF Plugin"
                    },
                    {
                        0: {type: "application/pdf", suffixes: "pdf", description: "Portable Document Format"},
                        description: "Portable Document Format", 
                        filename: "mhjfbmdgcfjbbpaeojofohoefgiehjai",
                        length: 1,
                        name: "Chrome PDF Viewer"
                    }
                ]
            });

            // 覆盖权限查询
            const originalQuery = window.navigator.permissions.query;
            window.navigator.permissions.query = (parameters) => (
                parameters.name === 'notifications' ?
                Promise.resolve({ state: Notification.permission }) :
                originalQuery(parameters)
            );

            // 添加一些随机的性能噪音
            const originalGetEntriesByType = performance.getEntriesByType;
            performance.getEntriesByType = function(type) {
                const entries = originalGetEntriesByType.call(this, type);
                return entries.map(entry => ({
                    ...entry,
                    startTime: entry.startTime + Math.random() * 2 - 1,
                    duration: entry.duration + Math.random() * 0.5 - 0.25
                }));
            };
        "#;

        page.add_init_script(anti_detection_script)
            .await
            .map_err(|e| format!("Failed to inject anti-detection script: {}", e))?;

        Ok(())
    }

    /// 在页面级别设置HTTP头
    async fn set_page_http_headers(
        &self,
        page: &playwright::api::Page,
        config: &FingerprintConfig,
    ) -> Result<(), String> {
        use std::collections::HashMap;

        let mut headers = HashMap::new();
        headers.insert("Accept-Language".to_string(), config.accept_language.clone());
        headers.insert("Sec-Ch-Ua-Platform".to_string(), format!("\"{}\"", config.platform));
        headers.insert(
            "Sec-Ch-Ua-Mobile".to_string(),
            if config.is_mobile { "?1" } else { "?0" }.to_string(),
        );
        headers.insert(
            "Sec-Ch-Ua".to_string(),
            "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"131\", \"Chromium\";v=\"131\""
                .to_string(),
        );

        page.set_extra_http_headers(headers)
            .await
            .map_err(|e| format!("Failed to set extra HTTP headers: {}", e))?;

        Ok(())
    }

    /// 执行人性化的搜索流程
    async fn perform_humanized_search(
        &self,
        page: &playwright::api::Page,
        query: &str,
        search_engine: &SearchEngine,
    ) -> Result<String, String> {
        println!("[SEARCH][HUMANIZED] Starting humanized search for: {}", query);

        // 随机延时模拟网络延迟
        let initial_delay = self.timing_config.action_delay_min
            + fastrand::u64(
                0..self.timing_config.action_delay_max - self.timing_config.action_delay_min,
            );
        sleep(Duration::from_millis(initial_delay)).await;

        // 带重试的导航到搜索引擎首页
        let homepage_url = search_engine.homepage_url();
        self.navigate_with_retry(page, homepage_url).await?;

        // 等待页面稳定
        sleep(Duration::from_millis(500 + fastrand::u64(0..500))).await;

        // 人性化的输入框定位和填写
        self.humanized_search_input(page, query, search_engine).await?;

        // 人性化的搜索触发
        self.humanized_search_submit(page, search_engine).await?;

        // 等待结果加载，带随机延时
        let wait_time = self.timing_config.page_load_timeout + fastrand::u64(0..2000);
        self.wait_for_results_with_timeout(page, wait_time, search_engine).await?;

        // 增强的HTML提取，带重试机制
        let html = self.extract_page_html_with_retry(page).await?;

        println!("[SEARCH][HUMANIZED] Successfully retrieved {} bytes", html.len());
        Ok(html)
    }

    /// 带重试机制的HTML提取
    async fn extract_page_html_with_retry(
        &self,
        page: &playwright::api::Page,
    ) -> Result<String, String> {
        let max_retries = 3;
        let mut last_error = String::new();

        for attempt in 1..=max_retries {
            println!(
                "[SEARCH][HUMANIZED] Attempting HTML extraction (attempt {}/{})",
                attempt, max_retries
            );

            // 等待页面稳定
            sleep(Duration::from_millis(1000 + fastrand::u64(0..1000))).await;

            // 检查页面是否准备就绪
            match self.check_page_ready(page).await {
                Ok(true) => {
                    // 页面准备就绪，尝试提取HTML
                    match page.eval("() => document.documentElement.outerHTML").await {
                        Ok(html) => {
                            let html_str: String = html;
                            if html_str.len() > 1000 {
                                // 确保HTML内容足够丰富
                                println!(
                                    "[SEARCH][HUMANIZED] HTML extraction successful on attempt {}",
                                    attempt
                                );
                                return Ok(html_str);
                            } else {
                                last_error = format!("HTML too short ({} bytes)", html_str.len());
                                println!("[SEARCH][HUMANIZED] HTML too short, retrying...");
                            }
                        }
                        Err(e) => {
                            last_error = format!("HTML extraction error: {}", e);
                            println!("[SEARCH][HUMANIZED] HTML extraction failed: {}", e);
                        }
                    }
                }
                Ok(false) => {
                    last_error = "Page not ready".to_string();
                    println!("[SEARCH][HUMANIZED] Page not ready, waiting...");
                }
                Err(e) => {
                    last_error = format!("Page check error: {}", e);
                    println!("[SEARCH][HUMANIZED] Page check error: {}", e);
                }
            }

            // 在重试之间等待
            if attempt < max_retries {
                sleep(Duration::from_millis(2000)).await;
            }
        }

        Err(format!(
            "Failed to extract HTML after {} attempts. Last error: {}",
            max_retries, last_error
        ))
    }

    /// 检查页面是否准备就绪
    async fn check_page_ready(&self, page: &playwright::api::Page) -> Result<bool, String> {
        // 检查document是否存在
        // let doc_ready: bool = page
        //     .eval("() => !!document && document.readyState === 'complete'")
        //     .await
        //     .unwrap_or(false);

        // if !doc_ready {
        //     return Ok(false);
        // }

        // 检查body是否存在且有内容
        let body_ready: bool = page
            .eval("() => !!document.body && document.body.children.length > 0")
            .await
            .unwrap_or(false);

        if !body_ready {
            return Ok(false);
        }

        // 检查是否存在任何搜索结果标识
        let has_content: bool = page
            .eval(
                "() => {
                const indicators = [
                    '#b_content', '#b_results', '.b_algo', // Bing
                    '#search', '#main', '.g', '.tF2Cxc', // Google
                    '#results', '.result', '.web-result' // 通用
                ];
                return indicators.some(sel => document.querySelector(sel));
            }",
            )
            .await
            .unwrap_or(false);

        Ok(has_content)
    }

    /// 带重试机制的页面导航
    async fn navigate_with_retry(
        &self,
        page: &playwright::api::Page,
        url: &str,
    ) -> Result<(), String> {
        let max_retries = 3;
        let mut last_error = String::new();

        for attempt in 1..=max_retries {
            println!(
                "[SEARCH][HUMANIZED] Attempting navigation to {} (attempt {}/{})",
                url, attempt, max_retries
            );

            match page.goto_builder(url).goto().await {
                Ok(_) => {
                    println!("[SEARCH][HUMANIZED] Navigation successful on attempt {}", attempt);

                    // 验证页面是否实际加载成功
                    sleep(Duration::from_millis(1000)).await;

                    let page_loaded: bool = page
                        .eval("() => document.readyState === 'complete' && !!document.body")
                        .await
                        .unwrap_or(false);

                    if page_loaded {
                        return Ok(());
                    } else {
                        last_error = "Page did not load completely".to_string();
                        println!("[SEARCH][HUMANIZED] Page not fully loaded, retrying...");
                    }
                }
                Err(e) => {
                    last_error = format!("Navigation error: {}", e);
                    println!("[SEARCH][HUMANIZED] Navigation failed: {}", e);

                    // 对于特定的错误，我们可以尝试不同的策略
                    if e.to_string().contains("ERR_CONNECTION_CLOSED")
                        || e.to_string().contains("ERR_NETWORK_CHANGED")
                    {
                        println!("[SEARCH][HUMANIZED] Network connection issue detected, waiting longer before retry...");
                        sleep(Duration::from_millis(5000)).await;
                    }
                }
            }

            // 在重试之间等待
            if attempt < max_retries {
                let wait_time = 2000 * attempt as u64; // 递增等待时间
                sleep(Duration::from_millis(wait_time)).await;
            }
        }

        Err(format!(
            "Failed to navigate to {} after {} attempts. Last error: {}",
            url, max_retries, last_error
        ))
    }

    /// 人性化的搜索输入处理
    async fn humanized_search_input(
        &self,
        page: &playwright::api::Page,
        query: &str,
        search_engine: &SearchEngine,
    ) -> Result<(), String> {
        // 内部辅助函数：构造激活脚本（避免重复 & 方便测试）
        fn build_activation_script(selector: &str) -> String {
            format!(
                r#"() => {{
    const el = document.querySelector('{sel}');
    if (!el) return {{ success: false, method: 'not_found' }};
    try {{
        // 清空（避免历史值影响）
        if ('value' in el) el.value = '';
        el.focus();
        try {{ el.click(); }} catch(_e) {{}}
        return {{ success: true, method: 'activated' }};
    }} catch (e) {{
        return {{ success: false, method: 'exception', error: String(e), stack: (e && e.stack) ? String(e.stack) : 'no_stack' }};
    }}
}}"#,
                sel = selector.replace("'", "\\'")
            )
        }

        // 内部辅助函数：构造输入脚本
        fn build_input_script(selector: &str, value: &str) -> String {
            format!(
                r#"() => {{
    const el = document.querySelector('{sel}');
    if (!el) return {{ success: false, reason: 'element_not_found' }};
    try {{
        el.focus();
        const v = '{val}';
        if ('value' in el) el.value = v;
        if ('textContent' in el) el.textContent = v;
        // 触发基础事件（使用单层花括号，防止格式化错误）
        el.dispatchEvent(new Event('input', {{ bubbles: true }}));
        el.dispatchEvent(new Event('change', {{ bubbles: true }}));
        return {{ success: true, value: ('value' in el ? el.value : (el.textContent||'')) }};
    }} catch(e) {{
        return {{ success: false, reason: String(e) }};
    }}
}}"#,
                sel = selector.replace("'", "\\'"),
                val = value.replace("'", "\\'").replace('"', "\\\"")
            )
        }

        // 优先尝试关闭可能阻挡输入框的 Consent / Cookie 弹窗（特别是 Google）
        // 这些弹窗会导致 querySelector 找不到真正可见的输入框或输入失败
        let consent_dismiss_scripts = [
            // Google 同意弹窗按钮："同意" / "接受全部" / "全部接受" / 英文 "Accept all" / "I agree"
            "() => { const btns = Array.from(document.querySelectorAll('button, div[role=button]')); const patterns = [/同意/, /接受全部/, /全部接受/, /Allow all/i, /Accept all/i, /I agree/i]; for (const b of btns){ const t=b.textContent||''; if(patterns.some(p=>p.test(t))){ b.click(); return true; } } return false; }",
            // Google "拒绝全部" 也可关闭遮罩
            "() => { const btns = Array.from(document.querySelectorAll('button, div[role=button]')); const patterns = [/拒绝/, /拒絕/, /Reject all/i, /Decline/i]; for (const b of btns){ const t=b.textContent||''; if(patterns.some(p=>p.test(t))){ b.click(); return true; } } return false; }",
            // 直接点击同意框内的第一个可点击按钮（退而求其次）
            r#"() => { const dlg = document.querySelector('form[action*=consent], div[role=dialog]'); if(!dlg) return false; const btn = dlg.querySelector('button, div[role=button], input[type=submit]'); if(btn){ btn.click(); return true; } return false; }"#,
        ];
        println!("[SEARCH][DEBUG] Checking for consent/cookie dialogs...");

        // 等待页面完全加载和JavaScript执行
        sleep(Duration::from_millis(1500)).await;

        for (idx, script) in consent_dismiss_scripts.iter().enumerate() {
            let dismissed: bool = page.eval(script).await.unwrap_or(false);
            println!("[SEARCH][DEBUG] Consent script {}: dismissed={}", idx + 1, dismissed);
            if dismissed {
                println!("[SEARCH][HUMANIZED] Dismissed a consent/cookie dialog");
                sleep(Duration::from_millis(500)).await;
                break;
            }
        }

        let selectors = search_engine.search_input_selectors();
        println!("[SEARCH][DEBUG] Trying {} selectors for search input", selectors.len());
        println!("[SEARCH][DEBUG] Selectors: {:?}", selectors);

        for (idx, selector) in selectors.iter().enumerate() {
            println!(
                "[SEARCH][DEBUG] Trying selector {}/{}: {}",
                idx + 1,
                selectors.len(),
                selector
            );
            // 检查元素是否存在和可见
            let element_info = page
                .eval(&format!(
                    "() => {{
                        const el = document.querySelector('{}');
                        if (!el) return {{ exists: false, visible: false, disabled: false }};
                        return {{
                            exists: true,
                            visible: el.offsetParent !== null,
                            disabled: el.disabled || false,
                            tagName: el.tagName,
                            type: el.type || 'none',
                            name: el.name || 'none'
                        }};
                    }}",
                    selector.replace("'", "\\'")
                ))
                .await;

            match element_info {
                Ok(info) => {
                    println!("[SEARCH][DEBUG] Element info for '{}': {:?}", selector, info);
                    let info_obj: serde_json::Value = info;
                    let exists = info_obj["exists"].as_bool().unwrap_or(false);
                    let visible = info_obj["visible"].as_bool().unwrap_or(false);
                    let disabled = info_obj["disabled"].as_bool().unwrap_or(true);

                    if !exists {
                        println!("[SEARCH][DEBUG] Element not found, trying next selector");
                        continue;
                    }
                    if !visible {
                        println!("[SEARCH][DEBUG] Element not visible, trying next selector");
                        continue;
                    }
                    if disabled {
                        println!("[SEARCH][DEBUG] Element disabled, trying next selector");
                        continue;
                    }
                }
                Err(e) => {
                    println!("[SEARCH][DEBUG] Failed to check element '{}': {}", selector, e);
                    continue;
                }
            }

            println!(
                "[SEARCH][DEBUG] Found valid element, attempting to interact with: {}",
                selector
            );
            // 随机延时模拟真实用户行为
            sleep(Duration::from_millis(100 + fastrand::u64(0..200))).await;

            // 使用新的脚本构造器（避免语法错误）
            let activation_script = build_activation_script(selector);
            println!("[SEARCH][TRACE] Activation script: {}", activation_script);
            let result: serde_json::Value = match page.eval(&activation_script).await {
                Ok(v) => v,
                Err(e) => {
                    println!(
                        "[SEARCH][DEBUG] Activation eval error for selector '{}': {}",
                        selector, e
                    );
                    serde_json::json!({"success": false, "method": "eval_error", "error": e.to_string()})
                }
            };

            let success = result["success"].as_bool().unwrap_or(false);
            println!("[SEARCH][DEBUG] Activation result for '{}': {:?}", selector, result);

            if !success {
                println!("[SEARCH][DEBUG] Failed to activate element (result: {:?}), trying next selector", result);
                continue;
            }

            // 延时后开始输入
            sleep(Duration::from_millis(150 + fastrand::u64(0..200))).await;

            let input_script = build_input_script(selector, query);
            println!("[SEARCH][TRACE] Input script: {}", input_script);
            let input_result: serde_json::Value = match page.eval(&input_script).await {
                Ok(v) => v,
                Err(e) => {
                    println!("[SEARCH][DEBUG] Input eval error for selector '{}': {}", selector, e);
                    serde_json::json!({"success": false, "reason": "eval_error", "error": e.to_string()})
                }
            };

            let input_success = input_result["success"].as_bool().unwrap_or(false);
            println!("[SEARCH][DEBUG] Input result for '{}': {:?}", selector, input_result);

            if input_success {
                println!("[SEARCH][HUMANIZED] Successfully filled search input: {}", selector);
                return Ok(());
            } else {
                println!("[SEARCH][DEBUG] Input failed for '{}', trying next selector", selector);
                continue;
            }
        }

        // Fallback broad candidate strategy before dumping diagnostics
        println!("[SEARCH][DEBUG] All direct selectors failed, attempting fallback candidate strategy...");
        let fallback_script = format!(
            r#"() => {{
                    const candSelectors = [
                        'textarea[name=\"q\"]','input[name=\"q\"]','textarea.gLFyf','input.gLFyf','#APjFqb',
                        'form[role=\"search\"] textarea','form[role=\"search\"] input[type=\"text\"]','form[role=\"search\"] input[type=\"search\"]'
                    ];
                    const cands = candSelectors.flatMap(sel => Array.from(document.querySelectorAll(sel)));
                    const dedup = Array.from(new Set(cands));
                    const visible = dedup.filter(el => el && el.offsetParent !== null && !el.disabled);
                    const target = visible[0] || dedup[0];
                    if(!target) return {{ success:false, stage:'fallback', reason:'no_candidates' }};
                    try {{ target.focus(); }} catch(e) {{}}
                    try {{ target.click(); }} catch(e) {{}}
                    try {{ if('value' in target) target.value = '{val}'; }} catch(e) {{}}
                    try {{ target.dispatchEvent(new Event('input', {{ bubbles:true }})); }} catch(e) {{}}
                    try {{ target.dispatchEvent(new Event('change', {{ bubbles:true }})); }} catch(e) {{}}
                    return {{ success:true, stage:'fallback', tag: target.tagName, id: target.id||'', name: target.name||'', className: target.className||'', value: target.value || target.textContent || '' }};
                }}"#,
            val = query.replace("'", "\\'").replace('"', "\\\"")
        );
        println!("[SEARCH][TRACE] Fallback fill script: {}", fallback_script);
        let fb_res: serde_json::Value = page.eval(&fallback_script).await.unwrap_or_else(
            |e| serde_json::json!({"success":false, "stage":"fallback", "error": e.to_string()}),
        );
        println!("[SEARCH][DEBUG] Fallback fill result: {:?}", fb_res);
        if fb_res["success"].as_bool().unwrap_or(false) {
            println!("[SEARCH][HUMANIZED] Fallback candidate strategy succeeded");
            return Ok(());
        }

        println!("[SEARCH][DEBUG] All selectors (including fallback) failed, dumping page info...");

        // 输出页面基本信息
        let page_info: serde_json::Value = page
            .eval(
                "() => ({ 
            url: window.location.href, 
            title: document.title,
            readyState: document.readyState,
            bodyExists: !!document.body,
            inputCount: document.querySelectorAll('input').length,
            textareaCount: document.querySelectorAll('textarea').length,
            formCount: document.querySelectorAll('form').length
        })",
            )
            .await
            .unwrap_or_default();
        println!("[SEARCH][DEBUG] Page info: {:?}", page_info);

        // 查找所有可能的输入框
        let input_elements: serde_json::Value = page
            .eval(
                "() => {
            const inputs = Array.from(document.querySelectorAll('input, textarea'));
            return inputs.slice(0, 10).map(el => ({
                tagName: el.tagName,
                type: el.type || 'none',
                name: el.name || 'none',
                id: el.id || 'none',
                className: el.className || 'none',
                placeholder: el.placeholder || 'none',
                visible: el.offsetParent !== null,
                disabled: el.disabled
            }));
        }",
            )
            .await
            .unwrap_or_default();
        println!("[SEARCH][DEBUG] Found input elements: {:?}", input_elements);

        Err("Could not find or fill any search input".to_string())
    }

    /// 人性化的搜索提交
    async fn humanized_search_submit(
        &self,
        page: &playwright::api::Page,
        search_engine: &SearchEngine,
    ) -> Result<(), String> {
        // 短暂延时，模拟用户思考
        sleep(Duration::from_millis(300 + fastrand::u64(0..700))).await;

        // 尝试点击搜索按钮
        let button_selectors = search_engine.search_button_selectors();
        for selector in button_selectors {
            let button_script = format!(
                "() => {{
                    const btn = document.querySelector('{}');
                    if (btn && btn.offsetParent !== null && !btn.disabled) {{
                        btn.click();
                        return true;
                    }}
                    return false;
                }}",
                selector.replace("'", "\\'")
            );

            let clicked: bool = page.eval(&button_script).await.unwrap_or(false);
            if clicked {
                println!("[SEARCH][HUMANIZED] Successfully clicked search button: {}", selector);
                return Ok(());
            }
        }

        // 如果按钮点击失败，尝试按Enter键
        let input_selectors = search_engine.search_input_selectors();
        for selector in input_selectors {
            let enter_script = format!(
                r#"() => {{
      const el = document.querySelector('{sel}');
      if(!el) return false;
      const evt = new KeyboardEvent('keydown', {{ key:'Enter', code:'Enter', keyCode:13, which:13, bubbles:true }});
      el.dispatchEvent(evt);
      return true;
    }}"#,
                sel = selector.replace("'", "\\'")
            );

            let pressed: bool = page.eval(&enter_script).await.unwrap_or(false);
            if pressed {
                println!("[SEARCH][HUMANIZED] Successfully pressed Enter on input: {}", selector);
                return Ok(());
            }
        }

        Err("Failed to submit search".to_string())
    }

    /// 等待搜索结果，带超时处理
    async fn wait_for_results_with_timeout(
        &self,
        page: &playwright::api::Page,
        timeout_ms: u64,
        search_engine: &SearchEngine,
    ) -> Result<(), String> {
        let start = tokio::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        let selectors = search_engine.default_wait_selectors();
        let selectors_json = serde_json::to_string(&selectors).unwrap_or("[]".to_string());

        loop {
            // 检查是否有任何结果选择器匹配
            let found_selector_script = format!(
                "() => {{ const sels = {}; for (const s of sels) {{ if (document.querySelector(s)) return s; }} return null; }}",
                selectors_json
            );

            let found: Option<String> = page.eval(&found_selector_script).await.unwrap_or(None);

            if let Some(sel) = found {
                println!("[SEARCH][HUMANIZED] Results loaded, found selector: {}", sel);
                // 额外等待一点时间确保内容完全渲染
                sleep(Duration::from_millis(500 + fastrand::u64(0..500))).await;
                return Ok(());
            }

            if start.elapsed() >= timeout {
                println!("[SEARCH][HUMANIZED] Results wait timeout, continuing anyway");
                break;
            }

            sleep(Duration::from_millis(250)).await;
        }

        Ok(())
    }
}
