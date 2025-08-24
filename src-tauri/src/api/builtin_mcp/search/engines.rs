use serde::{Deserialize, Serialize};
use playwright::api::Page;
use tokio::time::{sleep, Duration};
use std::time::Instant;

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

    /// 获取搜索引擎的基础URL - 已废弃，仅保留用于兼容性
    #[deprecated(note = "直接URL访问容易被拦截，请使用 homepage_url() 方法")]
    pub fn base_url(&self) -> &'static str {
        match self {
            SearchEngine::Google => "https://www.google.com/search",
            SearchEngine::Bing => "https://www.bing.com/search",
            SearchEngine::DuckDuckGo => "https://duckduckgo.com/html/",
            SearchEngine::Kagi => "https://kagi.com/search",
        }
    }

    /// 构建搜索URL - 已废弃，仅保留用于兼容性
    #[deprecated(note = "直接URL访问容易被拦截，请使用 perform_search() 方法")]
    pub fn build_search_url(&self, query: &str) -> String {
        let encoded_query = urlencoding::encode(query);
        match self {
            SearchEngine::Google => format!("{}?q={}", "https://www.google.com/search", encoded_query),
            SearchEngine::Bing => format!("{}?q={}", "https://www.bing.com/search", encoded_query),
            SearchEngine::DuckDuckGo => format!("{}?q={}", "https://duckduckgo.com/html/", encoded_query),
            SearchEngine::Kagi => format!("{}?q={}", "https://kagi.com/search", encoded_query),
        }
    }

    /// 获取默认的等待选择器
    pub fn default_wait_selectors(&self) -> Vec<String> {
        match self {
            SearchEngine::Google => vec![
                "#search".to_string(),
                "#main".to_string(),
                "#rcnt".to_string(),
                "#center_col".to_string(),
            ],
            SearchEngine::Bing => vec![
                "#b_content".to_string(),
                "#b_content > main".to_string(),
                ".b_algo".to_string(),
                "#b_results".to_string(),
            ],
            SearchEngine::DuckDuckGo => vec![
                "#links".to_string(),
                ".results".to_string(),
                ".result".to_string(),
                "#web_content".to_string(),
            ],
            SearchEngine::Kagi => vec![
                "#search-content".to_string(),
                ".search-result".to_string(),
                "#main".to_string(),
                ".search-container".to_string(),
            ],
        }
    }

    /// 获取搜索引擎的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            SearchEngine::Google => "Google",
            SearchEngine::Bing => "Bing",
            SearchEngine::DuckDuckGo => "DuckDuckGo",
            SearchEngine::Kagi => "Kagi",
        }
    }

    /// 获取搜索引擎的首页URL
    pub fn homepage_url(&self) -> &'static str {
        match self {
            SearchEngine::Google => "https://www.google.com",
            SearchEngine::Bing => "https://www.bing.com",
            SearchEngine::DuckDuckGo => "https://duckduckgo.com",
            SearchEngine::Kagi => "https://kagi.com",
        }
    }

    /// 获取搜索框选择器（优先级从高到低）
    pub fn search_input_selectors(&self) -> Vec<&'static str> {
        match self {
            SearchEngine::Google => vec![
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
            ],
            SearchEngine::Bing => vec![
                "#sb_form_q",
                "input[name='q']",
                "textarea[name='q']",
                ".b_searchbox",
                "#searchboxinput",
                "input[placeholder*='搜索']",
                "input[placeholder*='Search']",
            ],
            SearchEngine::DuckDuckGo => vec![
                "#search_form_input",
                "input[name='q']",
                "#searchbox_input", 
                ".js-search-input",
                "input[placeholder*='搜索']",
                "input[placeholder*='Search']",
            ],
            SearchEngine::Kagi => vec![
                "input[name='q']",
                "#searchInput",
                ".search-input",
                "input[placeholder*='搜索']",
                "input[placeholder*='Search']",
            ],
        }
    }

    /// 获取搜索按钮选择器（优先级从高到低）
    pub fn search_button_selectors(&self) -> Vec<&'static str> {
        match self {
            SearchEngine::Google => vec![
                "input[name='btnK']",
                "button[type='submit']",
                ".FPdoLc input[name='btnK']",
                ".tfB0Bf input[name='btnK']",
            ],
            SearchEngine::Bing => vec![
                "#search_icon",
                "input[type='submit']",
                ".b_searchboxSubmit",
                "#sb_form_go",
            ],
            SearchEngine::DuckDuckGo => vec![
                "input[type='submit']",
                "#search_button_homepage",
                ".search-wrap__button",
            ],
            SearchEngine::Kagi => vec![
                "button[type='submit']",
                ".search-button",
                "input[type='submit']",
            ],
        }
    }

    /// 执行完整的搜索流程
    pub async fn perform_search(&self, page: &Page, query: &str) -> Result<String, String> {
        println!(
            "[SEARCH][{}] Starting search flow for query: {}", 
            self.display_name(), 
            query
        );

        // 步骤1: 导航到首页
        let homepage = self.homepage_url();
        println!(
            "[SEARCH][{}] Navigating to homepage: {}", 
            self.display_name(), 
            homepage
        );
        
        page.goto_builder(homepage)
            .goto()
            .await
            .map_err(|e| format!("Failed to navigate to {}: {}", homepage, e))?;
        
        // 等待页面初始加载 - 增加等待时间
        sleep(Duration::from_millis(2500)).await;
        
        // 等待搜索框出现
        self.wait_for_search_input(page).await?;

        // 步骤2: 查找并填充搜索框
        let input_found = self.find_and_fill_search_input(page, query).await?;
        if !input_found {
            return Err(format!(
                "Could not find search input for {} after trying all selectors", 
                self.display_name()
            ));
        }

        // 步骤3: 点击搜索按钮或按Enter
        self.trigger_search(page).await?;
        
        // 步骤4: 等待搜索结果加载
        self.wait_for_search_results(page).await?;

        // 步骤5: 获取页面HTML
        let html: String = page
            .eval("() => document.documentElement.outerHTML")
            .await
            .map_err(|e| format!("Failed to extract HTML: {}", e))?;

        if html.trim().is_empty() {
            return Err("Retrieved HTML is empty".to_string());
        }

        println!(
            "[SEARCH][{}] Successfully completed search, HTML size: {} bytes", 
            self.display_name(), 
            html.len()
        );

        Ok(html)
    }

    /// 等待搜索框出现
    async fn wait_for_search_input(&self, page: &Page) -> Result<(), String> {
        let selectors = self.search_input_selectors();
        let start = Instant::now();
        let timeout = Duration::from_millis(10000); // 10秒超时
        
        loop {
            for selector in &selectors {
                let script = format!(
                    "() => {{
                        const element = document.querySelector('{}');
                        return element && element.offsetParent !== null;
                    }}",
                    selector.replace("'", "\\'")
                );
                
                let is_visible: bool = page
                    .eval(&script)
                    .await
                    .unwrap_or(false);
                
                if is_visible {
                    println!(
                        "[SEARCH][{}] Search input found: {}", 
                        self.display_name(), 
                        selector
                    );
                    return Ok(());
                }
            }
            
            if start.elapsed() >= timeout {
                println!(
                    "[SEARCH][{}] Search input wait timeout after {} ms", 
                    self.display_name(), 
                    timeout.as_millis()
                );
                return Ok(()); // 不要失败，继续尝试
            }
            
            sleep(Duration::from_millis(500)).await;
        }
    }

    /// 查找并填充搜索输入框
    async fn find_and_fill_search_input(&self, page: &Page, query: &str) -> Result<bool, String> {
        let selectors = self.search_input_selectors();
        
        for selector in selectors {
            println!(
                "[SEARCH][{}] Trying input selector: {}", 
                self.display_name(), 
                selector
            );
            
            // 检查元素是否存在和可见
            let script = format!(
                "() => {{
                    const element = document.querySelector('{}');
                    return element && element.offsetParent !== null;
                }}",
                selector.replace("'", "\\'") // 转义单引号
            );
            
            let is_visible: bool = page
                .eval(&script)
                .await
                .unwrap_or(false);
            
            if !is_visible {
                continue;
            }

            // 尝试填充输入框
            match self.fill_search_input(page, selector, query).await {
                Ok(_) => {
                    println!(
                        "[SEARCH][{}] Successfully filled input with selector: {}", 
                        self.display_name(), 
                        selector
                    );
                    return Ok(true);
                },
                Err(e) => {
                    println!(
                        "[SEARCH][{}] Failed to fill input with selector {}: {}", 
                        self.display_name(), 
                        selector, 
                        e
                    );
                    continue;
                }
            }
        }
        
        Ok(false)
    }

    /// 填充搜索输入框
    async fn fill_search_input(&self, page: &Page, selector: &str, query: &str) -> Result<(), String> {
        // 点击输入框以激活
        let click_script = format!(
            "() => {{
                const element = document.querySelector('{}');
                if (element) {{
                    element.focus();
                    element.click();
                    return true;
                }}
                return false;
            }}",
            selector.replace("'", "\\'") // 转义单引号
        );
        
        let clicked: bool = page
            .eval(&click_script)
            .await
            .map_err(|e| format!("Failed to click input: {}", e))?;
            
        if !clicked {
            return Err("Failed to click search input".to_string());
        }
        
        // 短暂延时模拟人工操作
        sleep(Duration::from_millis(300)).await;
        
        // 清空输入框
        let clear_script = format!(
            "() => {{
                const element = document.querySelector('{}');
                if (element) {{
                    element.value = '';
                    element.dispatchEvent(new Event('input', {{ bubbles: true }}));
                }}
            }}",
            selector.replace("'", "\\'") // 转义单引号
        );
        
        page.eval::<()>(&clear_script)
            .await
            .map_err(|e| format!("Failed to clear input: {}", e))?;
        
        // 模拟逐字符输入
        for ch in query.chars() {
            let char_script = format!(
                "() => {{
                    const element = document.querySelector('{}');
                    if (element) {{
                        element.value += '{}';
                        element.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        element.dispatchEvent(new Event('keyup', {{ bubbles: true }}));
                    }}
                }}",
                selector.replace("'", "\\'"), // 转义单引号
                ch.to_string().replace("'", "\\'") // 转义单引号
            );
            
            page.eval::<()>(&char_script)
                .await
                .map_err(|e| format!("Failed to input character: {}", e))?;
            
            // 随机延时模拟人工输入
            let delay = 50 + (rand::random::<u64>() % 100);
            sleep(Duration::from_millis(delay)).await;
        }
        
        // 触发最终的输入事件
        let final_script = format!(
            "() => {{
                const element = document.querySelector('{}');
                if (element) {{
                    element.dispatchEvent(new Event('change', {{ bubbles: true }}));
                }}
            }}",
            selector.replace("'", "\\'") // 转义单引号
        );
        
        page.eval::<()>(&final_script)
            .await
            .map_err(|e| format!("Failed to trigger change event: {}", e))?;
        
        Ok(())
    }

    /// 触发搜索（点击按钮或按Enter）
    async fn trigger_search(&self, page: &Page) -> Result<(), String> {
        // 方案1: 尝试点击搜索按钮
        let button_selectors = self.search_button_selectors();
        
        for selector in button_selectors {
            println!(
                "[SEARCH][{}] Trying search button selector: {}", 
                self.display_name(), 
                selector
            );
            
            let button_script = format!(
                "() => {{
                    const button = document.querySelector('{}');
                    if (button && button.offsetParent !== null) {{
                        button.click();
                        return true;
                    }}
                    return false;
                }}",
                selector.replace("'", "\\'") // 转义单引号
            );
            
            let clicked: bool = page
                .eval(&button_script)
                .await
                .unwrap_or(false);
            
            if clicked {
                println!(
                    "[SEARCH][{}] Successfully clicked search button: {}", 
                    self.display_name(), 
                    selector
                );
                return Ok(());
            }
        }
        
        // 方案2: 如果按钮点击失败，尝试按Enter键
        println!("[SEARCH][{}] Button click failed, trying Enter key", self.display_name());
        
        let input_selectors = self.search_input_selectors();
        for selector in input_selectors {
            let enter_script = format!(
                "() => {{
                    const input = document.querySelector('{}');
                    if (input) {{
                        const event = new KeyboardEvent('keydown', {{
                            key: 'Enter',
                            code: 'Enter',
                            keyCode: 13,
                            bubbles: true
                        }});
                        input.dispatchEvent(event);
                        return true;
                    }}
                    return false;
                }}",
                selector.replace("'", "\\'") // 转义单引号
            );
            
            let pressed: bool = page
                .eval(&enter_script)
                .await
                .unwrap_or(false);
            
            if pressed {
                println!(
                    "[SEARCH][{}] Successfully pressed Enter on input: {}", 
                    self.display_name(), 
                    selector
                );
                return Ok(());
            }
        }
        
        // 方案3: 提交表单
        let form_script = "() => {
            const forms = document.querySelectorAll('form');
            for (const form of forms) {
                const hasSearchInput = form.querySelector('input[name=\"q\"], textarea[name=\"q\"]');
                if (hasSearchInput) {
                    form.submit();
                    return true;
                }
            }
            return false;
        }";
        
        let submitted: bool = page
            .eval(form_script)
            .await
            .unwrap_or(false);
        
        if submitted {
            println!("[SEARCH][{}] Successfully submitted search form", self.display_name());
            return Ok(());
        }
        
        Err("Failed to trigger search with any method".to_string())
    }

    /// 等待搜索结果页面加载完成
    async fn wait_for_search_results(&self, page: &Page) -> Result<(), String> {
        println!("[SEARCH][{}] Waiting for search results...", self.display_name());
        
        let result_selectors = self.default_wait_selectors();
        let start = Instant::now();
        let timeout = Duration::from_millis(15000); // 15秒超时
        
        // 等待导航完成
        sleep(Duration::from_millis(1000)).await;
        
        // 检查结果选择器
        let selectors_json = serde_json::to_string(&result_selectors)
            .unwrap_or("[]".to_string());
        
        let script = format!(
            "() => {{ const sels = {}; for (const s of sels) {{ if (document.querySelector(s)) return s; }} return null; }}",
            selectors_json
        );

        let mut matched: Option<String> = None;
        while start.elapsed() < timeout {
            let found: Option<String> = page
                .eval(&script)
                .await
                .map_err(|e| format!("Failed to check result selectors: {}", e))?;

            if let Some(sel) = found {
                matched = Some(sel);
                break;
            }

            sleep(Duration::from_millis(250)).await;
        }
        
        if let Some(sel) = matched {
            println!(
                "[SEARCH][{}] Search results loaded, found selector: {}", 
                self.display_name(), 
                sel
            );
        } else {
            println!(
                "[SEARCH][{}] Search results wait timeout, but continuing...", 
                self.display_name()
            );
        }
        
        // 额外等待一点时间确保内容完全加载
        sleep(Duration::from_millis(1000)).await;
        
        Ok(())
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