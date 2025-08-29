use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BrowserType {
    Chrome,
    Edge,
}

impl BrowserType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "chrome" => Some(BrowserType::Chrome),
            "edge" => Some(BrowserType::Edge),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserType::Chrome => "chrome",
            BrowserType::Edge => "edge",
        }
    }
}

pub struct BrowserManager {
    preferred_type: Option<BrowserType>,
}

impl BrowserManager {
    pub fn new(browser_type_config: Option<&str>) -> Self {
        let preferred_type = browser_type_config
            .and_then(|s| BrowserType::from_str(s));
        
        Self { preferred_type }
    }

    /// 获取可用的浏览器路径，使用降级策略：Chrome -> Edge -> Error
    pub fn get_available_browser(&self) -> Result<(BrowserType, PathBuf), String> {
        // 先尝试用户配置的浏览器类型（或默认Chrome）
        let primary_type = self.preferred_type
            .as_ref()
            .unwrap_or(&BrowserType::Chrome);
        
        if let Some(path) = self.find_browser_path(primary_type) {
            println!("[BROWSER] Using {} at {}", primary_type.as_str(), path.display());
            return Ok((primary_type.clone(), path));
        }

        // 降级到另一种浏览器
        let fallback_type = match primary_type {
            BrowserType::Chrome => BrowserType::Edge,
            BrowserType::Edge => BrowserType::Chrome,
        };

        if let Some(path) = self.find_browser_path(&fallback_type) {
            println!("[BROWSER] Fallback to {} at {}", fallback_type.as_str(), path.display());
            return Ok((fallback_type, path));
        }

        Err(format!("No supported browser (Chrome/Edge) found on system"))
    }

    /// 查找指定类型浏览器的可执行文件路径
    fn find_browser_path(&self, browser_type: &BrowserType) -> Option<PathBuf> {
        match browser_type {
            BrowserType::Chrome => self.find_chrome_path(),
            BrowserType::Edge => self.find_edge_path(),
        }
    }

    /// 查找Chrome浏览器路径
    fn find_chrome_path(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let candidates = [
                r"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                r"C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
                "chrome.exe", // 从PATH中查找
            ];
            self.try_candidates(&candidates)
        }

        #[cfg(target_os = "macos")]
        {
            let candidates = [
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                "chrome", // 从PATH中查找
                "google-chrome",
            ];
            self.try_candidates(&candidates)
        }

        #[cfg(target_os = "linux")]
        {
            let candidates = [
                "google-chrome",
                "google-chrome-stable",
                "chrome",
                "chromium",
                "chromium-browser",
            ];
            self.try_candidates(&candidates)
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    /// 查找Edge浏览器路径
    fn find_edge_path(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let candidates = [
                r"C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
                r"C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
                "msedge.exe", // 从PATH中查找
            ];
            self.try_candidates(&candidates)
        }

        #[cfg(target_os = "macos")]
        {
            let candidates = [
                "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
                "msedge", // 从PATH中查找
                "edge",
            ];
            self.try_candidates(&candidates)
        }

        #[cfg(target_os = "linux")]
        {
            let candidates = [
                "microsoft-edge",
                "microsoft-edge-stable",
                "msedge",
                "edge",
            ];
            self.try_candidates(&candidates)
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    /// 尝试多个候选路径，找到第一个存在的
    fn try_candidates(&self, candidates: &[&str]) -> Option<PathBuf> {
        for candidate in candidates {
            let path = PathBuf::from(candidate);
            
            // 如果是绝对路径，直接检查文件是否存在
            if path.is_absolute() {
                if path.is_file() {
                    return Some(path);
                }
            } else {
                // 如果是相对路径或命令名，从PATH中查找
                if let Ok(found_path) = which::which(candidate) {
                    return Some(found_path);
                }
            }
        }
        None
    }
}
