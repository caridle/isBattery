@echo off
chcp 65001 >nul 2>&1
setlocal enabledelayedexpansion
echo ================================
echo isBattery 构建脚本
echo ================================
echo.

REM 检查Rust是否安装
rustc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误：未找到Rust编译器
    echo 请先安装Rust: https://rustup.rs/
    pause
    exit /b 1
)

echo 检测到Rust编译器...
rustc --version

echo.
echo 开始构建isBattery...
echo.

REM 检查依赖
echo 正在检查依赖...
cargo check
if %errorlevel% neq 0 (
    echo 错误：依赖检查失败
    pause
    exit /b 1
)

REM 运行测试
echo.
echo 正在运行测试...
cargo test
if %errorlevel% neq 0 (
    echo 警告：部分测试失败，但继续构建...
)

REM 构建发布版本
echo.
echo 正在构建发布版本...
cargo build --release
if %errorlevel% neq 0 (
    echo 错误：构建失败
    pause
    exit /b 1
)

echo.
echo ================================
echo 构建完成！
echo ================================
echo.
echo 可执行文件位置：
echo target\release\isbattery.exe
echo.
echo 如需构建Tauri应用程序包，请安装Tauri CLI：
echo cargo install tauri-cli
echo 然后运行：cargo tauri build
echo.
pause