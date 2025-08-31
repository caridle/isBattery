use crate::power::{PowerDetector, BatteryStatus, PowerEvent};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

pub struct PowerMonitor {
    detector: PowerDetector,
    check_interval: Duration,
    low_battery_threshold: u8,
    is_monitoring: Arc<Mutex<bool>>,
    last_status: Arc<Mutex<Option<BatteryStatus>>>,
}

#[derive(Debug, Clone)]
pub struct MonitorEvent {
    pub power_event: PowerEvent,
    pub current_status: BatteryStatus,
}

impl PowerMonitor {
    pub fn new(check_interval_secs: u64, low_battery_threshold: u8) -> Self {
        Self {
            detector: PowerDetector::new(),
            check_interval: Duration::from_secs(check_interval_secs),
            low_battery_threshold,
            is_monitoring: Arc::new(Mutex::new(false)),
            last_status: Arc::new(Mutex::new(None)),
        }
    }

    /// 开始监控电源状态
    pub async fn start_monitoring(&self) -> mpsc::Receiver<MonitorEvent> {
        let (tx, rx) = mpsc::channel(100);
        
        {
            let mut monitoring = self.is_monitoring.lock().unwrap();
            *monitoring = true;
        }

        let detector = self.detector.clone();
        let check_interval = self.check_interval;
        let low_battery_threshold = self.low_battery_threshold;
        let is_monitoring = Arc::clone(&self.is_monitoring);
        let last_status = Arc::clone(&self.last_status);

        tokio::spawn(async move {
            let mut interval = time::interval(check_interval);
            
            loop {
                interval.tick().await;
                
                // 检查是否应该继续监控
                {
                    let monitoring = is_monitoring.lock().unwrap();
                    if !*monitoring {
                        break;
                    }
                }

                // 获取当前电源状态
                match detector.get_power_status() {
                    Ok(current_status) => {
                        let previous_status = {
                            let mut last_status_guard = last_status.lock().unwrap();
                            let prev = last_status_guard.clone();
                            *last_status_guard = Some(current_status.clone());
                            prev
                        };
                        
                        if let Some(previous_status) = previous_status {
                            // 检测状态变化
                            let events = detector.detect_power_events(
                                &previous_status,
                                &current_status,
                                low_battery_threshold
                            );

                            // 发送事件
                            for event in events {
                                let monitor_event = MonitorEvent {
                                    power_event: event,
                                    current_status: current_status.clone(),
                                };

                                if let Err(_) = tx.send(monitor_event).await {
                                    // 接收器已关闭，停止监控
                                    let mut monitoring = is_monitoring.lock().unwrap();
                                    *monitoring = false;
                                    break;
                                }
                            }
                            
                            // 如果电量或功耗发生变化（即使没有触发事件），也发送一个状态更新事件
                            // 这确保提醒窗口和托盘菜单中的信息始终保持最新
                            let power_changed = previous_status.power_draw_watts != current_status.power_draw_watts;
                            let percentage_changed = previous_status.battery_percentage != current_status.battery_percentage;
                            
                            if percentage_changed || power_changed {
                                let status_update_event = MonitorEvent {
                                    power_event: crate::power::PowerEvent::StatusUpdate,
                                    current_status: current_status.clone(),
                                };
                                
                                if let Err(_) = tx.send(status_update_event).await {
                                    let mut monitoring = is_monitoring.lock().unwrap();
                                    *monitoring = false;
                                    break;
                                }
                            }
                        } else {
                            // 首次检测，检查是否需要立即显示提醒
                            let (should_alert, _, _) = detector.should_show_alert(
                                &current_status,
                                low_battery_threshold
                            );

                            if should_alert {
                                // 根据状态决定事件类型
                                let event = if current_status.battery_percentage <= low_battery_threshold {
                                    PowerEvent::BatteryLow(current_status.battery_percentage)
                                } else if !current_status.is_ac_connected {
                                    PowerEvent::AcDisconnected
                                } else {
                                    continue;
                                };

                                let monitor_event = MonitorEvent {
                                    power_event: event,
                                    current_status: current_status.clone(),
                                };

                                if let Err(_) = tx.send(monitor_event).await {
                                    let mut monitoring = is_monitoring.lock().unwrap();
                                    *monitoring = false;
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error getting power status: {}", e);
                    }
                }
            }
        });

        rx
    }

    /// 停止监控
    #[allow(dead_code)]
    pub fn stop_monitoring(&self) {
        let mut monitoring = self.is_monitoring.lock().unwrap();
        *monitoring = false;
    }

    /// 暂停监控
    #[allow(dead_code)]
    pub fn pause_monitoring(&self) {
        self.stop_monitoring();
    }

    /// 恢复监控
    #[allow(dead_code)]
    pub async fn resume_monitoring(&self) -> mpsc::Receiver<MonitorEvent> {
        self.start_monitoring().await
    }

    /// 检查当前是否正在监控
    #[allow(dead_code)]
    pub fn is_monitoring(&self) -> bool {
        *self.is_monitoring.lock().unwrap()
    }

    /// 获取当前电源状态
    #[allow(dead_code)]
    pub fn get_current_status(&self) -> Result<BatteryStatus, String> {
        self.detector.get_power_status()
    }

    /// 检查是否应该显示提醒
    #[allow(dead_code)]
    pub fn should_show_alert(&self, status: &BatteryStatus) -> (bool, String, String) {
        self.detector.should_show_alert(status, self.low_battery_threshold)
    }

    /// 更新低电量阈值
    #[allow(dead_code)]
    pub fn set_low_battery_threshold(&mut self, threshold: u8) {
        self.low_battery_threshold = threshold;
    }

    /// 更新检测间隔
    #[allow(dead_code)]
    pub fn set_check_interval(&mut self, interval_secs: u64) {
        self.check_interval = Duration::from_secs(interval_secs);
    }
}

impl Clone for PowerDetector {
    fn clone(&self) -> Self {
        PowerDetector::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_monitor_creation() {
        let monitor = PowerMonitor::new(10, 20);
        assert!(!monitor.is_monitoring());
    }

    #[tokio::test]
    async fn test_power_monitor_start_stop() {
        let monitor = PowerMonitor::new(1, 20);
        
        // 开始监控
        let _rx = monitor.start_monitoring().await;
        assert!(monitor.is_monitoring());
        
        // 停止监控
        monitor.stop_monitoring();
        
        // 给一些时间让监控循环停止
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!monitor.is_monitoring());
    }
}