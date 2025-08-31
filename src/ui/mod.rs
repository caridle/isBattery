pub mod tray;
pub mod alert;
pub mod settings;

pub use tray::*;
pub use alert::*;
// settings模块中的函数通过ui::settings::路径在main.rs中被调用，不需要重新导出