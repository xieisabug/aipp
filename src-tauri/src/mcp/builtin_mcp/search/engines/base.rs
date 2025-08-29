/// 搜索引擎通用基础功能
pub struct SearchEngineBase;

impl SearchEngineBase {
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
            r#"(?is)<div[^>]*id=\"?content\"?[^>]*>(.*?)</div>"#,
            r#"(?is)<div[^>]*class=\"[^"]*content[^"]*\"[^>]*>(.*?)</div>"#,
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
        let link_pattern = regex::Regex::new(r#"(?is)<a[^>]*href=\"([^\"]*)\"[^>]*>(.*?)</a>"#).unwrap();
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
}
