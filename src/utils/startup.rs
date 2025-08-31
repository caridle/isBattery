use auto_launch::AutoLaunch;

pub struct StartupManager {
    auto_launch: AutoLaunch,
}

impl StartupManager {
    /// 创建启动管理器
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let auto_launch = AutoLaunch::new(
            "isBattery",
            &std::env::current_exe()?.to_string_lossy(),
            &[] as &[&str],
        );

        Ok(Self { auto_launch })
    }

    /// 启用开机自启动
    pub fn enable(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.auto_launch.enable()?;
        Ok(())
    }

    /// 禁用开机自启动
    pub fn disable(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.auto_launch.disable()?;
        Ok(())
    }

    /// 检查是否已启用开机自启动
    pub fn is_enabled(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.auto_launch.is_enabled()?)
    }

    /// 切换开机自启动状态
    pub fn toggle(&self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.is_enabled()? {
            self.disable()?;
            Ok(false)
        } else {
            self.enable()?;
            Ok(true)
        }
    }

}

impl Default for StartupManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // 如果创建失败，返回一个无效的管理器
            // 这种情况下所有操作都会失败，但不会panic
            let auto_launch = AutoLaunch::new("isBattery", "", &[] as &[&str]);
            Self { auto_launch }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_manager_creation() {
        let manager = StartupManager::new();
        match manager {
            Ok(_) => println!("StartupManager created successfully"),
            Err(e) => println!("StartupManager creation failed: {}", e),
        }
    }

    #[test]
    fn test_startup_manager_operations() {
        let manager = StartupManager::default();
        
        // 测试检查状态（可能会失败，但不应该panic）
        match manager.is_enabled() {
            Ok(enabled) => println!("Startup is enabled: {}", enabled),
            Err(e) => println!("Failed to check startup status: {}", e),
        }
    }

    // 注意：实际的启用/禁用测试可能会修改系统设置，
    // 所以在单元测试中我们通常不会执行它们
    // 这些测试应该在集成测试或手动测试中进行
}