# mysh - AI 优先的智能 Shell

mysh 是一个用 Rust 编写的下一代智能 Shell，将 AI 深度集成到命令行体验中。

## 🌟 独特价值

### 与传统 Shell 的区别

| 功能 | 传统 Shell | mysh |
|------|-----------|------|
| 命令输入 | 记忆命令语法 | 自然语言描述 |
| 错误处理 | 显示错误代码 | AI 诊断 + 修复建议 |
| 学习曲线 | 需要记忆 | 命令解释 + 推荐 |
| 上下文 | 无状态 | 理解项目/会话历史 |

## 🚀 AI 功能

### 1. 自然语言转命令 (`ask`)
```bash
# 不用记复杂命令，直接描述意图
$ ask 查找所有包含 "main" 的 Rust 文件
思考中... 
建议命令：grep -r "main" --include="*.rs" .
```

### 2. 命令解释 (`explain`)
```bash
# 理解复杂命令的作用
$ explain find . -name "*.rs" -exec grep -l "fn main" {} \;
分析中...
这个命令会：
1. 在当前目录及子目录查找所有 .rs 文件
2. 在每个文件中搜索 "fn main"
3. 只输出包含匹配的文件名
```

### 3. 错误诊断 (`fix`)
```bash
# 自动分析错误并给出修复方案
$ fix git push fatal: remote origin not found
诊断中...
错误原因：远程仓库 origin 未配置
修复建议：
1. 查看远程：git remote -v
2. 添加远程：git remote add origin <url>
3. 然后推送：git push -u origin main
```

## 🛠️ 核心功能

### Shell 基础
- 管道和重定向 (`|`, `>`, `>>`, `<`)
- 变量替换 (`$VAR`, `${VAR}`, `$$`, `$?`)
- 命令替换 (`$(cmd)`, `` `cmd` ``)
- 通配符展开 (`*`, `?`, `[...]`)
- 后台作业 (`&`, `jobs`, `fg`, `bg`)

### 脚本控制
- `if/then/else/fi` 条件判断
- `for/in/do/done` 循环
- `while/do/done` 循环
- `test/[ ]` 条件测试

### 开发工具集成
- Git 快捷命令 (`gstatus`, `gdiff`, `glog`)
- 代码分析 (`codeanalyze`)
- 文件搜索 (`find`, `grep`, `wc`)

### 个性化
- 可定制提示符 (PS1 with `\u`, `\h`, `\w`, etc.)
- 别名系统 (`alias`)
- 启动文件 (`~/.myshrc`)
- 命令历史 (`~/.mysh_history`)

## 📦 安装

### 从源码编译
```bash
cd /root/mysh
cargo build --release
./target/release/mysh
```

### 配置 AI 功能
```bash
# 设置 OpenAI API Key
export OPENAI_API_KEY=sk-...

# 或使用兼容的 API 服务
export OPENAI_BASE_URL=https://api.your-service.com
export LLM_MODEL=gpt-3.5-turbo
```

## 💡 使用场景

### 场景 1：查找文件
```bash
# 传统方式（需要记住命令）
$ find . -name "*.rs" -exec grep -l "fn main" {} \;

# mysh 方式（自然语言）
$ ask 查找所有有 main 函数的 Rust 文件
```

### 场景 2：Git 错误
```bash
# 传统方式（搜索错误信息）
$ git push
fatal: ...

# mysh 方式（自动诊断）
$ fix git push fatal: ...
```

### 场景 3：学习命令
```bash
# 理解复杂命令
$ explain tar -czvf archive.tar.gz --exclude='*.log' .
```

## 🔮 路线图

### v0.2 (计划中)
- [ ] 会话历史与上下文理解
- [ ] 本地 AI 模型支持 (Ollama)
- [ ] 智能命令补全
- [ ] 项目感知（理解 Cargo 项目结构）

### v0.3 (愿景)
- [ ] 语音交互
- [ ] 多步骤任务自动化
- [ ] Shell 脚本自动生成
- [ ] 协作分享（命令片段分享）

## 🤝 贡献

mysh 是一个开源项目，欢迎贡献！

## 📄 许可证

MIT License
