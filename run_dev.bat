@echo off
echo ================================
echo isBattery 开发模式
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

echo.
echo 启动开发模式...
echo 注意：首次运行可能需要下载依赖，请耐心等待
echo.

cargo run
if %errorlevel% neq 0 (
    echo.
    echo 运行失败，可能的原因：
    echo 1. 网络连接问题导致依赖下载失败
    echo 2. 编译错误
    echo 3. 缺少必要的系统组件
    echo.
    echo 尝试解决方案：
    echo 1. 检查网络连接
    echo 2. 运行 cargo clean 清理构建缓存
    echo 3. 重新运行此脚本
    echo.
)

pause