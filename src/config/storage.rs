use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[allow(dead_code)] // 许多配置方法为将来的完整性而保留

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub check_interval: u64,
    pub sound_enabled: bool,
    pub auto_close_alert: bool,
    pub low_battery_threshold: u8,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            check_interval: 10,
            sound_enabled: true,
            auto_close_alert: true,
            low_battery_threshold: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub alert_color: String,
    pub low_battery_color: String,
    pub window_opacity: f32,
    pub always_on_top: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            alert_color: "#FF6B35".to_string(),
            low_battery_color: "#FF0000".to_string(),
            window_opacity: 0.95,
            always_on_top: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub auto_startup: bool,
    pub minimize_to_tray: bool,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            auto_startup: false,
            minimize_to_tray: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub monitoring: MonitoringConfig,
    pub ui: UiConfig,
    pub system: SystemConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            monitoring: MonitoringConfig::default(),
            ui: UiConfig::default(),
            system: SystemConfig::default(),
        }
    }
}

#[allow(dead_code)] // 许多配置方法为将来的完整性而保留
impl AppConfig {
    /// 获取配置文件路径
    pub fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .or_else(|| dirs::home_dir().map(|p| p.join(".config")))
            .ok_or("Could not determine config directory")?;
        
        let app_config_dir = config_dir.join("isBattery");
        
        // 确保配置目录存在
        if !app_config_dir.exists() {
            std::fs::create_dir_all(&app_config_dir)?;
        }
        
        Ok(app_config_dir.join("config.toml"))
    }

    /// 从文件加载配置
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            // 如果配置文件不存在，创建默认配置并保存
            let default_config = AppConfig::default();
            default_config.save()?;
            Ok(default_config)
        }
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 验证配置参数
    pub fn validate(&self) -> Result<(), String> {
        if self.monitoring.check_interval == 0 {
            return Err("检测间隔不能为0".to_string());
        }
        
        if self.monitoring.check_interval > 3600 {
            return Err("检测间隔不能超过3600秒".to_string());
        }

        if self.monitoring.low_battery_threshold > 100 {
            return Err("低电量阈值不能超过100%".to_string());
        }

        if self.ui.window_opacity < 0.0 || self.ui.window_opacity > 1.0 {
            return Err("窗口透明度必须在0.0到1.0之间".to_string());
        }

        // 验证颜色格式
        if !self.ui.alert_color.starts_with('#') || self.ui.alert_color.len() != 7 {
            return Err("提醒颜色格式无效，应为#RRGGBB格式".to_string());
        }

        if !self.ui.low_battery_color.starts_with('#') || self.ui.low_battery_color.len() != 7 {
            return Err("低电量提醒颜色格式无效，应为#RRGGBB格式".to_string());
        }

        Ok(())
    }

    /// 重置为默认配置
    pub fn reset_to_default(&mut self) {
        *self = AppConfig::default();
    }

    /// 更新监控配置
    pub fn update_monitoring(&mut self, config: MonitoringConfig) {
        self.monitoring = config;
    }

    /// 更新UI配置
    pub fn update_ui(&mut self, config: UiConfig) {
        self.ui = config;
    }

    /// 更新系统配置
    pub fn update_system(&mut self, config: SystemConfig) {
        self.system = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.monitoring.check_interval, 10);
        assert_eq!(config.monitoring.low_battery_threshold, 20);
        assert!(config.monitoring.sound_enabled);
        assert!(config.monitoring.auto_close_alert);
        assert_eq!(config.ui.alert_color, "#FF6B35");
        assert_eq!(config.ui.low_battery_color, "#FF0000");
        assert!(!config.system.auto_startup);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::default();
        assert!(config.validate().is_ok());

        // 测试无效的检测间隔
        config.monitoring.check_interval = 0;
        assert!(config.validate().is_err());

        config.monitoring.check_interval = 4000;
        assert!(config.validate().is_err());

        // 测试无效的电量阈值
        config.monitoring.check_interval = 10;
        config.monitoring.low_battery_threshold = 150;
        assert!(config.validate().is_err());

        // 测试无效的透明度
        config.monitoring.low_battery_threshold = 20;
        config.ui.window_opacity = 1.5;
        assert!(config.validate().is_err());

        // 测试无效的颜色格式
        config.ui.window_opacity = 0.95;
        config.ui.alert_color = "invalid_color".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(config.monitoring.check_interval, deserialized.monitoring.check_interval);
        assert_eq!(config.ui.alert_color, deserialized.ui.alert_color);
    }

    #[test]
    fn test_config_path() {
        let path = AppConfig::get_config_path();
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("isBattery"));
        assert!(path.to_string_lossy().ends_with("config.toml"));
    }
}