use playwright::api::Page;
use tokio::time::{sleep, Duration};
use std::time::Instant;
use crate::api::builtin_mcp::search::types::{SearchResultType, SearchResponse, SearchResults};

/// 搜索引擎通用基础功能
pub struct SearchEngineBase;

impl SearchEngineBase {
    /// 执行完整的搜索流程
    pub async fn perform_search(
        page: &Page, 
        query: &str,
        display_name: &str,
        homepage_url: &str,
        search_input_selectors: &[&str],
        search_button_selectors: &[&str],
        default_wait_selectors: &[String],
    ) -> Result<String, String> {
        println!(
            "[SEARCH][{}] Starting search flow for query: {}", 
            display_name, 
            query
        );

        // 步骤1: 导航到首页
        println!(
            "[SEARCH][{}] Navigating to homepage: {}", 
            display_name, 
            homepage_url
        );
        
        page.goto_builder(homepage_url)
            .goto()
            .await
            .map_err(|e| format!("Failed to navigate to {}: {}", homepage_url, e))?;
        
        // 等待搜索框出现
        Self::wait_for_search_input(page, display_name, search_input_selectors).await?;

        // 步骤2: 查找并填充搜索框
        let input_found = Self::find_and_fill_search_input(page, query, display_name, search_input_selectors).await?;
        if !input_found {
            return Err(format!(
                "Could not find search input for {} after trying all selectors", 
                display_name
            ));
        }

        // 步骤3: 点击搜索按钮或按Enter
        Self::trigger_search(page, display_name, search_button_selectors, search_input_selectors).await?;
        
        // 步骤4: 等待搜索结果加载
        Self::wait_for_search_results(page, display_name, default_wait_selectors).await?;

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
            display_name, 
            html.len()
        );

        Ok(html)
    }

    /// 根据结果类型处理搜索结果
    pub async fn process_search_with_type<F>(
        page: &Page, 
        query: &str,
        result_type: SearchResultType,
        display_name: &str,
        homepage_url: &str,
        search_input_selectors: &[&str],
        search_button_selectors: &[&str],
        default_wait_selectors: &[String],
        engine_id: &str,
        parse_results_fn: F,
    ) -> Result<SearchResponse, String>
    where
        F: Fn(&str, &str) -> SearchResults,
    {
        // 执行搜索获取HTML
        let html = Self::perform_search(
            page,
            query,
            display_name,
            homepage_url,
            search_input_selectors,
            search_button_selectors,
            default_wait_selectors,
        ).await?;

        // 根据结果类型处理返回
        match result_type {
            SearchResultType::Html => {
                Ok(SearchResponse::Html {
                    query: query.to_string(),
                    homepage_url: homepage_url.to_string(),
                    search_engine: display_name.to_string(),
                    engine_id: engine_id.to_string(),
                    html_content: html,
                    message: format!("Successfully retrieved HTML search results from {}", display_name),
                })
            }
            SearchResultType::Markdown => {
                let markdown_content = Self::html_to_markdown(&html);
                Ok(SearchResponse::Markdown {
                    query: query.to_string(),
                    homepage_url: homepage_url.to_string(),
                    search_engine: display_name.to_string(),
                    engine_id: engine_id.to_string(),
                    markdown_content,
                    message: format!("Successfully converted {} search results to Markdown format", display_name),
                })
            }
            SearchResultType::Items => {
                let search_results = parse_results_fn(&html, query);
                Ok(SearchResponse::Items(search_results))
            }
        }
    }

    /// 将HTML转换为Markdown格式
    pub fn html_to_markdown(html: &str) -> String {
        // 基本的HTML到Markdown转换
        let mut markdown = html.to_string();
        
        // 清理HTML，只保留主要内容相关的部分
        markdown = Self::extract_main_content(&markdown);
        
        // HTML标签转换为Markdown语法
        markdown = Self::convert_html_tags_to_markdown(&markdown);
        
        // 清理多余的空白行
        let lines: Vec<&str> = markdown.lines().collect();
        let mut cleaned_lines = Vec::new();
        let mut prev_empty = false;
        
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                if !prev_empty {
                    cleaned_lines.push(String::new());
                    prev_empty = true;
                }
            } else {
                cleaned_lines.push(line.to_string());
                prev_empty = false;
            }
        }
        
        cleaned_lines.join("\n").trim().to_string()
    }

    /// 提取HTML中的主要内容
    fn extract_main_content(html: &str) -> String {
        let mut content = html.to_string();
        
        // 移除脚本和样式标签
        let script_pattern = regex::Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
        content = script_pattern.replace_all(&content, "").to_string();
        
        let style_pattern = regex::Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
        content = style_pattern.replace_all(&content, "").to_string();
        
        // 移除注释
        let comment_pattern = regex::Regex::new(r"<!--.*?-->").unwrap();
        content = comment_pattern.replace_all(&content, "").to_string();
        
        // 尝试提取主要内容区域
        let main_patterns = [
            r"(?is)<main[^>]*>(.*?)</main>",
            r"(?is)<article[^>]*>(.*?)</article>",
            r#"(?is)<div[^>]*id="?content"?[^>]*>(.*?)</div>"#,
            r#"(?is)<div[^>]*class="[^"]*content[^"]*"[^>]*>(.*?)</div>"#,
        ];
        
        for pattern in &main_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(cap) = re.captures(&content) {
                    if let Some(matched) = cap.get(1) {
                        content = matched.as_str().to_string();
                        break;
                    }
                }
            }
        }
        
        content
    }

    /// 将HTML标签转换为Markdown语法
    fn convert_html_tags_to_markdown(html: &str) -> String {
        let mut markdown = html.to_string();
        
        // 标题转换
        for i in 1..=6 {
            let pattern = format!(r"(?is)<h{0}[^>]*>(.*?)</h{0}>", i);
            if let Ok(re) = regex::Regex::new(&pattern) {
                let replacement = format!("{} $1\n", "#".repeat(i));
                markdown = re.replace_all(&markdown, replacement.as_str()).to_string();
            }
        }
        
        // 段落转换
        let p_pattern = regex::Regex::new(r"(?is)<p[^>]*>(.*?)</p>").unwrap();
        markdown = p_pattern.replace_all(&markdown, "$1\n\n").to_string();
        
        // 链接转换
        let link_pattern = regex::Regex::new(r#"(?is)<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#).unwrap();
        markdown = link_pattern.replace_all(&markdown, "[$2]($1)").to_string();
        
        // 粗体和斜体
        let strong_pattern = regex::Regex::new(r"(?is)<(?:strong|b)[^>]*>(.*?)</(?:strong|b)>").unwrap();
        markdown = strong_pattern.replace_all(&markdown, "**$1**").to_string();
        
        let em_pattern = regex::Regex::new(r"(?is)<(?:em|i)[^>]*>(.*?)</(?:em|i)>").unwrap();
        markdown = em_pattern.replace_all(&markdown, "*$1*").to_string();
        
        // 列表转换
        let ul_pattern = regex::Regex::new(r"(?is)<ul[^>]*>(.*?)</ul>").unwrap();
        let li_pattern = regex::Regex::new(r"(?is)<li[^>]*>(.*?)</li>").unwrap();
        
        markdown = ul_pattern.replace_all(&markdown, |caps: &regex::Captures| {
            let list_content = &caps[1];
            let items = li_pattern.replace_all(list_content, "- $1\n");
            format!("\n{}\n", items)
        }).to_string();
        
        // 有序列表
        let ol_pattern = regex::Regex::new(r"(?is)<ol[^>]*>(.*?)</ol>").unwrap();
        markdown = ol_pattern.replace_all(&markdown, |caps: &regex::Captures| {
            let list_content = &caps[1];
            let mut counter = 1;
            let items = li_pattern.replace_all(list_content, |_: &regex::Captures| {
                let result = format!("{}. $1\n", counter);
                counter += 1;
                result
            });
            format!("\n{}\n", items)
        }).to_string();
        
        // 代码块
        let pre_pattern = regex::Regex::new(r"(?is)<pre[^>]*>(.*?)</pre>").unwrap();
        markdown = pre_pattern.replace_all(&markdown, "```\n$1\n```\n").to_string();
        
        let code_pattern = regex::Regex::new(r"(?is)<code[^>]*>(.*?)</code>").unwrap();
        markdown = code_pattern.replace_all(&markdown, "`$1`").to_string();
        
        // 分割线
        let hr_pattern = regex::Regex::new(r"(?is)<hr[^>]*/?>\s*").unwrap();
        markdown = hr_pattern.replace_all(&markdown, "\n---\n").to_string();
        
        // 换行
        let br_pattern = regex::Regex::new(r"(?is)<br[^>]*/?>\s*").unwrap();
        markdown = br_pattern.replace_all(&markdown, "\n").to_string();
        
        // 移除剩余的HTML标签
        let tag_pattern = regex::Regex::new(r"<[^>]*>").unwrap();
        markdown = tag_pattern.replace_all(&markdown, "").to_string();
        
        // 解码HTML实体
        markdown = markdown
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ")
            .replace("&ndash;", "–")
            .replace("&mdash;", "—");
        
        markdown
    }
    
    /// 等待搜索框出现
    async fn wait_for_search_input(page: &Page, display_name: &str, selectors: &[&str]) -> Result<(), String> {
        let start = Instant::now();
        let timeout = Duration::from_millis(10000); // 10秒超时
        
        loop {
            for selector in selectors {
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
                        display_name, 
                        selector
                    );
                    return Ok(());
                }
            }
            
            if start.elapsed() >= timeout {
                println!(
                    "[SEARCH][{}] Search input wait timeout after {} ms", 
                    display_name, 
                    timeout.as_millis()
                );
                return Ok(()); // 不要失败，继续尝试
            }
            
            sleep(Duration::from_millis(500)).await;
        }
    }

    /// 查找并填充搜索输入框
    async fn find_and_fill_search_input(page: &Page, query: &str, display_name: &str, selectors: &[&str]) -> Result<bool, String> {
        for selector in selectors {
            println!(
                "[SEARCH][{}] Trying input selector: {}", 
                display_name, 
                selector
            );
            
            // 检查元素是否存在和可见
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
            
            if !is_visible {
                continue;
            }

            // 尝试填充输入框
            match Self::fill_search_input(page, selector, query).await {
                Ok(_) => {
                    println!(
                        "[SEARCH][{}] Successfully filled input with selector: {}", 
                        display_name, 
                        selector
                    );
                    return Ok(true);
                },
                Err(e) => {
                    println!(
                        "[SEARCH][{}] Failed to fill input with selector {}: {}", 
                        display_name, 
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
    async fn fill_search_input(page: &Page, selector: &str, query: &str) -> Result<(), String> {
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
            selector.replace("'", "\\'")
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
            selector.replace("'", "\\'")
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
                selector.replace("'", "\\'"),
                ch.to_string().replace("'", "\\'")
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
            selector.replace("'", "\\'")
        );
        
        page.eval::<()>(&final_script)
            .await
            .map_err(|e| format!("Failed to trigger change event: {}", e))?;
        
        Ok(())
    }

    /// 触发搜索（点击按钮或按Enter）
    async fn trigger_search(page: &Page, display_name: &str, button_selectors: &[&str], input_selectors: &[&str]) -> Result<(), String> {
        // 方案1: 尝试点击搜索按钮
        for selector in button_selectors {
            println!(
                "[SEARCH][{}] Trying search button selector: {}", 
                display_name, 
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
                selector.replace("'", "\\'")
            );
            
            let clicked: bool = page
                .eval(&button_script)
                .await
                .unwrap_or(false);
            
            if clicked {
                println!(
                    "[SEARCH][{}] Successfully clicked search button: {}", 
                    display_name, 
                    selector
                );
                return Ok(());
            }
        }
        
        // 方案2: 如果按钮点击失败，尝试按Enter键
        println!("[SEARCH][{}] Button click failed, trying Enter key", display_name);
        
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
                selector.replace("'", "\\'")
            );
            
            let pressed: bool = page
                .eval(&enter_script)
                .await
                .unwrap_or(false);
            
            if pressed {
                println!(
                    "[SEARCH][{}] Successfully pressed Enter on input: {}", 
                    display_name, 
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
            println!("[SEARCH][{}] Successfully submitted search form", display_name);
            return Ok(());
        }
        
        Err("Failed to trigger search with any method".to_string())
    }

    /// 等待搜索结果页面加载完成
    async fn wait_for_search_results(page: &Page, display_name: &str, result_selectors: &[String]) -> Result<(), String> {
        println!("[SEARCH][{}] Waiting for search results...", display_name);
        
        let start = Instant::now();
        let timeout = Duration::from_millis(15000); // 15秒超时
        
        // 等待导航完成
        sleep(Duration::from_millis(1000)).await;
        
        // 检查结果选择器
        let selectors_json = serde_json::to_string(result_selectors)
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
                display_name, 
                sel
            );
        } else {
            println!(
                "[SEARCH][{}] Search results wait timeout, but continuing...", 
                display_name
            );
        }
        
        // 额外等待一点时间确保内容完全加载
        sleep(Duration::from_millis(1000)).await;
        
        Ok(())
    }
}