use crate::config::{AppConfig, ConfigManager, MonitoringConfig, UiConfig, SystemConfig};
use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsData {
    pub monitoring: MonitoringConfig,
    pub ui: UiConfig,
    pub system: SystemConfig,
}

impl From<AppConfig> for SettingsData {
    fn from(config: AppConfig) -> Self {
        Self {
            monitoring: config.monitoring,
            ui: config.ui,
            system: config.system,
        }
    }
}

impl Into<AppConfig> for SettingsData {
    fn into(self) -> AppConfig {
        AppConfig {
            monitoring: self.monitoring,
            ui: self.ui,
            system: self.system,
        }
    }
}

// Tauri 命令函数
#[tauri::command]
pub async fn get_settings(config_manager: tauri::State<'_, ConfigManager>) -> Result<SettingsData, String> {
    let settings = config_manager.get_config().into();
    Ok(settings)
}

#[tauri::command]
pub async fn save_settings(
    config_manager: tauri::State<'_, ConfigManager>,
    app_handle: AppHandle,
    settings: SettingsData
) -> Result<(), String> {
    let config: AppConfig = settings.into();
    config_manager.update_config(config).map_err(|e| e.to_string())?;
    
    // 发送配置更新事件
    app_handle.emit_all("config-updated", ()).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn reset_settings(
    config_manager: tauri::State<'_, ConfigManager>,
    app_handle: AppHandle
) -> Result<SettingsData, String> {
    config_manager.reset_to_default().map_err(|e| e.to_string())?;
    
    // 发送配置重置事件
    app_handle.emit_all("config-reset", ()).map_err(|e| e.to_string())?;
    
    let settings = config_manager.get_config().into();
    Ok(settings)
}

#[tauri::command]
pub async fn validate_settings(settings: SettingsData) -> Result<bool, String> {
    let config: AppConfig = settings.into();
    config.validate().map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn export_settings(config_manager: tauri::State<'_, ConfigManager>) -> Result<String, String> {
    config_manager.export_config_json().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_settings(
    config_manager: tauri::State<'_, ConfigManager>,
    app_handle: AppHandle,
    json_data: String
) -> Result<SettingsData, String> {
    config_manager.import_config_json(&json_data).map_err(|e| e.to_string())?;
    
    // 发送配置导入事件
    app_handle.emit_all("config-imported", ()).map_err(|e| e.to_string())?;
    
    let settings = config_manager.get_config().into();
    Ok(settings)
}

#[tauri::command]
pub async fn get_config_file_path() -> Result<String, String> {
    AppConfig::get_config_path()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_config_directory() -> Result<(), String> {
    let config_path = AppConfig::get_config_path().map_err(|e| e.to_string())?;
    let config_dir = config_path.parent().ok_or("Could not get config directory")?;
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(config_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(config_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(config_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn test_audio_alert(config_manager: tauri::State<'_, ConfigManager>) -> Result<(), String> {
    use crate::audio::AudioManager;
    
    if config_manager.is_sound_enabled() {
        let audio_manager = AudioManager::new(true);
        audio_manager.test_audio().map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_data_conversion() {
        let config = AppConfig::default();
        let settings: SettingsData = config.clone().into();
        let converted_back: AppConfig = settings.into();
        
        assert_eq!(config.monitoring.check_interval, converted_back.monitoring.check_interval);
        assert_eq!(config.ui.alert_color, converted_back.ui.alert_color);
        assert_eq!(config.system.auto_startup, converted_back.system.auto_startup);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = SettingsData {
            monitoring: MonitoringConfig::default(),
            ui: UiConfig::default(),
            system: SystemConfig::default(),
        };
        
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: SettingsData = serde_json::from_str(&json).unwrap();
        
        assert_eq!(settings.monitoring.check_interval, deserialized.monitoring.check_interval);
        assert_eq!(settings.ui.alert_color, deserialized.ui.alert_color);
    }
}