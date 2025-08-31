# 发布到 GitHub 的步骤指南

## 1. 准备工作

### 检查项目完整性
- [x] 代码已完成并测试
- [x] 文档已更新 (README.md, IMPLEMENTATION_SUMMARY.md, POWER_MONITORING.md)
- [x] .gitignore 文件已配置
- [x] LICENSE 文件已创建
- [x] CHANGELOG.md 已创建
- [x] GitHub Actions 工作流已配置

### 清理不必要的文件
```bash
# 清理构建产物
cargo clean

# 删除临时文件
del *.tmp
del *.log
```

## 2. Git 提交和推送

### 添加所有文件
```bash
git add .
```

### 提交更改
```bash
git commit -m "feat: 初始版本 - 电源监控工具 v0.1.0

✨ 新功能:
- 实时电源状态监控
- 功率负载监控和显示  
- 智能提醒系统
- 系统托盘集成
- 开机自启动支持
- 暂停/恢复监控

🔧 技术特性:
- Rust + Tauri 架构
- 多级功耗检测策略
- 异步监控系统
- 完整的错误处理

🐛 修复:
- 托盘功耗动态更新
- PowerShell 窗口闪现
- 开机启动菜单响应

📚 文档:
- 完整的使用说明
- 技术实现文档
- 功率监控详细说明"
```

## 3. 创建 GitHub 仓库

### 在 GitHub 网站上创建仓库
1. 登录 GitHub (https://github.com)
2. 点击右上角的 "+" 号，选择 "New repository"
3. 填写仓库信息：
   - Repository name: `isBattery`
   - Description: `基于 Rust + Tauri 的 Windows 电源监控工具`
   - 选择 Public 或 Private
   - 不要初始化 README（我们已经有了）
4. 点击 "Create repository"

### 连接本地仓库到 GitHub
```bash
# 添加远程仓库（替换为你的 GitHub 用户名）
git remote add origin https://github.com/你的用户名/isBattery.git

# 推送代码到 GitHub
git branch -M main
git push -u origin main
```

## 4. 创建首个发布版本

### 创建版本标签
```bash
# 创建并推送标签
git tag -a v0.1.0 -m "发布版本 v0.1.0

主要功能:
- 实时电源监控
- 功率负载显示
- 智能提醒系统
- 系统托盘集成
- 开机自启动

技术架构:
- Rust + Tauri
- 异步监控
- 多级功耗检测
- Windows API 集成"

git push origin v0.1.0
```

### GitHub Release（自动创建）
推送标签后，GitHub Actions 会自动：
1. 运行测试
2. 构建发布版本
3. 创建 GitHub Release
4. 上传安装包

## 5. 完善仓库信息

### 添加仓库描述和主题
在 GitHub 仓库页面：
1. 点击右上角的 ⚙️ 设置
2. 在 "About" 部分：
   - Description: `基于 Rust + Tauri 的 Windows 电源监控工具，支持实时功耗检测、智能提醒和系统托盘集成`
   - Website: （如果有）
   - Topics: `rust` `tauri` `windows` `battery-monitor` `power-management` `system-tray` `gui`

### 设置仓库 README 徽章
在 README.md 顶部添加状态徽章：

```markdown
# isBattery - 电源监控程序

[![Build Status](https://github.com/你的用户名/isBattery/workflows/Build%20and%20Release/badge.svg)](https://github.com/你的用户名/isBattery/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Release](https://img.shields.io/github/v/release/你的用户名/isBattery)](https://github.com/你的用户名/isBattery/releases)
[![Platform](https://img.shields.io/badge/platform-Windows-blue.svg)](https://github.com/你的用户名/isBattery)
```

## 6. 监控构建状态

### 检查 GitHub Actions
1. 进入仓库的 "Actions" 标签页
2. 查看工作流运行状态
3. 如果构建失败，检查日志并修复问题

### 验证发布
1. 进入 "Releases" 页面
2. 确认自动创建的发布版本
3. 下载测试安装包

## 7. 后续维护

### 更新代码时
```bash
# 正常提交
git add .
git commit -m "fix: 修复某个问题"
git push

# 发布新版本时
git tag -a v0.1.1 -m "版本 v0.1.1 - 修复版本"
git push origin v0.1.1
```

### 管理 Issues 和 PR
1. 启用 Issues 模板
2. 设置贡献指南
3. 配置自动标签

## 注意事项

1. **敏感信息**：确保没有提交密码、API密钥等敏感信息
2. **二进制文件**：不要提交大型二进制文件或构建产物
3. **许可证**：确保代码符合开源许可证要求
4. **文档**：保持文档与代码同步更新
5. **版本管理**：遵循语义化版本控制规范

## 推荐的仓库结构

```
isBattery/
├── .github/
│   └── workflows/
│       └── build.yml          # 自动构建
├── src/                       # 源代码
├── dist/                      # 前端资源
├── assets/                    # 静态资源
├── icons/                     # 图标文件
├── README.md                  # 项目说明
├── CHANGELOG.md               # 更新日志
├── LICENSE                    # 许可证
├── IMPLEMENTATION_SUMMARY.md  # 实现总结
├── POWER_MONITORING.md        # 功能文档
├── Cargo.toml                 # Rust 配置
├── tauri.conf.json           # Tauri 配置
├── .gitignore                # Git 忽略文件
├── build.bat                 # 构建脚本
└── run_dev.bat               # 开发脚本
```

现在你可以按照这个指南将项目发布到 GitHub 了！