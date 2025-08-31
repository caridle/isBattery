use tauri::{AppHandle, Manager, Window, WindowBuilder, WindowUrl};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub message: String,
    pub background_color: String,
    pub text_color: String,
    pub opacity: f32,
    pub always_on_top: bool,
    pub auto_close: bool,
    pub show_battery_info: bool,
    pub battery_percentage: u8,
    // 新增功率相关字段
    pub power_draw_watts: Option<f32>,
    pub remaining_time_minutes: Option<u32>,
    pub charge_rate_watts: Option<f32>,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            message: "请连接电源适配器".to_string(),
            background_color: "#FF6B35".to_string(),
            text_color: "#FFFFFF".to_string(),
            opacity: 0.95,
            always_on_top: true,
            auto_close: true,
            show_battery_info: true,
            battery_percentage: 100,
            power_draw_watts: None,
            remaining_time_minutes: None,
            charge_rate_watts: None,
        }
    }
}

pub struct AlertManager {
    app_handle: Option<AppHandle>,
    active_alerts: HashMap<String, Window>,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            app_handle: None,
            active_alerts: HashMap::new(),
        }
    }

    /// 设置应用句柄
    pub fn set_app_handle(&mut self, app_handle: AppHandle) {
        self.app_handle = Some(app_handle);
    }

    /// 显示电源断开提醒
    pub fn show_power_disconnected_alert(&mut self, battery_status: &crate::power::BatteryStatus) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = AlertConfig::default();
        config.message = "请连接电源适配器".to_string();
        config.background_color = "#FF6B35".to_string();
        config.battery_percentage = battery_status.battery_percentage;
        config.power_draw_watts = battery_status.power_draw_watts;
        config.remaining_time_minutes = battery_status.remaining_time_minutes;
        config.charge_rate_watts = battery_status.charge_rate_watts;
        
        self.show_alert("power_disconnected", config)
    }

    /// 显示低电量提醒
    pub fn show_low_battery_alert(&mut self, battery_status: &crate::power::BatteryStatus) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = AlertConfig::default();
        config.message = "电池电量不足！请及时充电".to_string();
        config.background_color = "#FF0000".to_string();
        config.battery_percentage = battery_status.battery_percentage;
        config.power_draw_watts = battery_status.power_draw_watts;
        config.remaining_time_minutes = battery_status.remaining_time_minutes;
        config.charge_rate_watts = battery_status.charge_rate_watts;
        
        self.show_alert("low_battery", config)
    }

    /// 显示通用提醒窗口
    pub fn show_alert(&mut self, alert_id: &str, config: AlertConfig) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref app_handle) = self.app_handle {
            // 如果已经有相同类型的提醒窗口，先关闭它
            if let Some(existing_window) = self.active_alerts.get(alert_id) {
                let _ = existing_window.close();
            }

            let window_label = format!("alert_{}", alert_id);
            let window_title = "电源提醒";

            // 创建提醒窗口，初始状态为隐藏
            let window = WindowBuilder::new(
                app_handle,
                &window_label,
                WindowUrl::App("alert.html".into())
            )
            .title(window_title)
            .inner_size(400.0, 200.0)
            .min_inner_size(300.0, 150.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(config.always_on_top)
            .skip_taskbar(true)
            .visible(false) // 初始状态为隐藏，避免闪现
            .build()?;

            // 设置窗口位置到屏幕右下角（托盘区域附近）
            if let Ok(monitor) = window.primary_monitor() {
                if let Some(monitor) = monitor {
                    let size = monitor.size();
                    let window_width = 400.0;
                    let window_height = 200.0;
                    let margin = 50.0; // 距离屏幕边缘的边距
                    
                    let x = size.width as f64 - window_width - margin;
                    let y = size.height as f64 - window_height - margin;
                    
                    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x: x as i32, y: y as i32 }));
                }
            }

            // 设置窗口透明度（暂时禁用）
            // if config.opacity < 1.0 {
            //     let _ = window.set_opacity(config.opacity as f64);
            // }

            // 发送配置数据并显示窗口
            let config_clone = config.clone();
            let window_clone = window.clone();
            tokio::spawn(async move {
                // 稍微延迟确保窗口加载完成
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let _ = window_clone.emit("alert-config", &config_clone);
                // 在配置发送后显示窗口，避免空窗口闪现
                let _ = window_clone.show();
            });

            // 存储窗口引用
            self.active_alerts.insert(alert_id.to_string(), window);
        }

        Ok(())
    }

    /// 更新已打开的提醒窗口中的电量信息
    pub fn update_battery_percentage(&self, battery_percentage: u8) -> Result<(), Box<dyn std::error::Error>> {
        for (alert_id, window) in &self.active_alerts {
            let updated_config = match alert_id.as_str() {
                "power_disconnected" => AlertConfig {
                    message: "请连接电源适配器".to_string(),
                    background_color: "#FF6B35".to_string(),
                    battery_percentage,
                    ..AlertConfig::default()
                },
                "low_battery" => AlertConfig {
                    message: "电池电量不足！请及时充电".to_string(),
                    background_color: "#FF0000".to_string(),
                    battery_percentage,
                    ..AlertConfig::default()
                },
                _ => continue,
            };
            
            let _ = window.emit("alert-config", &updated_config);
        }
        Ok(())
    }

    /// 关闭指定的提醒窗口
    pub fn close_alert(&mut self, alert_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(window) = self.active_alerts.remove(alert_id) {
            window.close()?;
        }
        Ok(())
    }

    /// 关闭所有提醒窗口
    pub fn close_all_alerts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (_, window) in self.active_alerts.drain() {
            let _ = window.close();
        }
        Ok(())
    }

}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

// Tauri 命令函数
#[tauri::command]
pub fn close_alert_window(app_handle: AppHandle, alert_id: String) -> Result<(), String> {
    if let Some(window) = app_handle.get_window(&format!("alert_{}", alert_id)) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn pause_monitoring_from_alert(app_handle: AppHandle) -> Result<(), String> {
    app_handle.emit_all("pause-monitoring", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_alert_config(alert_type: String) -> Result<AlertConfig, String> {
    match alert_type.as_str() {
        "power_disconnected" => Ok(AlertConfig {
            message: "请连接电源适配器".to_string(),
            background_color: "#FF6B35".to_string(),
            ..AlertConfig::default()
        }),
        "low_battery" => Ok(AlertConfig {
            message: "电池电量不足！请及时充电".to_string(),
            background_color: "#FF0000".to_string(),
            ..AlertConfig::default()
        }),
        _ => Err("Unknown alert type".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_manager_creation() {
        let alert_manager = AlertManager::new();
        assert!(!alert_manager.has_active_alerts());
        assert_eq!(alert_manager.active_alert_count(), 0);
    }

    #[test]
    fn test_alert_config_default() {
        let config = AlertConfig::default();
        assert_eq!(config.message, "请连接电源适配器");
        assert_eq!(config.background_color, "#FF6B35");
        assert!(config.always_on_top);
        assert!(config.auto_close);
    }

    #[test]
    fn test_alert_config_serialization() {
        let config = AlertConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AlertConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(config.message, deserialized.message);
        assert_eq!(config.background_color, deserialized.background_color);
    }
}