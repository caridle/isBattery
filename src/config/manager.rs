use crate::config::{AppConfig, MonitoringConfig, UiConfig, SystemConfig};
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // 许多配置方法为将来的完整性而保留

#[derive(Clone)]
pub struct ConfigManager {
    config: Arc<Mutex<AppConfig>>,
}

#[allow(dead_code)] // 许多配置方法为将来的完整性而保留
impl ConfigManager {
    /// 创建配置管理器
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = AppConfig::load()?;
        
        // 验证配置
        config.validate().map_err(|e| format!("配置验证失败: {}", e))?;
        
        Ok(Self {
            config: Arc::new(Mutex::new(config)),
        })
    }

    /// 获取当前配置的副本
    pub fn get_config(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    /// 更新整个配置
    pub fn update_config(&self, new_config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
        // 验证新配置
        new_config.validate().map_err(|e| format!("配置验证失败: {}", e))?;
        
        {
            let mut config = self.config.lock().unwrap();
            *config = new_config;
        }
        
        self.save_config()
    }

    /// 更新监控配置
    pub fn update_monitoring_config(&self, monitoring_config: MonitoringConfig) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.lock().unwrap();
            config.update_monitoring(monitoring_config);
            
            // 验证更新后的配置
            config.validate().map_err(|e| format!("配置验证失败: {}", e))?;
        }
        
        self.save_config()
    }

    /// 更新UI配置
    pub fn update_ui_config(&self, ui_config: UiConfig) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.lock().unwrap();
            config.update_ui(ui_config);
            
            // 验证更新后的配置
            config.validate().map_err(|e| format!("配置验证失败: {}", e))?;
        }
        
        self.save_config()
    }

    /// 更新系统配置
    pub fn update_system_config(&self, system_config: SystemConfig) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.lock().unwrap();
            config.update_system(system_config);
        }
        
        self.save_config()
    }

    /// 保存配置到文件
    pub fn save_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.lock().unwrap();
        config.save()
    }

    /// 重置配置为默认值
    pub fn reset_to_default(&self) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.lock().unwrap();
            config.reset_to_default();
        }
        
        self.save_config()
    }

    /// 获取监控配置
    pub fn get_monitoring_config(&self) -> MonitoringConfig {
        self.config.lock().unwrap().monitoring.clone()
    }

    /// 获取UI配置
    pub fn get_ui_config(&self) -> UiConfig {
        self.config.lock().unwrap().ui.clone()
    }

    /// 获取系统配置
    pub fn get_system_config(&self) -> SystemConfig {
        self.config.lock().unwrap().system.clone()
    }

    /// 获取检测间隔（秒）
    pub fn get_check_interval(&self) -> u64 {
        self.config.lock().unwrap().monitoring.check_interval
    }

    /// 获取低电量阈值
    pub fn get_low_battery_threshold(&self) -> u8 {
        self.config.lock().unwrap().monitoring.low_battery_threshold
    }

    /// 是否启用声音提醒
    pub fn is_sound_enabled(&self) -> bool {
        self.config.lock().unwrap().monitoring.sound_enabled
    }

    /// 是否自动关闭提醒
    pub fn is_auto_close_alert_enabled(&self) -> bool {
        self.config.lock().unwrap().monitoring.auto_close_alert
    }

    /// 获取提醒颜色
    pub fn get_alert_color(&self) -> String {
        self.config.lock().unwrap().ui.alert_color.clone()
    }

    /// 获取低电量提醒颜色
    pub fn get_low_battery_alert_color(&self) -> String {
        self.config.lock().unwrap().ui.low_battery_color.clone()
    }

    /// 是否窗口置顶
    pub fn is_always_on_top(&self) -> bool {
        self.config.lock().unwrap().ui.always_on_top
    }

    /// 获取窗口透明度
    pub fn get_window_opacity(&self) -> f32 {
        self.config.lock().unwrap().ui.window_opacity
    }

    /// 是否开机自启动
    pub fn is_auto_startup_enabled(&self) -> bool {
        self.config.lock().unwrap().system.auto_startup
    }

    /// 是否最小化到托盘
    pub fn is_minimize_to_tray_enabled(&self) -> bool {
        self.config.lock().unwrap().system.minimize_to_tray
    }

    /// 设置检测间隔
    pub fn set_check_interval(&self, interval: u64) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.lock().unwrap();
            config.monitoring.check_interval = interval;
            config.validate().map_err(|e| format!("配置验证失败: {}", e))?;
        }
        self.save_config()
    }

    /// 设置低电量阈值
    pub fn set_low_battery_threshold(&self, threshold: u8) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.lock().unwrap();
            config.monitoring.low_battery_threshold = threshold;
            config.validate().map_err(|e| format!("配置验证失败: {}", e))?;
        }
        self.save_config()
    }

    /// 设置是否启用声音
    pub fn set_sound_enabled(&self, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.lock().unwrap();
            config.monitoring.sound_enabled = enabled;
        }
        self.save_config()
    }

    /// 设置开机自启动
    pub fn set_auto_startup(&self, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.lock().unwrap();
            config.system.auto_startup = enabled;
        }
        self.save_config()
    }

    /// 导出配置为JSON字符串（用于设置界面）
    pub fn export_config_json(&self) -> Result<String, Box<dyn std::error::Error>> {
        let config = self.config.lock().unwrap();
        Ok(serde_json::to_string_pretty(&*config)?)
    }

    /// 从JSON字符串导入配置（用于设置界面）
    pub fn import_config_json(&self, json: &str) -> Result<(), Box<dyn std::error::Error>> {
        let new_config: AppConfig = serde_json::from_str(json)?;
        self.update_config(new_config)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // 如果加载失败，使用默认配置
            Self {
                config: Arc::new(Mutex::new(AppConfig::default())),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_manager_creation() {
        let manager = ConfigManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_config_manager_operations() {
        let manager = ConfigManager::default();
        
        // 测试获取配置
        let config = manager.get_config();
        assert_eq!(config.monitoring.check_interval, 10);
        
        // 测试更新检测间隔
        assert!(manager.set_check_interval(15).is_ok());
        assert_eq!(manager.get_check_interval(), 15);
        
        // 测试更新低电量阈值
        assert!(manager.set_low_battery_threshold(25).is_ok());
        assert_eq!(manager.get_low_battery_threshold(), 25);
        
        // 测试声音设置
        assert!(manager.set_sound_enabled(false).is_ok());
        assert!(!manager.is_sound_enabled());
    }

    #[test]
    fn test_config_validation_in_manager() {
        let manager = ConfigManager::default();
        
        // 测试无效的检测间隔
        assert!(manager.set_check_interval(0).is_err());
        assert!(manager.set_check_interval(4000).is_err());
        
        // 测试无效的阈值
        assert!(manager.set_low_battery_threshold(150).is_err());
    }

    #[test]
    fn test_json_import_export() {
        let manager = ConfigManager::default();
        
        // 导出配置
        let json = manager.export_config_json().unwrap();
        assert!(json.contains("check_interval"));
        
        // 修改配置
        manager.set_check_interval(20).unwrap();
        
        // 导入配置应该恢复原来的值
        manager.import_config_json(&json).unwrap();
        assert_eq!(manager.get_check_interval(), 10);
    }
}