// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod power;
mod config;
mod audio;
mod utils;
mod ui;

use config::ConfigManager;
use power::{PowerMonitor, MonitorEvent, PowerEvent, PowerDetector};
use audio::AudioManager;
use ui::{TrayManager, AlertManager};
use utils::{StartupManager, init_logger};

use std::sync::{Arc, Mutex};
use tauri::{
    AppHandle, Manager, WindowEvent, State
};
use tokio::sync::mpsc;

#[derive(Clone)]
struct AppState {
    config_manager: ConfigManager,
    audio_manager: Arc<Mutex<AudioManager>>,
    tray_manager: Arc<Mutex<TrayManager>>,
    alert_manager: Arc<Mutex<AlertManager>>,
    startup_manager: Arc<Mutex<StartupManager>>,
    monitoring_receiver: Arc<Mutex<Option<mpsc::Receiver<MonitorEvent>>>>,
}

impl AppState {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_manager = ConfigManager::new()?;
        let startup_manager = StartupManager::new()?;
        
        let monitoring_config = config_manager.get_monitoring_config();
        let audio_manager = AudioManager::new(monitoring_config.sound_enabled);

        Ok(Self {
            config_manager,
            audio_manager: Arc::new(Mutex::new(audio_manager)),
            tray_manager: Arc::new(Mutex::new(TrayManager::new())),
            alert_manager: Arc::new(Mutex::new(AlertManager::new())),
            startup_manager: Arc::new(Mutex::new(startup_manager)),
            monitoring_receiver: Arc::new(Mutex::new(None)),
        })
    }

    async fn start_monitoring(&self, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        // 获取监控配置
        let monitoring_config = self.config_manager.get_monitoring_config();
        
        // 创建新的监控器
        let monitor = PowerMonitor::new(
            monitoring_config.check_interval,
            monitoring_config.low_battery_threshold
        );

        // 启动监控并获取接收器
        let receiver = monitor.start_monitoring().await;
        
        {
            let mut receiver_guard = self.monitoring_receiver.lock().unwrap();
            *receiver_guard = Some(receiver);
        }

        // 更新托盘状态
        {
            let tray_manager = self.tray_manager.lock().unwrap();
            tray_manager.update_monitoring_status(true);
        }

        // 启动事件处理任务
        self.spawn_event_handler(app_handle).await;

        Ok(())
    }

    async fn stop_monitoring(&self) {
        // 清除接收器
        {
            let mut receiver_guard = self.monitoring_receiver.lock().unwrap();
            *receiver_guard = None;
        }

        // 更新托盘状态
        {
            let tray_manager = self.tray_manager.lock().unwrap();
            tray_manager.update_monitoring_status(false);
        }

        // 关闭所有提醒窗口
        {
            let mut alert_manager = self.alert_manager.lock().unwrap();
            let _ = alert_manager.close_all_alerts();
        }
    }

    async fn spawn_event_handler(&self, app_handle: AppHandle) {
        let audio_manager = Arc::clone(&self.audio_manager);
        let tray_manager = Arc::clone(&self.tray_manager);
        let alert_manager = Arc::clone(&self.alert_manager);
        let monitoring_receiver = Arc::clone(&self.monitoring_receiver);
        let config_manager = self.config_manager.clone();

        tokio::spawn(async move {
            while let Some(mut receiver) = {
                let mut guard = monitoring_receiver.lock().unwrap();
                guard.take()
            } {
                while let Some(event) = receiver.recv().await {
                    Self::handle_power_event(
                        &event,
                        &config_manager,
                        &audio_manager,
                        &tray_manager,
                        &alert_manager,
                        &app_handle
                    ).await;
                }

                // 将接收器放回去
                {
                    let mut guard = monitoring_receiver.lock().unwrap();
                    *guard = Some(receiver);
                }
            }
        });
    }

    async fn handle_power_event(
        event: &MonitorEvent,
        config_manager: &ConfigManager,
        audio_manager: &Arc<Mutex<AudioManager>>,
        tray_manager: &Arc<Mutex<TrayManager>>,
        alert_manager: &Arc<Mutex<AlertManager>>,
        _app_handle: &AppHandle
    ) {
        let current_status = &event.current_status;
        let power_event = &event.power_event;

        // 更新托盘状态
        {
            let tray_manager = tray_manager.lock().unwrap();
            tray_manager.update_status(current_status);
        }

        // 更新已打开的提醒窗口中的电量信息
        {
            let alert_manager = alert_manager.lock().unwrap();
            if let Err(e) = alert_manager.update_battery_percentage(current_status.battery_percentage) {
                log_error!("Failed to update battery percentage in alert windows: {}", e);
            }
        }

        // 处理不同类型的电源事件
        match power_event {
            PowerEvent::AcDisconnected => {
                log_info!("AC power disconnected, battery: {}%", current_status.battery_percentage);
                
                // 显示电源断开提醒
                {
                    let mut alert_manager = alert_manager.lock().unwrap();
                    if let Err(e) = alert_manager.show_power_disconnected_alert(current_status) {
                        log_error!("Failed to show power disconnected alert: {}", e);
                    }
                }

                // 播放提醒音
                {
                    let audio_manager = audio_manager.lock().unwrap();
                    if let Err(e) = audio_manager.play_power_disconnected_alert() {
                        log_error!("Failed to play alert sound: {}", e);
                    }
                }

                // 显示托盘通知
                {
                    let tray_manager = tray_manager.lock().unwrap();
                    tray_manager.show_notification(
                        "电源提醒",
                        &format!("电源已断开，当前电量：{}%", current_status.battery_percentage)
                    );
                }
            }
            PowerEvent::AcConnected => {
                log_info!("AC power connected, battery: {}%", current_status.battery_percentage);
                
                // 如果设置了自动关闭提醒，则关闭相关提醒窗口
                let monitoring_config = config_manager.get_monitoring_config();
                if monitoring_config.auto_close_alert {
                    let mut alert_manager = alert_manager.lock().unwrap();
                    let _ = alert_manager.close_alert("power_disconnected");
                }

                // 显示托盘通知
                {
                    let tray_manager = tray_manager.lock().unwrap();
                    tray_manager.show_notification(
                        "电源提醒",
                        "电源已连接"
                    );
                }
            }
            PowerEvent::BatteryLow(percentage) => {
                log_info!("Low battery warning: {}%", percentage);
                
                // 显示低电量提醒（优先级高，即使连接电源也显示）
                {
                    let mut alert_manager = alert_manager.lock().unwrap();
                    if let Err(e) = alert_manager.show_low_battery_alert(current_status) {
                        log_error!("Failed to show low battery alert: {}", e);
                    }
                }

                // 播放提醒音
                {
                    let audio_manager = audio_manager.lock().unwrap();
                    if let Err(e) = audio_manager.play_low_battery_alert() {
                        log_error!("Failed to play alert sound: {}", e);
                    }
                }

                // 显示托盘通知
                {
                    let tray_manager = tray_manager.lock().unwrap();
                    tray_manager.show_notification(
                        "电量不足",
                        &format!("电池电量不足：{}%，请及时充电！", percentage)
                    );
                }
            }
            PowerEvent::BatteryNormal(percentage) => {
                log_info!("Battery level normal: {}%", percentage);
                
                // 关闭低电量提醒
                {
                    let mut alert_manager = alert_manager.lock().unwrap();
                    let _ = alert_manager.close_alert("low_battery");
                }

                // 显示托盘通知
                {
                    let tray_manager = tray_manager.lock().unwrap();
                    tray_manager.show_notification(
                        "电源提醒",
                        &format!("电池电量恢复正常：{}%", percentage)
                    );
                }
            }
            PowerEvent::StatusUpdate => {
                // 状态更新事件，不需要特殊处理，因为托盘和提醒窗口已经更新
                // log_info!("Status update: battery {}%", current_status.battery_percentage);
            }
        }
    }
}

// Tauri 命令
#[tauri::command]
async fn pause_monitoring(app_state: State<'_, AppState>) -> Result<(), String> {
    app_state.stop_monitoring().await;
    log_info!("Monitoring paused by user");
    Ok(())
}

#[tauri::command]
async fn resume_monitoring(app_handle: AppHandle, app_state: State<'_, AppState>) -> Result<(), String> {
    app_state.start_monitoring(app_handle).await.map_err(|e| e.to_string())?;
    log_info!("Monitoring resumed by user");
    Ok(())
}

#[tauri::command]
async fn toggle_startup(app_state: State<'_, AppState>) -> Result<bool, String> {
    let startup_manager = app_state.startup_manager.lock().unwrap();
    let enabled = startup_manager.toggle().map_err(|e| e.to_string())?;
    
    // 更新配置
    app_state.config_manager.set_auto_startup(enabled).map_err(|e| e.to_string())?;
    
    // 更新托盘菜单
    {
        let tray_manager = app_state.tray_manager.lock().unwrap();
        tray_manager.update_startup_menu(enabled);
    }
    
    log_info!("Auto startup toggled: {}", enabled);
    Ok(enabled)
}

#[tauri::command]
async fn get_current_power_status() -> Result<power::BatteryStatus, String> {
    let detector = PowerDetector::new();
    detector.get_power_status()
}

#[tauri::command]
async fn debug_power_status() -> Result<String, String> {
    let detector = PowerDetector::new();
    match detector.get_power_status() {
        Ok(status) => {
            let mut debug_info = format!(
                "调试信息:\n电池存在: {}\n电源连接: {}\n充电状态: {}\n电池电量: {}%",
                status.is_battery_present,
                status.is_ac_connected,
                status.is_charging,
                status.battery_percentage
            );
            
            // 添加功率信息
            if let Some(power_watts) = status.power_draw_watts {
                debug_info.push_str(&format!("\n当前功耗: {:.1}W", power_watts));
            }
            
            if let Some(capacity_mwh) = status.battery_capacity_mwh {
                debug_info.push_str(&format!("\n电池容量: {:.1}Wh", capacity_mwh as f32 / 1000.0));
            }
            
            if let Some(remaining_min) = status.remaining_time_minutes {
                if remaining_min > 0 {
                    let hours = remaining_min / 60;
                    let minutes = remaining_min % 60;
                    debug_info.push_str(&format!("\n剩余时间: {}h{}m", hours, minutes));
                }
            }
            
            if let Some(charge_rate) = status.charge_rate_watts {
                if charge_rate != 0.0 {
                    debug_info.push_str(&format!("\n充电速率: {:.1}W", charge_rate));
                }
            }
            
            Ok(debug_info)
        }
        Err(e) => Err(format!("获取电源状态失败: {}", e))
    }
}

#[tauri::command]
async fn get_power_info() -> Result<serde_json::Value, String> {
    let detector = PowerDetector::new();
    match detector.get_power_status() {
        Ok(status) => {
            let power_info = serde_json::json!({
                "battery_percentage": status.battery_percentage,
                "is_charging": status.is_charging,
                "is_ac_connected": status.is_ac_connected,
                "power_draw_watts": status.power_draw_watts,
                "battery_capacity_mwh": status.battery_capacity_mwh,
                "remaining_time_minutes": status.remaining_time_minutes,
                "charge_rate_watts": status.charge_rate_watts
            });
            Ok(power_info)
        }
        Err(e) => Err(format!("获取电源信息失败: {}", e))
    }
}

/// 测试WMI查询命令
#[tauri::command]
fn test_wmi_query() -> Result<String, String> {
    let detector = PowerDetector::new();
    match detector.get_power_status() {
        Ok(status) => {
            Ok(format!("电池状态: 电量 {}%, 功耗 {:?}W, 剩余时间 {:?}分钟", 
                      status.battery_percentage, 
                      status.power_draw_watts, 
                      status.remaining_time_minutes))
        }
        Err(e) => Err(format!("获取电池状态失败: {}", e))
    }
}

fn main() {
    // 初始化日志记录器
    if let Err(e) = init_logger(true) {
        eprintln!("Failed to initialize logger: {}", e);
    }

    log_info!("isBattery application starting");

    // 创建应用状态
    let app_state = match AppState::new() {
        Ok(state) => state,
        Err(e) => {
            log_error!("Failed to initialize app state: {}", e);
            std::process::exit(1);
        }
    };

    // 创建系统托盘
    let tray = TrayManager::create_system_tray();

    // 克隆状态用于不同的生命周期
    let app_state_setup = app_state.clone();
    let app_state_manage = app_state.clone();

    tauri::Builder::default()
        .system_tray(tray)
        .on_system_tray_event(|app, event| {
            TrayManager::handle_tray_event(app, event);
        })
        .setup(move |app| {
            let app_handle = app.handle();
            
            // 设置应用句柄到各个管理器
            {
                let mut tray_manager = app_state_setup.tray_manager.lock().unwrap();
                tray_manager.set_app_handle(app_handle.clone());
            }
            
            {
                let mut alert_manager = app_state_setup.alert_manager.lock().unwrap();
                alert_manager.set_app_handle(app_handle.clone());
            }

            // 初始化开机自启动状态（以系统实际状态为准）
            if let Ok(startup_manager) = app_state_setup.startup_manager.lock() {
                if let Ok(system_enabled) = startup_manager.is_enabled() {
                    // 同步系统状态到配置文件
                    let _ = app_state_setup.config_manager.set_auto_startup(system_enabled);
                    
                    // 更新托盘菜单显示
                    let tray_manager = app_state_setup.tray_manager.lock().unwrap();
                    tray_manager.update_startup_menu(system_enabled);
                    
                    log_info!("Startup status synchronized: {}", system_enabled);
                } else {
                    log_error!("Failed to get startup status from system");
                }
            }

            // 初始化电源状态显示
            {
                let detector = PowerDetector::new();
                if let Ok(current_status) = detector.get_power_status() {
                    let tray_manager = app_state_setup.tray_manager.lock().unwrap();
                    tray_manager.update_status(&current_status);
                    log_info!("Initial power status: AC connected: {}, Battery: {}%", 
                             current_status.is_ac_connected, current_status.battery_percentage);
                } else {
                    log_error!("Failed to get initial power status");
                }
            }

            // 启动电源监控
            let app_handle_clone = app_handle.clone();
            let app_state_clone = app_state_setup.clone();
            
            // 使用 app.app_handle() 来确保在 Tauri 运行时上下文中启动
            tauri::async_runtime::spawn(async move {
                if let Err(e) = app_state_clone.start_monitoring(app_handle_clone).await {
                    log_error!("Failed to start monitoring: {}", e);
                }
            });

            // 监听应用事件
            let app_state_clone = app_state_setup.clone();
            app.listen_global("pause-monitoring", move |_| {
                let app_state = app_state_clone.clone();
                tauri::async_runtime::spawn(async move {
                    app_state.stop_monitoring().await;
                });
            });

            let app_handle_clone = app_handle.clone();
            let app_state_clone = app_state_setup.clone();
            app.listen_global("resume-monitoring", move |_| {
                let app_handle = app_handle_clone.clone();
                let app_state = app_state_clone.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = app_state.start_monitoring(app_handle).await {
                        log_error!("Failed to resume monitoring: {}", e);
                    }
                });
            });

            let app_state_clone = app_state_setup.clone();
            app.listen_global("toggle-startup", move |_| {
                let app_state = app_state_clone.clone();
                tauri::async_runtime::spawn(async move {
                    if let Ok(startup_manager) = app_state.startup_manager.lock() {
                        match startup_manager.toggle() {
                            Ok(enabled) => {
                                let _ = app_state.config_manager.set_auto_startup(enabled);
                                let tray_manager = app_state.tray_manager.lock().unwrap();
                                tray_manager.update_startup_menu(enabled);
                                log_info!("Auto startup toggled: {}", enabled);
                            }
                            Err(e) => log_error!("Failed to toggle startup: {}", e),
                        }
                    }
                });
            });

            let app_state_clone = app_state_setup.clone();
            app.listen_global("config-updated", move |_| {
                let app_state = app_state_clone.clone();
                tauri::async_runtime::spawn(async move {
                    // 重新加载配置
                    let config = app_state.config_manager.get_monitoring_config();
                    
                    // 更新音频管理器
                    {
                        let mut audio_manager = app_state.audio_manager.lock().unwrap();
                        audio_manager.set_enabled(config.sound_enabled);
                    }
                    
                    log_info!("Configuration updated");
                });
            });

            Ok(())
        })
        .on_window_event(|event| {
            if let WindowEvent::CloseRequested { api, .. } = event.event() {
                // 阻止窗口关闭，改为隐藏到托盘
                api.prevent_close();
                event.window().hide().unwrap();
            }
        })
        .manage(app_state_manage.config_manager.clone())
        .manage(app_state_manage.clone())
        .invoke_handler(tauri::generate_handler![
            ui::settings::get_settings,
            ui::settings::save_settings,
            ui::settings::reset_settings,
            ui::settings::validate_settings,
            ui::settings::export_settings,
            ui::settings::import_settings,
            ui::settings::get_config_file_path,
            ui::settings::open_config_directory,
            ui::settings::test_audio_alert,
            ui::alert::close_alert_window,
            ui::alert::pause_monitoring_from_alert,
            ui::alert::get_alert_config,
            pause_monitoring,
            resume_monitoring,
            toggle_startup,
            get_current_power_status,
            debug_power_status,
            get_power_info,
            test_wmi_query
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    log_info!("isBattery application stopped");
}