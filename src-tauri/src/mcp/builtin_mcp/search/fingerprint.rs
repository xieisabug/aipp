use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use chrono::Timelike;
use playwright::api::browser_type::PersistentContextLauncher;

// 指纹配置接口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintConfig {
    pub device_name: String,
    pub locale: String,
    pub timezone_id: String,
    pub color_scheme: String, // "dark" | "light"
    pub reduced_motion: String, // "reduce" | "no-preference"
    pub forced_colors: String, // "active" | "none"
    pub user_agent: String,
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub device_scale_factor: f64,
    pub is_mobile: bool,
    pub has_touch: bool,
    pub screen_width: i32,
    pub screen_height: i32,
    pub accept_language: String,
    pub platform: String,
}

// 保存的状态文件接口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedState {
    pub fingerprint: Option<FingerprintConfig>,
    pub google_domain: Option<String>,
    pub last_update: Option<i64>,
}

/// 指纹管理器
pub struct FingerprintManager {
    state_file_path: String,
    saved_state: SavedState,
}

impl FingerprintManager {
    pub fn new(app_data_dir: &Path) -> Self {
        let state_file_path = app_data_dir.join("search_fingerprint_state.json").to_string_lossy().to_string();
        let saved_state = Self::load_saved_state(&state_file_path);
        
        Self {
            state_file_path,
            saved_state,
        }
    }

    /// 获取或生成稳定的指纹配置
    pub fn get_stable_fingerprint(&mut self, user_locale: Option<&str>) -> &FingerprintConfig {
        // 如果没有保存的指纹配置，生成一个新的
        if self.saved_state.fingerprint.is_none() {
            let config = self.generate_host_machine_config(user_locale);
            self.saved_state.fingerprint = Some(config);
            self.saved_state.last_update = Some(chrono::Utc::now().timestamp());
            self.save_state();
        }
        
        self.saved_state.fingerprint.as_ref().unwrap()
    }

    /// 生成基于宿主机器的指纹配置
    fn generate_host_machine_config(&self, user_locale: Option<&str>) -> FingerprintConfig {
        // 获取系统区域设置
        let system_locale = user_locale.unwrap_or("zh-CN");

        // 获取系统时区
        let timezone_id = self.detect_timezone();

        // 检测系统颜色方案（基于时间智能推断）
        let hour = chrono::Local::now().hour();
        let color_scheme = if hour >= 19 || hour < 7 { "dark" } else { "light" };

        // 选择一个常见的桌面设备
        let devices = self.get_common_desktop_devices();
        let device_template = &devices[fastrand::usize(0..devices.len())];

        // 生成随机但合理的屏幕分辨率变化
        let scale_variation = 0.9 + fastrand::f64() * 0.2; // 0.9 到 1.1
        let viewport_width = (device_template.viewport_width as f64 * scale_variation) as i32;
        let viewport_height = (device_template.viewport_height as f64 * scale_variation) as i32;
        let screen_width = (device_template.screen_width as f64 * scale_variation) as i32;
        let screen_height = (device_template.screen_height as f64 * scale_variation) as i32;

        FingerprintConfig {
            device_name: device_template.name.clone(),
            locale: system_locale.to_string(),
            timezone_id,
            color_scheme: color_scheme.to_string(),
            reduced_motion: "no-preference".to_string(),
            forced_colors: "none".to_string(),
            user_agent: device_template.user_agent.clone(),
            viewport_width,
            viewport_height,
            device_scale_factor: device_template.device_scale_factor,
            is_mobile: device_template.is_mobile,
            has_touch: device_template.has_touch,
            screen_width,
            screen_height,
            accept_language: self.generate_accept_language(system_locale),
            platform: self.detect_platform(),
        }
    }

    /// 检测系统时区
    fn detect_timezone(&self) -> String {
        // 获取系统时区偏移量
        let local_offset = chrono::Local::now().offset().local_minus_utc();
        let hours_offset = local_offset / 3600;

        match hours_offset {
            28800 => "Asia/Shanghai".to_string(),        // UTC+8 中国
            32400 => "Asia/Tokyo".to_string(),           // UTC+9 日本
            25200 => "Asia/Bangkok".to_string(),         // UTC+7 东南亚
            0 => "Europe/London".to_string(),            // UTC+0 英国
            3600 => "Europe/Berlin".to_string(),         // UTC+1 欧洲
            -18000 => "America/New_York".to_string(),    // UTC-5 美国东部
            -28800 => "America/Los_Angeles".to_string(), // UTC-8 美国西部
            _ => "Asia/Shanghai".to_string(),             // 默认
        }
    }

    /// 检测系统平台
    fn detect_platform(&self) -> String {
        if cfg!(target_os = "windows") {
            "Win32".to_string()
        } else if cfg!(target_os = "macos") {
            "MacIntel".to_string()
        } else {
            "Linux x86_64".to_string()
        }
    }

    /// 生成Accept-Language头
    fn generate_accept_language(&self, locale: &str) -> String {
        match locale {
            l if l.starts_with("zh") => "zh-CN,zh;q=0.9,en;q=0.8,en-US;q=0.7".to_string(),
            l if l.starts_with("en") => "en-US,en;q=0.9,zh-CN;q=0.8,zh;q=0.7".to_string(),
            l if l.starts_with("ja") => "ja,en;q=0.9,zh-CN;q=0.8".to_string(),
            l if l.starts_with("ko") => "ko,en;q=0.9,zh-CN;q=0.8".to_string(),
            _ => "zh-CN,zh;q=0.9,en;q=0.8,en-US;q=0.7".to_string(),
        }
    }

    /// 获取常见桌面设备配置
    fn get_common_desktop_devices(&self) -> Vec<DeviceTemplate> {
        vec![
            DeviceTemplate {
                name: "Desktop Chrome Windows".to_string(),
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
                viewport_width: 1920,
                viewport_height: 1080,
                device_scale_factor: 1.0,
                is_mobile: false,
                has_touch: false,
                screen_width: 1920,
                screen_height: 1080,
            },
            DeviceTemplate {
                name: "Desktop Chrome macOS".to_string(),
                user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
                viewport_width: 1440,
                viewport_height: 900,
                device_scale_factor: 2.0,
                is_mobile: false,
                has_touch: false,
                screen_width: 2880,
                screen_height: 1800,
            },
            DeviceTemplate {
                name: "Desktop Edge Windows".to_string(),
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0".to_string(),
                viewport_width: 1366,
                viewport_height: 768,
                device_scale_factor: 1.0,
                is_mobile: false,
                has_touch: false,
                screen_width: 1366,
                screen_height: 768,
            },
            DeviceTemplate {
                name: "Desktop High DPI".to_string(),
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
                viewport_width: 2560,
                viewport_height: 1440,
                device_scale_factor: 1.5,
                is_mobile: false,
                has_touch: false,
                screen_width: 2560,
                screen_height: 1440,
            },
        ]
    }

    /// 应用指纹配置到浏览器上下文  
    pub fn apply_fingerprint_to_context<'a>(
        &self,
        mut launcher: PersistentContextLauncher<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>,
        config: &'a FingerprintConfig,
    ) -> PersistentContextLauncher<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a> {
        use playwright::api::Viewport;
        
        launcher = launcher
            .user_agent(&config.user_agent)
            .locale(&config.locale)
            .timezone_id(&config.timezone_id)
            .viewport(Some(Viewport {
                width: config.viewport_width,
                height: config.viewport_height,
            }))
            .device_scale_factor(config.device_scale_factor)
            .is_mobile(config.is_mobile)
            .has_touch(config.has_touch);

        // 根据字符串设置颜色方案
        launcher = match config.color_scheme.as_str() {
            "dark" => launcher.color_scheme(playwright::api::ColorScheme::Dark),
            _ => launcher.color_scheme(playwright::api::ColorScheme::Light),
        };

        // 注意：暂时移除extra_http_headers调用以避免API兼容性问题
        // 将来可以通过页面级别的set_extra_http_headers来设置
        // 
        // 其他指纹伪装功能（反检测脚本、人性化行为等）仍然有效
        
        launcher
    }

    /// 获取增强的浏览器启动参数
    pub fn get_stealth_launch_args() -> Vec<String> {
        vec![
            // 基础隐身参数
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
            "--disable-dev-shm-usage".to_string(),
            "--disable-extensions".to_string(),
            
            // 重要：移除自动化控制标识
            "--disable-blink-features=AutomationControlled".to_string(),
            "--disable-features=VizDisplayCompositor".to_string(),
            
            // 禁用各种检测
            "--disable-background-timer-throttling".to_string(),
            "--disable-backgrounding-occluded-windows".to_string(),
            "--disable-renderer-backgrounding".to_string(),
            "--disable-feature-policy".to_string(),
            "--disable-ipc-flooding-protection".to_string(),
            
            // 模拟正常用户行为
            "--enable-features=NetworkService".to_string(),
            "--use-mock-keychain".to_string(),
            "--disable-component-update".to_string(),
            
            // 内存和性能优化
            "--max_old_space_size=4096".to_string(),
            "--memory-pressure-off".to_string(),
            
            // 禁用日志和错误报告
            "--disable-logging".to_string(),
            "--log-level=3".to_string(),
            "--silent".to_string(),
            
            // 网络优化
            "--aggressive-cache-discard".to_string(),
            "--enable-features=NetworkServiceInProcess".to_string(),
        ]
    }

    /// 获取随机但一致的延时配置
    pub fn get_timing_config() -> TimingConfig {
        TimingConfig {
            typing_delay_min: 50 + fastrand::u64(0..50),
            typing_delay_max: 120 + fastrand::u64(0..80),
            action_delay_min: 200 + fastrand::u64(0..100),
            action_delay_max: 500 + fastrand::u64(0..200),
            page_load_timeout: 15000 + fastrand::u64(0..5000),
        }
    }

    /// 加载保存的状态
    fn load_saved_state(file_path: &str) -> SavedState {
        if let Ok(content) = fs::read_to_string(file_path) {
            if let Ok(state) = serde_json::from_str::<SavedState>(&content) {
                return state;
            }
        }
        
        SavedState {
            fingerprint: None,
            google_domain: None,
            last_update: None,
        }
    }

    /// 保存状态到文件
    fn save_state(&self) {
        if let Ok(content) = serde_json::to_string_pretty(&self.saved_state) {
            if let Some(parent) = Path::new(&self.state_file_path).parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&self.state_file_path, content);
        }
    }

}

#[derive(Debug, Clone)]
struct DeviceTemplate {
    name: String,
    user_agent: String,
    viewport_width: i32,
    viewport_height: i32,
    device_scale_factor: f64,
    is_mobile: bool,
    has_touch: bool,
    screen_width: i32,
    screen_height: i32,
}

#[derive(Debug, Clone)]
pub struct TimingConfig {
    pub typing_delay_min: u64,
    pub typing_delay_max: u64,
    pub action_delay_min: u64,
    pub action_delay_max: u64,
    pub page_load_timeout: u64,
}
