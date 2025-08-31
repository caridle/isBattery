use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use chrono::{DateTime, Local};

pub struct Logger {
    log_path: PathBuf,
    enabled: bool,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Debug => write!(f, "DEBUG"),
        }
    }
}

impl Logger {
    /// 创建日志记录器
    pub fn new(enabled: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let log_dir = dirs::config_dir()
            .or_else(|| dirs::home_dir().map(|p| p.join(".config")))
            .ok_or("Could not determine config directory")?
            .join("isBattery");

        // 确保日志目录存在
        if !log_dir.exists() {
            std::fs::create_dir_all(&log_dir)?;
        }

        let log_path = log_dir.join("app.log");

        Ok(Self {
            log_path,
            enabled,
        })
    }

    /// 记录日志
    pub fn log(&self, level: LogLevel, message: &str) {
        if !self.enabled {
            return;
        }

        let now: DateTime<Local> = Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let log_entry = format!("[{}] [{}] {}\n", timestamp, level, message);

        // 写入文件
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path) 
        {
            let _ = file.write_all(log_entry.as_bytes());
        }

        // 同时输出到控制台（在调试模式下）
        #[cfg(debug_assertions)]
        print!("{}", log_entry);
    }

    /// 记录信息日志
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    /// 记录警告日志
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warning, message);
    }

    /// 记录错误日志
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    /// 记录调试日志
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 设置启用状态
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 获取日志文件路径
    pub fn get_log_path(&self) -> &PathBuf {
        &self.log_path
    }

    /// 读取日志内容
    pub fn read_log(&self) -> Result<String, Box<dyn std::error::Error>> {
        std::fs::read_to_string(&self.log_path).map_err(|e| e.into())
    }

}

impl Default for Logger {
    fn default() -> Self {
        Self::new(true).unwrap_or_else(|_| {
            Self {
                log_path: PathBuf::from("app.log"),
                enabled: false,
            }
        })
    }
}

/// 全局日志记录器实例
use std::sync::{Mutex, OnceLock};

static GLOBAL_LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

/// 初始化全局日志记录器
pub fn init_logger(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new(enabled)?;
    GLOBAL_LOGGER.set(Mutex::new(logger))
        .map_err(|_| "Failed to initialize global logger")?;
    Ok(())
}

/// 获取全局日志记录器
pub fn get_logger() -> Option<&'static Mutex<Logger>> {
    GLOBAL_LOGGER.get()
}

/// 便捷的日志记录宏
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::utils::get_logger() {
            if let Ok(logger) = logger.lock() {
                logger.info(&format!($($arg)*));
            }
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::utils::get_logger() {
            if let Ok(logger) = logger.lock() {
                logger.warn(&format!($($arg)*));
            }
        }
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::utils::get_logger() {
            if let Ok(logger) = logger.lock() {
                logger.error(&format!($($arg)*));
            }
        }
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::utils::get_logger() {
            if let Ok(logger) = logger.lock() {
                logger.debug(&format!($($arg)*));
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_logger_creation() {
        let logger = Logger::new(true);
        match logger {
            Ok(logger) => {
                assert!(logger.is_enabled());
                println!("Logger created at: {:?}", logger.get_log_path());
            }
            Err(e) => println!("Logger creation failed: {}", e),
        }
    }

    #[test]
    fn test_logging() {
        let logger = Logger::new(true).unwrap();
        
        logger.info("Test info message");
        logger.warn("Test warning message");
        logger.error("Test error message");
        logger.debug("Test debug message");
        
        // 测试读取日志
        if let Ok(content) = logger.read_log() {
            assert!(content.contains("Test info message"));
            assert!(content.contains("Test warning message"));
            assert!(content.contains("Test error message"));
            assert!(content.contains("Test debug message"));
        }
    }

    #[test]
    fn test_log_levels() {
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Warning.to_string(), "WARN");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
    }

    #[test]
    fn test_logger_enable_disable() {
        let mut logger = Logger::new(false).unwrap();
        assert!(!logger.is_enabled());
        
        logger.set_enabled(true);
        assert!(logger.is_enabled());
        
        logger.set_enabled(false);
        assert!(!logger.is_enabled());
    }
}