# Unified AI Coding Assistant

> 一个强大的统一 AI 编程助手，集成聊天、IDE 和工具管理功能于一体。

![GitHub Actions](https://github.com/looklee/rust-tool/actions/workflows/ci.yml/badge.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/language-Rust-orange.svg)

## 🚀 功能特性

### 🔮 多种模式
- **Chat Mode**: 简单快捷的 AI 聊天界面
- **IDE Mode**: 交互式代码开发环境
- **Manager Mode**: 工具自我进化管理系统

### 🤖 多 AI 提供商支持
- OpenAI (GPT-4, GPT-3.5)
- Anthropic (Claude)
- Gemini (Google)
- Ollama (本地模型)
- DeepSeek
- Moonshot (Kimi)
- Zhipu (GLM)
- Qwen (阿里百炼)
- MiniMax

### 📱 跨平台支持
- Linux
- macOS
- Windows
- **Termux (Android)**

## 📦 安装

### 一键运行

**Linux / Mac / Termux:**
```bash
git clone https://github.com/looklee/rust-tool.git
cd rust-tool
chmod +x run.sh
./run.sh
```

**Windows PowerShell:**
```powershell
git clone https://github.com/looklee/rust-tool.git
cd rust-tool
.\run.ps1
```

### 系统安装

```bash
git clone https://github.com/looklee/rust-tool.git
cd rust-tool
chmod +x install.sh
sudo ./install.sh
```

安装完成后可以直接使用 `code-ai` 命令。

## 🎯 使用方法

### 基础用法

```bash
# 启动聊天模式
code-ai

# 快速提问
code-ai '用 Rust 写一个 HTTP 服务器'

# IDE 模式
code-ai -i

# 工具管理模式
code-ai -M
```

### 命令行参数

```
Unified AI Coding Assistant

Usage: code-ai [OPTIONS] [COMMAND]

OPTIONS:
  -p, --provider <PROVIDER>  AI provider (ollama, openai, anthropic, gemini, deepseek, moonshot, zhipu, qwen)
  -m, --model <MODEL>       Model name
  --base-url <URL>          Custom API endpoint
  -y, --yolo                 YOLO mode: auto-apply changes

MODES:
  code-ai                    Chat mode (default)
  code-ai -i, --ide          IDE interactive mode
  code-ai -M, --manager      Tool manager & evolution mode

CHAT MODE:
  code-ai 'Hello'              Simple chat
  code-ai -i                   Interactive chat

IDE MODE (-i):
  :help, :open, :explain, :fix, :refactor, :test, :build, :run

MANAGER MODE (-M):
  code-ai -M list              List all tools
  code-ai -M scan              Scan and analyze all tools
  code-ai -M diagnose <tool>   Diagnose a specific tool
  code-ai -M diagnose-all      Diagnose all tools
  code-ai -M improve <tool>    AI-improve a tool
  code-ai -M evolve            Full AI-powered evolution
  code-ai -M create <name>     Create new tool with AI
  code-ai -M suggest           Suggest new tools
```

## 📁 项目结构

```
rust-tool/
├── common/              # 共享库
│   ├── src/
│   │   ├── ai.rs        # AI 客户端
│   │   ├── config/      # 配置系统
│   │   ├── errors/      # 错误处理
│   │   ├── logging/     # 日志系统
│   │   ├── termux/      # Termux 支持
│   │   └── utils/       # 工具函数
├── code/                # 统一 CLI 入口
├── 60+ 工具模块...      # 各种命令行工具
├── run.sh               # Linux/Mac/Termux 一键运行
├── run.ps1              # Windows 一键运行
└── install.sh           # 系统安装脚本
```

## 🛠️ 开发

### 构建

```bash
# 构建所有项目
cargo build --release

# 只构建核心模块
cargo build --release -p common
cargo build --release -p code
```

### 测试

```bash
# 运行测试
cargo test -p common
cargo test -p code
```

### 代码检查

```bash
# 格式化检查
cargo fmt --check

# 静态分析
cargo clippy
```

## ⚙️ 配置

配置文件位于 `~/.config/rust-tool/config.toml`：

```toml
[ai]
provider = "ollama"
model = "qwen2.5-coder:7b"
max_tokens = 4096
temperature = 0.7

[ui]
colors = true
verbose = false
yolo_mode = false

[logging]
level = "info"
format = "text"
```

## 🌐 环境变量

| 变量 | 说明 |
|------|------|
| `AI_PROVIDER` | 默认 AI 提供商 |
| `AI_MODEL` | 默认模型名称 |
| `AI_API_KEY` | API 密钥 |
| `AI_BASE_URL` | 自定义 API 地址 |
| `RUST_TOOL_ROOT` | 工具根目录 |

## 📱 Termux 支持

项目完全支持 Termux 环境：

```bash
# 在 Termux 中安装
pkg install rust git
git clone https://github.com/looklee/rust-tool.git
cd rust-tool
./run.sh
```

### Termux 特殊功能
- 自动检测 Termux 环境
- 访问 Android 外部存储
- Toast 通知支持
- 设备震动反馈

## 📜 许可证

MIT License - 详见 [LICENSE](LICENSE)

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📧 联系方式

如有问题或建议，请通过 GitHub Issues 联系。

---

**Built with ❤️ using Rust**