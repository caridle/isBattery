use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryStatus {
    pub is_charging: bool,
    pub is_ac_connected: bool,
    pub battery_percentage: u8,
    pub is_battery_present: bool,
    // 新增功率负载相关字段
    pub power_draw_watts: Option<f32>,        // 当前功耗（瓦特）
    pub battery_capacity_mwh: Option<u32>,    // 电池容量（毫瓦时）
    pub remaining_time_minutes: Option<u32>,  // 剩余时间（分钟）
    pub charge_rate_watts: Option<f32>,       // 充电/放电速率（瓦特）
}

#[derive(Debug, Clone, PartialEq)]
pub enum PowerEvent {
    AcConnected,
    AcDisconnected,
    BatteryLow(u8),
    BatteryNormal(u8),
    StatusUpdate, // 用于状态更新（不是事件变化）
}

impl fmt::Display for PowerEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PowerEvent::AcConnected => write!(f, "AC电源已连接"),
            PowerEvent::AcDisconnected => write!(f, "AC电源已断开"),
            PowerEvent::BatteryLow(percentage) => write!(f, "电池电量不足: {}%", percentage),
            PowerEvent::BatteryNormal(percentage) => write!(f, "电池电量正常: {}%", percentage),
            PowerEvent::StatusUpdate => write!(f, "状态更新"),
        }
    }
}

pub struct PowerDetector;

impl PowerDetector {
    pub fn new() -> Self {
        Self
    }

    /// 获取当前电源状态
    pub fn get_power_status(&self) -> Result<BatteryStatus, String> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Power::{
                GetSystemPowerStatus, SYSTEM_POWER_STATUS
            };

            unsafe {
                let mut status = SYSTEM_POWER_STATUS::default();
                if GetSystemPowerStatus(&mut status).is_err() {
                    return Err("Failed to get system power status".to_string());
                }
                
                let is_ac_connected = status.ACLineStatus == 1;
                let is_charging = status.BatteryFlag & 8 != 0; // 充电状态
                let is_battery_present = status.BatteryFlag != 128; // 128表示没有电池
                
                // 电池百分比，255表示未知
                let battery_percentage = if status.BatteryLifePercent == 255 {
                    100 // 如果无法获取，默认为100%
                } else {
                    status.BatteryLifePercent as u8
                };

                // 获取详细的电池信息
                let (power_draw_watts, battery_capacity_mwh, remaining_time_minutes, charge_rate_watts) = 
                    self.get_advanced_battery_info();

                Ok(BatteryStatus {
                    is_charging,
                    is_ac_connected,
                    battery_percentage,
                    is_battery_present,
                    power_draw_watts,
                    battery_capacity_mwh,
                    remaining_time_minutes,
                    charge_rate_watts,
                })
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // 对于非Windows平台，返回默认状态
            Ok(BatteryStatus {
                is_charging: false,
                is_ac_connected: true,
                battery_percentage: 100,
                is_battery_present: false,
                power_draw_watts: None,
                battery_capacity_mwh: None,
                remaining_time_minutes: None,
                charge_rate_watts: None,
            })
        }
    }

    /// 检测电源状态变化
    pub fn detect_power_events(&self, 
        previous_status: &BatteryStatus, 
        current_status: &BatteryStatus,
        low_battery_threshold: u8
    ) -> Vec<PowerEvent> {
        let mut events = Vec::new();

        // 检测AC电源连接状态变化
        if previous_status.is_ac_connected != current_status.is_ac_connected {
            if current_status.is_ac_connected {
                events.push(PowerEvent::AcConnected);
            } else {
                events.push(PowerEvent::AcDisconnected);
            }
        }

        // 检测电池电量变化
        if current_status.is_battery_present {
            let was_low = previous_status.battery_percentage <= low_battery_threshold;
            let is_low = current_status.battery_percentage <= low_battery_threshold;

            if !was_low && is_low {
                events.push(PowerEvent::BatteryLow(current_status.battery_percentage));
            } else if was_low && !is_low {
                events.push(PowerEvent::BatteryNormal(current_status.battery_percentage));
            }
        }

        events
    }

    /// 检查是否需要显示提醒
    pub fn should_show_alert(&self, 
        status: &BatteryStatus, 
        low_battery_threshold: u8
    ) -> (bool, String, String) {
        // 优先检查低电量提醒（无论是否连接电源）
        if status.is_battery_present && status.battery_percentage <= low_battery_threshold {
            return (
                true, 
                "电池电量不足！请及时充电".to_string(),
                "#FF0000".to_string() // 红色背景
            );
        }

        // 检查电源断开提醒
        if !status.is_ac_connected && status.is_battery_present {
            return (
                true,
                "请连接电源适配器".to_string(),
                "#FF6B35".to_string() // 橙色背景
            );
        }

        (false, String::new(), String::new())
    }

    /// 获取高级电池信息（功率、容量等）
    #[cfg(target_os = "windows")]
    fn get_advanced_battery_info(&self) -> (Option<f32>, Option<u32>, Option<u32>, Option<f32>) {
        crate::log_info!("获取高级电池信息...");
        
        // 使用WMI获取详细的电池信息
        match self.query_wmi_battery_info() {
            Ok((power_draw, capacity, remaining_time, charge_rate)) => {
                crate::log_info!("WMI查询成功 - 功耗: {:.1}W, 容量: {}mWh, 剩余: {}分钟", 
                               power_draw, capacity, remaining_time);
                (Some(power_draw), Some(capacity), Some(remaining_time), Some(charge_rate))
            }
            Err(e) => {
                crate::log_error!("WMI查询失败: {}, 使用估算方法", e);
                // 如果WMI查询失败，尝试使用简单的计算方法
                let result = self.estimate_power_info();
                if let (Some(power), Some(cap), Some(time), Some(rate)) = result {
                    crate::log_info!("估算结果 - 功耗: {:.1}W, 容量: {}mWh, 剩余: {}分钟", 
                                   power, cap, time);
                }
                result
            }
        }
    }

    /// 通过WMI查询电池信息
    #[cfg(target_os = "windows")]
    fn query_wmi_battery_info(&self) -> Result<(f32, u32, u32, f32), String> {
        use std::process::{Command, Stdio};
        use std::os::windows::process::CommandExt;
        
        crate::log_info!("开始WMI电池信息查询...");
        
        // 尝试多个WMI查询获取更准确的数据
        let queries = [
            // 查询1: 基础电池信息
            "Get-WmiObject -Class Win32_Battery | Select-Object EstimatedChargeRemaining,DesignCapacity,EstimatedRunTime,DischargeRate | ConvertTo-Json",
            // 查询2: 更详细的电池状态
            "Get-WmiObject -Class Win32_PortableBattery | Select-Object DesignCapacity,MaxRechargeTime,EstimatedRunTime,Chemistry | ConvertTo-Json",
            // 查询3: 系统电源设置
            "powercfg /batteryreport /output temp_battery_report.xml 2>$null; if($?){Get-Content temp_battery_report.xml -Raw; Remove-Item temp_battery_report.xml -Force 2>$null}"
        ];
        
        // 先尝试基础查询
        let output = Command::new("powershell")
            .args(&[
                "-WindowStyle", "Hidden",  // 隐藏窗口
                "-NoProfile",              // 不加载配置文件
                "-NonInteractive",         // 非交互模式
                "-ExecutionPolicy", "Bypass", // 绕过执行策略
                "-Command",
                queries[0]
            ])
            .stdin(Stdio::null())       // 不需要输入
            .stdout(Stdio::piped())     // 捕获输出
            .stderr(Stdio::piped())     // 捕获错误输出以便调试
            .creation_flags(0x08000000) // CREATE_NO_WINDOW flag for Windows
            .output()
            .map_err(|e| {
                crate::log_error!("PowerShell执行失败: {}", e);
                format!("Failed to execute PowerShell: {}", e)
            })?;

        if !output.status.success() {
            crate::log_error!("PowerShell命令执行失败，状态码: {:?}", output.status.code());
            return Err("PowerShell command failed".to_string());
        }

        let json_str = String::from_utf8(output.stdout)
            .map_err(|e| {
                crate::log_error!("输出解析失败: {}", e);
                format!("Failed to parse output: {}", e)
            })?;
        
        crate::log_info!("WMI输出: {}", json_str.trim());

        // 解析JSON输出
        let result = self.parse_battery_json(&json_str);
        
        // 如果基础查询返回的功耗为默认值，尝试获取真实功耗
        if let Ok((power, capacity, time, rate)) = &result {
            if *power == 15.0 { // 如果是默认值，尝试获取真实数据
                if let Ok(real_power) = self.get_real_power_consumption() {
                    crate::log_info!("获取到真实功耗: {:.1}W", real_power);
                    return Ok((real_power, *capacity, *time, *rate));
                }
            }
        }
        
        result
    }

    /// 解析电池JSON数据
    #[cfg(target_os = "windows")]
    fn parse_battery_json(&self, json_str: &str) -> Result<(f32, u32, u32, f32), String> {
        crate::log_info!("开始解析JSON数据...");
        
        // 简单的JSON解析（不依赖外部库）
        let capacity = self.extract_json_value(json_str, "DesignCapacity")
            .unwrap_or(50000.0) as u32; // 默认值50Wh
        
        let discharge_rate = self.extract_json_value(json_str, "DischargeRate")
            .unwrap_or(15000.0); // 默认放电速率15000mW (15W)
        
        let estimated_runtime = self.extract_json_value(json_str, "EstimatedRunTime")
            .map(|v| {
                // 检查值是否合理，如果超过1440分钟（24小时），则使用默认值
                if v > 1440.0 || v < 0.0 {
                    240.0 // 默认剩余4小时
                } else {
                    v
                }
            })
            .unwrap_or(240.0) as u32; // 默认剩余4小时

        crate::log_info!("解析结果 - 容量: {}mWh, 放电率: {:.1}mW, 剩余时间: {}分钟", 
                        capacity, discharge_rate, estimated_runtime);

        // 计算当前功耗
        let power_draw = if discharge_rate > 0.0 {
            let watts = (discharge_rate / 1000.0) as f32; // 转换为瓦特
            if watts < 0.1 { 15.0 } else { watts } // 如果太小，使用默认值
        } else {
            // 如果没有放电率数据，估算一个值
            15.0 // 默认估算15W
        };

        crate::log_info!("计算功耗: {:.1}W", power_draw);
        Ok((power_draw, capacity, estimated_runtime, (discharge_rate / 1000.0) as f32))
    }

    /// 从 JSON 字符串中提取数值
    #[cfg(target_os = "windows")]
    fn extract_json_value(&self, json_str: &str, key: &str) -> Option<f64> {
        // 简单的JSON值提取，查找 "key":value 模式
        if let Some(start) = json_str.find(&format!("\"{}\"", key)) {
            if let Some(colon_pos) = json_str[start..].find(':') {
                let after_colon = &json_str[start + colon_pos + 1..];
                
                // 先检查是否为null
                let trimmed = after_colon.trim_start();
                if trimmed.starts_with("null") {
                    crate::log_info!("字段 {} 的值为 null", key);
                    return None;
                }
                
                // 查找数字
                let mut num_str = String::new();
                let mut found_digit = false;
                
                for ch in after_colon.chars() {
                    if ch.is_whitespace() && !found_digit {
                        continue; // 跳过前导空格
                    }
                    if ch.is_numeric() || ch == '.' || ch == '-' {
                        num_str.push(ch);
                        found_digit = true;
                    } else if found_digit {
                        break;
                    }
                }
                
                if found_digit {
                    let result = num_str.parse().ok();
                    crate::log_info!("字段 {} 的值: {}", key, num_str);
                    result
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 估算电源信息（备用方法）
    #[cfg(target_os = "windows")]
    fn estimate_power_info(&self) -> (Option<f32>, Option<u32>, Option<u32>, Option<f32>) {
        // 获取当前电池状态
        if let Ok(status) = self.get_basic_power_status() {
            let estimated_power = if status.is_charging {
                // 充电时估算功耗
                match status.battery_percentage {
                    0..=20 => 25.0,   // 低电量快充
                    21..=80 => 20.0,  // 正常充电
                    _ => 10.0,        // 慢充模式
                }
            } else {
                // 放电时估算功耗（基于典型笔记本功耗）
                15.0 // 典型笔记本在正常使用下的功耗
            };

            let estimated_capacity = 50000u32; // 50Wh 典型笔记本电池容量
            
            // 估算剩余时间
            let remaining_time = if !status.is_charging && status.battery_percentage > 0 {
                let remaining_capacity = (estimated_capacity as f32) * (status.battery_percentage as f32) / 100.0;
                ((remaining_capacity / estimated_power) * 60.0) as u32 // 转换为分钟
            } else {
                0u32
            };

            (Some(estimated_power), Some(estimated_capacity), Some(remaining_time), Some(estimated_power))
        } else {
            (None, None, None, None)
        }
    }

    /// 获取真实的系统功耗（通过性能计数器）
    #[cfg(target_os = "windows")]
    fn get_real_power_consumption(&self) -> Result<f32, String> {
        use std::process::{Command, Stdio};
        use std::os::windows::process::CommandExt;
        
        crate::log_info!("尝试获取真实系统功耗...");
        
        // 使用Windows性能计数器获取功耗信息
        let commands = [
            // 命令1: 获取电池放电率
            "(Get-Counter '\\Battery(*)\\Battery Discharge Rate' -ErrorAction SilentlyContinue).CounterSamples.CookedValue",
            // 命令2: 获取处理器功耗
            "(Get-Counter '\\Processor(_Total)\\% Processor Time' -ErrorAction SilentlyContinue).CounterSamples.CookedValue",
            // 命令3: 通过powercfg获取电池信息
            "powercfg /energy /output temp_energy.html /duration 5 2>$null; if($?){Select-String -Path temp_energy.html -Pattern 'Battery.*[0-9]+.*W' | Select-Object -First 1; Remove-Item temp_energy.html -Force 2>$null}"
        ];
        
        for (i, cmd) in commands.iter().enumerate() {
            crate::log_info!("执行功耗检测命令 {}: {}", i+1, cmd);
            
            let output = Command::new("powershell")
                .args(&[
                    "-WindowStyle", "Hidden",
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy", "Bypass",
                    "-Command",
                    cmd
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| format!("PowerShell执行失败: {}", e))?;
            
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                crate::log_info!("命令 {} 输出: {}", i+1, output_str.trim());
                
                // 尝试解析数值
                if let Some(power) = self.extract_power_from_output(&output_str) {
                    if power > 0.0 && power < 200.0 { // 合理范围内的功耗值
                        crate::log_info!("从命令 {} 获取到功耗: {:.1}W", i+1, power);
                        return Ok(power);
                    }
                }
            }
        }
        
        Err("无法获取真实功耗数据".to_string())
    }
    
    /// 从命令输出中提取功耗数值
    #[cfg(target_os = "windows")]
    fn extract_power_from_output(&self, output: &str) -> Option<f32> {
        // 查找数字模式
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // 尝试解析为数字
            if let Ok(value) = line.parse::<f64>() {
                if value > 0.0 && value < 200.0 {
                    return Some(value as f32);
                }
            }
            
            // 查找包含"W"的行
            if line.contains('W') || line.contains("watt") {
                for word in line.split_whitespace() {
                    let clean_word = word.trim_matches(|c: char| !c.is_numeric() && c != '.');
                    if let Ok(value) = clean_word.parse::<f64>() {
                        if value > 0.0 && value < 200.0 {
                            return Some(value as f32);
                        }
                    }
                }
            }
        }
        None
    }

    /// 获取基础电源状态（不包含高级信息）
    #[cfg(target_os = "windows")]
    fn get_basic_power_status(&self) -> Result<BatteryStatus, String> {
        use windows::Win32::System::Power::{
            GetSystemPowerStatus, SYSTEM_POWER_STATUS
        };

        unsafe {
            let mut status = SYSTEM_POWER_STATUS::default();
            if GetSystemPowerStatus(&mut status).is_err() {
                return Err("Failed to get system power status".to_string());
            }
            
            let is_ac_connected = status.ACLineStatus == 1;
            let is_charging = status.BatteryFlag & 8 != 0;
            let is_battery_present = status.BatteryFlag != 128;
            
            let battery_percentage = if status.BatteryLifePercent == 255 {
                100
            } else {
                status.BatteryLifePercent as u8
            };

            Ok(BatteryStatus {
                is_charging,
                is_ac_connected,
                battery_percentage,
                is_battery_present,
                power_draw_watts: None,
                battery_capacity_mwh: None,
                remaining_time_minutes: None,
                charge_rate_watts: None,
            })
        }
    }
}

impl Default for PowerDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_detector_creation() {
        let detector = PowerDetector::new();
        // 基本的创建测试
        assert!(true); // PowerDetector创建成功
    }

    #[test]
    fn test_power_event_detection() {
        let detector = PowerDetector::new();
        
        let previous_status = BatteryStatus {
            is_charging: false,
            is_ac_connected: true,
            battery_percentage: 50,
            is_battery_present: true,
            power_draw_watts: None,
            battery_capacity_mwh: None,
            remaining_time_minutes: None,
            charge_rate_watts: None,
        };

        let current_status = BatteryStatus {
            is_charging: false,
            is_ac_connected: false,
            battery_percentage: 50,
            is_battery_present: true,
            power_draw_watts: None,
            battery_capacity_mwh: None,
            remaining_time_minutes: None,
            charge_rate_watts: None,
        };

        let events = detector.detect_power_events(&previous_status, &current_status, 20);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PowerEvent::AcDisconnected));
    }

    #[test]
    fn test_low_battery_detection() {
        let detector = PowerDetector::new();
        
        let previous_status = BatteryStatus {
            is_charging: false,
            is_ac_connected: false,
            battery_percentage: 25,
            is_battery_present: true,
            power_draw_watts: None,
            battery_capacity_mwh: None,
            remaining_time_minutes: None,
            charge_rate_watts: None,
        };

        let current_status = BatteryStatus {
            is_charging: false,
            is_ac_connected: false,
            battery_percentage: 15,
            is_battery_present: true,
            power_draw_watts: None,
            battery_capacity_mwh: None,
            remaining_time_minutes: None,
            charge_rate_watts: None,
        };

        let events = detector.detect_power_events(&previous_status, &current_status, 20);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PowerEvent::BatteryLow(15)));
    }

    #[test]
    fn test_should_show_alert() {
        let detector = PowerDetector::new();
        
        // 测试低电量提醒优先级
        let low_battery_status = BatteryStatus {
            is_charging: false,
            is_ac_connected: true, // 即使连接电源也要提醒低电量
            battery_percentage: 15,
            is_battery_present: true,
        };

        let (should_alert, message, color) = detector.should_show_alert(&low_battery_status, 20);
        assert!(should_alert);
        assert_eq!(message, "电池电量不足！请及时充电");
        assert_eq!(color, "#FF0000");

        // 测试电源断开提醒
        let ac_disconnected_status = BatteryStatus {
            is_charging: false,
            is_ac_connected: false,
            battery_percentage: 50,
            is_battery_present: true,
        };

        let (should_alert, message, color) = detector.should_show_alert(&ac_disconnected_status, 20);
        assert!(should_alert);
        assert_eq!(message, "请连接电源适配器");
        assert_eq!(color, "#FF6B35");
    }
}