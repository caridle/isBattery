use crate::power::BatteryStatus;
use crate::{log_info, log_error};
use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TrayManager {
    app_handle: Option<AppHandle>,
    current_status: Arc<Mutex<Option<BatteryStatus>>>,
    is_monitoring: Arc<Mutex<bool>>,
}

impl TrayManager {
    pub fn new() -> Self {
        Self {
            app_handle: None,
            current_status: Arc::new(Mutex::new(None)),
            is_monitoring: Arc::new(Mutex::new(false)),
        }
    }

    /// 设置应用句柄
    pub fn set_app_handle(&mut self, app_handle: AppHandle) {
        self.app_handle = Some(app_handle);
    }

    /// 创建系统托盘
    pub fn create_system_tray() -> SystemTray {
        let status_item = CustomMenuItem::new("status".to_string(), "获取状态中...");
        let settings_item = CustomMenuItem::new("settings".to_string(), "设置");
        let pause_item = CustomMenuItem::new("pause".to_string(), "暂停监控");
        let resume_item = CustomMenuItem::new("resume".to_string(), "恢复监控");
        let startup_item = CustomMenuItem::new("startup".to_string(), "开机启动");
        let about_item = CustomMenuItem::new("about".to_string(), "关于");
        let quit_item = CustomMenuItem::new("quit".to_string(), "退出");

        let tray_menu = SystemTrayMenu::new()
            .add_item(status_item.disabled())
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(settings_item)
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(pause_item)
            .add_item(resume_item.disabled())
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(startup_item)
            .add_item(about_item)
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(quit_item);

        SystemTray::new().with_menu(tray_menu)
    }

    /// 更新托盘状态
    pub fn update_status(&self, status: &BatteryStatus) {
        {
            let mut current_status = self.current_status.lock().unwrap();
            *current_status = Some(status.clone());
        }

        if let Some(ref app_handle) = self.app_handle {
            let status_text = self.format_status_text(status);
            let _ = app_handle.tray_handle().get_item("status").set_title(&status_text);
            
            // 更新托盘图标（暂时禁用）
            // let icon_data = self.get_icon_data_for_status(status);
            // if let Ok(icon) = tauri::Icon::Raw(icon_data) {
            //     let _ = app_handle.tray_handle().set_icon(icon);
            // }
        }
    }

    /// 更新监控状态
    pub fn update_monitoring_status(&self, is_monitoring: bool) {
        {
            let mut monitoring = self.is_monitoring.lock().unwrap();
            *monitoring = is_monitoring;
        }

        if let Some(ref app_handle) = self.app_handle {
            let tray_handle = app_handle.tray_handle();
            
            if is_monitoring {
                let _ = tray_handle.get_item("pause").set_enabled(true);
                let _ = tray_handle.get_item("resume").set_enabled(false);
            } else {
                let _ = tray_handle.get_item("pause").set_enabled(false);
                let _ = tray_handle.get_item("resume").set_enabled(true);
            }
        }
    }

    /// 格式化状态文本
    fn format_status_text(&self, status: &BatteryStatus) -> String {
        let power_source = if status.is_ac_connected {
            "电源适配器"
        } else {
            "电池"
        };

        let charging_status = if status.is_charging {
            " (充电中)"
        } else {
            ""
        };

        if status.is_battery_present {
            let mut status_text = format!("电源: {} | 电量: {}%{}", power_source, status.battery_percentage, charging_status);
            
            // 添加功率信息
            if let Some(power_watts) = status.power_draw_watts {
                status_text.push_str(&format!(" | 功耗: {:.1}W", power_watts));
            }
            
            // 添加剩余时间
            if let Some(remaining_minutes) = status.remaining_time_minutes {
                if remaining_minutes > 0 && !status.is_charging {
                    let hours = remaining_minutes / 60;
                    let minutes = remaining_minutes % 60;
                    if hours > 0 {
                        status_text.push_str(&format!(" | 剩余: {}h{}m", hours, minutes));
                    } else {
                        status_text.push_str(&format!(" | 剩余: {}m", minutes));
                    }
                }
            }
            
            status_text
        } else {
            format!("电源: {}", power_source)
        }
    }

    /// 处理托盘事件
    pub fn handle_tray_event(app_handle: &AppHandle, event: SystemTrayEvent) {
        match event {
            SystemTrayEvent::LeftClick { .. } => {
                // 左键点击显示主窗口或状态信息
                Self::show_settings_window(app_handle);
            }
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "settings" => {
                        Self::show_settings_window(app_handle);
                    }
                    "pause" => {
                        app_handle.emit_all("pause-monitoring", ()).unwrap();
                    }
                    "resume" => {
                        app_handle.emit_all("resume-monitoring", ()).unwrap();
                    }
                    "startup" => {
                        // 直接在此处处理，避免生命周期问题
                        let app_state: tauri::State<crate::AppState> = app_handle.state();
                        let startup_manager = app_state.startup_manager.lock().unwrap();
                        
                        match startup_manager.toggle() {
                            Ok(enabled) => {
                                // 更新配置
                                let _ = app_state.config_manager.set_auto_startup(enabled);
                                
                                // 更新托盘菜单
                                let tray_manager = app_state.tray_manager.lock().unwrap();
                                tray_manager.update_startup_menu(enabled);
                                
                                log_info!("Auto startup toggled: {}", enabled);
                            }
                            Err(e) => {
                                log_error!("Failed to toggle startup from tray: {}", e);
                            }
                        }
                    }
                    "about" => {
                        Self::show_about_dialog(app_handle);
                    }
                    "quit" => {
                        app_handle.exit(0);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// 显示设置窗口（避免重复创建）
    fn show_settings_window(app_handle: &AppHandle) {
        if let Some(window) = app_handle.get_window("main") {
            // 窗口已存在，只显示并设置焦点
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.unminimize(); // 确保窗口不是最小化状态
        } else {
            // 如果窗口不存在，记录错误但不创建新窗口
            log_error!("Main window not found, cannot show settings");
        }
    }

    /// 显示关于对话框
    fn show_about_dialog(app_handle: &AppHandle) {
        use tauri::api::dialog;
        
        let message = format!(
            "isBattery v{}\n\n电源监控程序\n\n功能特性:\n• 电源状态监控\n• 低电量提醒\n• 系统托盘显示\n• 开机自启动\n• 功率负载监控\n\n作者: isBattery Team",
            env!("CARGO_PKG_VERSION")
        );

        // 使用异步方式显示对话框，避免阻塞主线程
        tauri::async_runtime::spawn(async move {
            dialog::message(None::<&tauri::Window>, "关于 isBattery", &message);
        });
    }

    /// 更新自启动菜单项状态
    pub fn update_startup_menu(&self, enabled: bool) {
        if let Some(ref app_handle) = self.app_handle {
            let title = if enabled { "✓ 开机启动" } else { "开机启动" };
            let _ = app_handle.tray_handle().get_item("startup").set_title(title);
        }
    }

    /// 显示托盘通知
    pub fn show_notification(&self, title: &str, message: &str) {
        if let Some(ref app_handle) = self.app_handle {
            let _ = app_handle.emit_all("show-notification", serde_json::json!({
                "title": title,
                "message": message
            }));
        }
    }
}

impl Default for TrayManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_manager_creation() {
        let tray_manager = TrayManager::new();
        assert!(tray_manager.app_handle.is_none());
    }

    #[test]
    fn test_status_formatting() {
        let tray_manager = TrayManager::new();
        
        let status_ac = BatteryStatus {
            is_charging: false,
            is_ac_connected: true,
            battery_percentage: 85,
            is_battery_present: true,
            power_draw_watts: Some(12.5),
            battery_capacity_mwh: Some(50000),
            remaining_time_minutes: Some(240),
            charge_rate_watts: Some(0.0),
        };
        
        let text = tray_manager.format_status_text(&status_ac);
        assert!(text.contains("电源适配器"));
        assert!(text.contains("85%"));
        
        let status_battery = BatteryStatus {
            is_charging: true,
            is_ac_connected: false,
            battery_percentage: 45,
            is_battery_present: true,
            power_draw_watts: Some(18.0),
            battery_capacity_mwh: Some(50000),
            remaining_time_minutes: Some(120),
            charge_rate_watts: Some(20.0),
        };
        
        let text = tray_manager.format_status_text(&status_battery);
        assert!(text.contains("电池"));
        assert!(text.contains("45%"));
        assert!(text.contains("充电中"));
    }

    #[test]
    fn test_icon_selection() {
        let tray_manager = TrayManager::new();
        
        let status_ac = BatteryStatus {
            is_charging: false,
            is_ac_connected: true,
            battery_percentage: 85,
            is_battery_present: true,
            power_draw_watts: Some(12.5),
            battery_capacity_mwh: Some(50000),
            remaining_time_minutes: Some(240),
            charge_rate_watts: Some(0.0),
        };
        
        let icon_path = tray_manager.get_icon_for_status(&status_ac);
        assert!(icon_path.contains("connected"));
        
        let status_battery = BatteryStatus {
            is_charging: false,
            is_ac_connected: false,
            battery_percentage: 45,
            is_battery_present: true,
            power_draw_watts: Some(18.0),
            battery_capacity_mwh: Some(50000),
            remaining_time_minutes: Some(120),
            charge_rate_watts: Some(0.0),
        };
        
        let icon_path = tray_manager.get_icon_for_status(&status_battery);
        assert!(icon_path.contains("battery"));
    }
}