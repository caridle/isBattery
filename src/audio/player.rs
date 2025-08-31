use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[allow(dead_code)] // AudioPlayer为将来扩展而保留

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Sink,
}

#[allow(dead_code)] // AudioPlayer为将来扩展而保留
impl AudioPlayer {
    /// 创建音频播放器
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        
        Ok(Self {
            _stream: stream,
            sink,
        })
    }

    /// 播放音频文件
    pub fn play_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let decoder = Decoder::new(reader)?;
        
        self.sink.append(decoder);
        Ok(())
    }

    /// 播放系统警告音
    pub fn play_system_alert(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        {
            // 使用简单的 Beep 函数替代 MessageBeep
            println!("\x07"); // ASCII Bell character
        }

        #[cfg(not(target_os = "windows"))]
        {
            // 在非Windows系统上，我们可以尝试播放一个默认的beep sound
            println!("\x07"); // ASCII Bell character
        }

        Ok(())
    }

    /// 播放预设的提醒音
    pub fn play_alert_sound(&self, sound_type: AlertSoundType) -> Result<(), Box<dyn std::error::Error>> {
        match sound_type {
            AlertSoundType::SystemWarning => self.play_system_alert(),
            AlertSoundType::CustomFile(path) => {
                if Path::new(&path).exists() {
                    self.play_file(&path)
                } else {
                    // 如果自定义文件不存在，回退到系统警告音
                    self.play_system_alert()
                }
            }
            AlertSoundType::EmbeddedAlert => {
                // 如果有内嵌的警告音文件，在这里播放
                // 目前先使用系统警告音
                self.play_system_alert()
            }
        }
    }

    /// 停止播放
    pub fn stop(&self) {
        self.sink.stop();
    }

    /// 暂停播放
    pub fn pause(&self) {
        self.sink.pause();
    }

    /// 恢复播放
    pub fn resume(&self) {
        self.sink.play();
    }

    /// 检查是否正在播放
    pub fn is_playing(&self) -> bool {
        !self.sink.empty()
    }

    /// 设置音量 (0.0 - 1.0)
    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume.clamp(0.0, 1.0));
    }
}

#[allow(dead_code)] // 为将来扩展而保留的声音类型
#[derive(Debug, Clone)]
pub enum AlertSoundType {
    SystemWarning,
    CustomFile(String),
    EmbeddedAlert,
}

impl Default for AlertSoundType {
    fn default() -> Self {
        AlertSoundType::SystemWarning
    }
}

/// 音频管理器，负责管理应用程序的所有音频播放
/// 使用简化实现避免线程安全问题
pub struct AudioManager {
    enabled: bool,
    #[allow(dead_code)] // 为将来音量控制而保留
    volume: f32,
}

impl AudioManager {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            volume: 1.0,
        }
    }

    /// 播放提醒音
    pub fn play_alert(&self, _sound_type: AlertSoundType) -> Result<(), Box<dyn std::error::Error>> {
        if !self.enabled {
            return Ok(());
        }

        // 使用系统警告音
        self.play_system_alert()
    }

    /// 播放电源断开提醒音
    pub fn play_power_disconnected_alert(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.play_alert(AlertSoundType::SystemWarning)
    }

    /// 播放系统警告音
    fn play_system_alert(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        {
            // 使用简单的 Beep 函数替代 MessageBeep
            println!("\x07"); // ASCII Bell character
        }

        #[cfg(not(target_os = "windows"))]
        {
            // 在非Windows系统上，我们可以尝试播放一个默认的beep sound
            println!("\x07"); // ASCII Bell character
        }

        Ok(())
    }

    /// 播放低电量提醒音
    pub fn play_low_battery_alert(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.play_alert(AlertSoundType::SystemWarning)
    }

    /// 设置是否启用音频
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 测试音频播放
    pub fn test_audio(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.play_alert(AlertSoundType::SystemWarning)
    }
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_player_creation() {
        let player = AudioPlayer::new();
        // 音频设备可能不可用，所以我们只测试创建过程不会panic
        match player {
            Ok(_) => println!("Audio player created successfully"),
            Err(e) => println!("Audio player creation failed: {}", e),
        }
    }

    #[test]
    fn test_audio_manager() {
        let mut manager = AudioManager::new(true);
        assert!(manager.is_enabled());
        
        manager.set_enabled(false);
        assert!(!manager.is_enabled());
        
        manager.set_volume(0.5);
        assert_eq!(manager.get_volume(), 0.5);
        
        manager.set_volume(1.5); // 应该被限制在1.0
        assert_eq!(manager.get_volume(), 1.0);
        
        manager.set_volume(-0.1); // 应该被限制在0.0
        assert_eq!(manager.get_volume(), 0.0);
    }

    #[test]
    fn test_alert_sound_types() {
        let system_sound = AlertSoundType::SystemWarning;
        let custom_sound = AlertSoundType::CustomFile("test.wav".to_string());
        let embedded_sound = AlertSoundType::EmbeddedAlert;
        
        // 测试克隆
        let _cloned_system = system_sound.clone();
        let _cloned_custom = custom_sound.clone();
        let _cloned_embedded = embedded_sound.clone();
        
        // 测试默认值
        let default_sound = AlertSoundType::default();
        assert!(matches!(default_sound, AlertSoundType::SystemWarning));
    }

    #[test]
    fn test_system_alert_playback() {
        let manager = AudioManager::new(true);
        
        // 测试播放系统警告音（应该不会失败）
        let result = manager.play_power_disconnected_alert();
        match result {
            Ok(_) => println!("System alert played successfully"),
            Err(e) => println!("System alert playback failed: {}", e),
        }
        
        let result = manager.play_low_battery_alert();
        match result {
            Ok(_) => println!("Low battery alert played successfully"),
            Err(e) => println!("Low battery alert playback failed: {}", e),
        }
    }
}