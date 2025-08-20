use chrono::Local;
use futures::future::BoxFuture;
use futures::FutureExt;
use htmd;
use regex::Regex;
use reqwest;
use serde::Serialize;
use std::collections::HashMap;

// 定义命令处理函数类型
type CommandFn = fn(TemplateEngine, String, HashMap<String, String>) -> BoxFuture<'static, String>;

// 获取当前日期的命令处理函数
fn current_date(
    _: TemplateEngine,
    _: String,
    _: HashMap<String, String>,
) -> BoxFuture<'static, String> {
    async { Local::now().format("%Y-%m-%d").to_string() }.boxed()
}

// 获取当前时间的命令处理函数
fn current_time(
    _: TemplateEngine,
    _: String,
    _: HashMap<String, String>,
) -> BoxFuture<'static, String> {
    async { Local::now().format("%H:%M:%S").to_string() }.boxed()
}

// 截取指定长度字符的命令处理函数
fn sub_start(
    engine: TemplateEngine,
    input: String,
    context: HashMap<String, String>,
) -> BoxFuture<'static, String> {
    async move {
        println!("input : {}", input);
        let re = Regex::new(r"\((.*),(\d+)\)").unwrap();
        for cap in re.captures_iter(&input) {
            println!("cap : {:?}", &cap);
            let text_origin = &cap[1];
            let num = &cap[2];

            let text = engine.parse(text_origin.trim(), &context).await;
            if let Ok(count) = num.trim().parse::<usize>() {
                return text.chars().take(count).collect();
            }
        }
        String::new()
    }
    .boxed()
}

fn selected_text(
    _: TemplateEngine,
    _: String,
    context: HashMap<String, String>,
) -> BoxFuture<'static, String> {
    async move { context.get("selected_text").unwrap_or(&String::default()).to_string() }.boxed()
}

// 新增获取网页内容的函数
fn web(_: TemplateEngine, url: String, _: HashMap<String, String>) -> BoxFuture<'static, String> {
    async move {
        // 移除url中前后的括号
        let url = url.trim_start_matches('(').trim_end_matches(')');

        let client = reqwest::Client::builder().danger_accept_invalid_certs(true).build().unwrap();

        match client.get(url).send().await {
            Ok(response) => {
                let html = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Failed to get web content".to_string());
                format!("<bangweb url=\"{}\">\n{}\n</bangweb>", url, html)
            }
            Err(err) => err.to_string(),
        }
    }
    .boxed()
}

// 新增获取网页内容并转换为 Markdown 的函数
fn web_to_markdown(
    _: TemplateEngine,
    url: String,
    _: HashMap<String, String>,
) -> BoxFuture<'static, String> {
    async move {
        // 移除url中前后的括号
        let url = url.trim_start_matches('(').trim_end_matches(')');

        let client = reqwest::Client::new();
        match client.get(url).send().await {
            Ok(response) => {
                let html = response.text().await.unwrap_or_default();
                format!(
                    "<bangwebtomarkdown url=\"{}\">\n{}\n</bangwebtomarkdown>",
                    url,
                    htmd::convert(&html).unwrap()
                )
            }
            Err(_) => "".to_string(),
        }
    }
    .boxed()
}

// 模板解析器结构体
#[derive(Clone)]
pub struct TemplateEngine {
    commands: HashMap<String, Bang>,
}

#[derive(Clone)]
pub struct Bang {
    pub name: String,
    pub complete: String,
    pub description: String,
    pub bang_type: BangType,
    pub command: CommandFn,
}

#[derive(Clone, Serialize)]
pub enum BangType {
    Text,
    Image,
    Audio,
}

impl TemplateEngine {
    // 初始化模板解析器
    pub fn new() -> Self {
        let mut commands = HashMap::new();

        commands.insert(
            "current_date".to_string(),
            Bang {
                name: "current_date".to_string(),
                complete: "current_date".to_string(),
                description: "获取当前日期".to_string(),
                bang_type: BangType::Text,
                command: current_date as CommandFn,
            },
        );
        commands.insert(
            "cd".to_string(),
            Bang {
                name: "cd".to_string(),
                complete: "cd".to_string(),
                description: "获取当前日期".to_string(),
                bang_type: BangType::Text,
                command: current_date as CommandFn,
            },
        );

        commands.insert(
            "current_time".to_string(),
            Bang {
                name: "current_time".to_string(),
                complete: "current_time".to_string(),
                description: "获取当前时间".to_string(),
                bang_type: BangType::Text,
                command: current_time as CommandFn,
            },
        );
        commands.insert(
            "ct".to_string(),
            Bang {
                name: "ct".to_string(),
                complete: "ct".to_string(),
                description: "获取当前时间".to_string(),
                bang_type: BangType::Text,
                command: current_time as CommandFn,
            },
        );

        commands.insert(
            "sub_start".to_string(),
            Bang {
                name: "sub_start".to_string(),
                complete: "sub_start(|)".to_string(),
                description: "截取文本的前多少个字符".to_string(),
                bang_type: BangType::Text,
                command: sub_start as CommandFn,
            },
        );

        commands.insert(
            "selected_text".to_string(),
            Bang {
                name: "selected_text".to_string(),
                complete: "selected_text".to_string(),
                description: "获取当前选中的文本".to_string(),
                bang_type: BangType::Text,
                command: selected_text as CommandFn,
            },
        );
        commands.insert(
            "s".to_string(),
            Bang {
                name: "s".to_string(),
                complete: "s".to_string(),
                description: "获取当前选中的文本".to_string(),
                bang_type: BangType::Text,
                command: selected_text as CommandFn,
            },
        );

        commands.insert(
            "web".to_string(),
            Bang {
                name: "web".to_string(),
                complete: "web(|)".to_string(),
                description: "通过网络获取URL的网页信息".to_string(),
                bang_type: BangType::Text,
                command: web as CommandFn,
            },
        );
        commands.insert(
            "w".to_string(),
            Bang {
                name: "w".to_string(),
                complete: "w(|)".to_string(),
                description: "通过网络获取URL的网页信息".to_string(),
                bang_type: BangType::Text,
                command: web as CommandFn,
            },
        );

        commands.insert(
            "web_to_markdown".to_string(),
            Bang {
                name: "web_to_markdown".to_string(),
                complete: "web_to_markdown(|)".to_string(),
                description: "通过网络获取URL的网页信息并且转换为markdown格式".to_string(),
                bang_type: BangType::Text,
                command: web_to_markdown as CommandFn,
            },
        );
        commands.insert(
            "wm".to_string(),
            Bang {
                name: "wm".to_string(),
                complete: "wm(|)".to_string(),
                description: "通过网络获取URL的网页信息并且转换为markdown格式".to_string(),
                bang_type: BangType::Text,
                command: web_to_markdown as CommandFn,
            },
        );

        TemplateEngine { commands }
    }

    // 注册命令
    pub fn register_command(&mut self, name: &str, handler: CommandFn) {
        self.commands.insert(
            name.to_string(),
            Bang {
                name: name.to_string(),
                complete: name.to_string(),
                description: "Custom command".to_string(),
                bang_type: BangType::Text,
                command: handler,
            },
        );
    }

    // 解析并替换模板字符串
    pub async fn parse(&self, template: &str, context: &HashMap<String, String>) -> String {
        let re = Regex::new(r"[!！](\w+)(\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\([^()]*\))*\))*\))*\))*\))*\))*\))*\))*\))*\))?").unwrap();
        let mut result = template.to_string();

        for cap in re.captures_iter(template) {
            println!("cap : {:?}", &cap);
            let command = &cap[1];
            let args = cap.get(2).map_or("", |m| m.as_str());
            if let Some(bang) = self.commands.get(command) {
                let replacement =
                    (bang.command)(self.clone(), args.to_string(), context.clone()).await;
                result = result.replace(&cap[0], &replacement);
            }
        }

        // 替换上下文变量
        for (key, value) in context {
            let placeholder = format!("!{}", key);
            result = result.replace(&placeholder, value);
        }

        result
    }

    pub fn get_commands(&self) -> Vec<Bang> {
        self.commands.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests;
