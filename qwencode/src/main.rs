use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use walkdir::WalkDir;

/// 支持的模型提供商
#[derive(Debug, Clone, Copy)]
enum Provider {
    Ollama,
    OpenAI,
    Anthropic,
    Gemini,
    DeepSeek,
    Moonshot,
    Zhipu,
    Qwen,  // 通义千问 / 百炼
}

impl Provider {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ollama" => Some(Provider::Ollama),
            "openai" | "gpt" => Some(Provider::OpenAI),
            "anthropic" | "claude" => Some(Provider::Anthropic),
            "gemini" | "google" => Some(Provider::Gemini),
            "deepseek" => Some(Provider::DeepSeek),
            "moonshot" | "kimi" => Some(Provider::Moonshot),
            "zhipu" | "glm" => Some(Provider::Zhipu),
            "qwen" | "aliyun" | "dashscope" | "百炼" => Some(Provider::Qwen),
            _ => None,
        }
    }

    fn api_key_env(&self) -> &'static str {
        match self {
            Provider::Ollama => "",
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::Gemini => "GEMINI_API_KEY",
            Provider::DeepSeek => "DEEPSEEK_API_KEY",
            Provider::Moonshot => "MOONSHOT_API_KEY",
            Provider::Zhipu => "ZHIPU_API_KEY",
            Provider::Qwen => "DASHSCOPE_API_KEY",
        }
    }

    fn base_url(&self) -> &'static str {
        match self {
            Provider::Ollama => "http://localhost:11434",
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Provider::DeepSeek => "https://api.deepseek.com",
            Provider::Moonshot => "https://api.moonshot.cn/v1",
            Provider::Zhipu => "https://open.bigmodel.cn/api/paas/v4",
            Provider::Qwen => "https://coding.dashscope.aliyuncs.com/v1",
        }
    }

    fn default_model(&self) -> &'static str {
        match self {
            Provider::Ollama => "qwen2.5-coder:7b",
            Provider::OpenAI => "gpt-4o-mini",
            Provider::Anthropic => "claude-sonnet-4-20250514",
            Provider::Gemini => "gemini-2.0-flash",
            Provider::DeepSeek => "deepseek-coder",
            Provider::Moonshot => "moonshot-v1-8k",
            Provider::Zhipu => "code-glm",
            Provider::Qwen => "qwen3.5-plus",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

struct Config {
    provider: Provider,
    api_key: Option<String>,
    model: String,
    base_url: Option<String>,
    working_dir: String,
    yolo: bool, // YOLO 模式：跳过确认
}

/// 项目上下文信息
#[derive(Debug, Clone)]
struct ProjectContext {
    name: String,
    project_type: String,
    files: Vec<String>,
    dependencies: Vec<String>,
    entry_points: Vec<String>,
    structure: HashMap<String, usize>,
}

/// QwenCode 风格命令
enum Command {
    Help,
    Chat(String),
    Explain(String),
    Generate(String),
    Refactor(String),
    Debug(String),
    Test(String),
    Review(String),
    Search(String),
    Context,
    Edit(String),
    Apply(String),
    Run(String),
    Fix(String),
    Doc(String),
    Quit,
}

fn print_banner() {
    println!(r#"
╔═══════════════════════════════════════════════════════════╗
║                    QwenCode CLI                           ║
║              AI-Powered Coding Assistant                  ║
╚═══════════════════════════════════════════════════════════╝

Type /help for available commands, /quit to exit.
"#);
}

fn print_banner_yolo() {
    println!(r#"
╔═══════════════════════════════════════════════════════════╗
║                    QwenCode CLI                           ║
║              AI-Powered Coding Assistant                  ║
╠═══════════════════════════════════════════════════════════╣
║  ⚡ YOLO MODE ENABLED - Auto-execute without confirmation ║
╚═══════════════════════════════════════════════════════════╝

Type /help for available commands, /quit to exit.
"#);
}

fn print_help() {
    println!(r#"
Available Commands:
  /help, /h              Show this help message
  /quit, /exit, /q       Exit the program

  /chat <message>        General chat about coding
  /explain <code>        Explain code
  /generate <desc>       Generate code from description
  /refactor <code>       Suggest refactoring
  /debug <error>         Help debug an error
  /test <code>           Generate tests for code
  /review <code>         Code review
  
  /context, /ctx         Analyze and show project context
  /edit <file>           Edit a file (AI generates patch)
  /apply <patch>         Apply a patch file
  /run <cmd>, /! <cmd>   Run shell command
  /fix <error>, /f       Diagnose and fix error
  /doc <file>            Generate documentation
  
  /search <pattern>      Search in codebase
  /files                 List project files
  /pwd                   Show current directory

Slash commands can be abbreviated:
  /c = /chat, /e = /explain, /g = /generate
  /r = /refactor, /d = /debug, /t = /test
  /rev = /review, /s = /search, /ctx = /context
  /! = /run, /f = /fix
"#);
}

fn get_api_key(provider: Provider) -> Option<String> {
    match provider {
        Provider::Ollama => None,
        _ => env::var(provider.api_key_env()).ok(),
    }
}

fn chat(provider: Provider, config: &Config, messages: &[Message]) -> Result<String, String> {
    let api_key = config.api_key.as_ref();
    let base_url = config.base_url.as_deref().unwrap_or(provider.base_url());
    let model = &config.model;

    match provider {
        Provider::Ollama => {
            let url = format!("{}/api/chat", base_url);
            let body = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": false
            });

            let response = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_json(body)
                .map_err(|e| e.to_string())?;

            let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;
            json["message"]["content"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "Invalid response".to_string())
        }
        Provider::OpenAI | Provider::DeepSeek | Provider::Moonshot | Provider::Zhipu | Provider::Qwen => {
            let key = api_key.ok_or_else(|| format!("Set {}", provider.api_key_env()))?;
            let url = format!("{}/chat/completions", base_url);
            let body = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": false
            });

            let response = ureq::post(&url)
                .set("Authorization", &format!("Bearer {}", key))
                .set("Content-Type", "application/json")
                .send_json(body)
                .map_err(|e| e.to_string())?;

            let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;
            json["choices"][0]["message"]["content"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "Invalid response".to_string())
        }
        Provider::Anthropic => {
            let key = api_key.ok_or_else(|| format!("Set {}", provider.api_key_env()))?;
            let url = format!("{}/messages", base_url);
            let body = serde_json::json!({
                "model": model,
                "max_tokens": 4096,
                "messages": messages
            });

            let response = ureq::post(&url)
                .set("x-api-key", key)
                .set("anthropic-version", "2023-06-01")
                .set("Content-Type", "application/json")
                .send_json(body)
                .map_err(|e| e.to_string())?;

            let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;
            json["content"][0]["text"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "Invalid response".to_string())
        }
        Provider::Gemini => {
            let key = api_key.ok_or_else(|| format!("Set {}", provider.api_key_env()))?;
            let last = messages.last().ok_or_else(|| "No messages".to_string())?;
            let body = serde_json::json!({
                "contents": [{"parts": [{"text": last.content}]}]
            });

            let url = format!(
                "{}/models/{}:generateContent?key={}",
                base_url, model, key
            );

            let response = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_json(body)
                .map_err(|e| e.to_string())?;

            let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;
            json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "Invalid response".to_string())
        }
    }
}

/// 获取项目上下文（读取相关文件）
fn get_project_context(dir: &str, max_files: usize) -> String {
    let mut context = String::new();
    let mut file_count = 0;

    // 读取常见代码文件
    let extensions = ["rs", "py", "js", "ts", "go", "java", "cpp", "c", "h", "json", "toml", "yaml", "md"];
    
    for entry in WalkDir::new(dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if file_count >= max_files {
            break;
        }

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // 跳过隐藏目录和常见忽略目录
        if path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s.starts_with('.') || s == "target" || s == "node_modules" || s == ".git"
        }) {
            continue;
        }

        // 检查扩展名
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !extensions.contains(&ext) {
            continue;
        }

        // 读取文件内容
        if let Ok(content) = fs::read_to_string(path) {
            if content.len() < 10000 {
                context.push_str(&format!("\n\n// File: {}\n{}\n", path.display(), content));
                file_count += 1;
            }
        }
    }

    context
}

/// 分析项目上下文
fn analyze_project(dir: &str) -> ProjectContext {
    let mut files = Vec::new();
    let mut dependencies = Vec::new();
    let mut entry_points = Vec::new();
    let mut structure: HashMap<String, usize> = HashMap::new();
    let mut project_type = "unknown".to_string();
    let mut name = Path::new(dir).file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.to_string());

    let extensions = ["rs", "py", "js", "ts", "go", "java", "cpp", "c", "h", "json", "toml", "yaml", "md", "sh"];

    for entry in WalkDir::new(dir)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // 跳过隐藏目录和常见忽略目录
        if path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s.starts_with('.') && s != ".github" || s == "target" || s == "node_modules" || s == ".git" || s == "__pycache__"
        }) {
            continue;
        }

        // 检查项目配置文件
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        match file_name {
            "Cargo.toml" => {
                project_type = "rust".to_string();
                if let Ok(content) = fs::read_to_string(path) {
                    for line in content.lines() {
                        if line.starts_with("name = ") {
                            name = line.trim_start_matches("name = ").trim_matches('"').to_string();
                        }
                        if line.starts_with("[dependencies]") || line.starts_with("dependencies = ") {
                            break;
                        }
                        if line.contains('=') && !line.starts_with('[') && !line.starts_with('#') {
                            let dep = line.split('=').next().unwrap_or("").trim();
                            if !dep.is_empty() && !["name", "version", "edition"].contains(&dep) {
                                dependencies.push(dep.to_string());
                            }
                        }
                    }
                }
            }
            "package.json" => {
                project_type = "nodejs".to_string();
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(n) = json["name"].as_str() {
                            name = n.to_string();
                        }
                        if let Some(deps) = json["dependencies"].as_object() {
                            for dep in deps.keys() {
                                dependencies.push(dep.clone());
                            }
                        }
                    }
                }
            }
            "go.mod" => {
                project_type = "go".to_string();
                if let Ok(content) = fs::read_to_string(path) {
                    if let Some(first) = content.lines().skip(1).next() {
                        name = first.trim_start_matches("module ").trim().to_string();
                    }
                }
            }
            "main.rs" | "main.py" | "main.js" | "index.js" => {
                entry_points.push(path.display().to_string());
            }
            _ => {}
        }

        // 统计文件
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("no-ext");
        *structure.entry(ext.to_string()).or_insert(0) += 1;

        if extensions.contains(&ext) {
            if let Some(s) = path.strip_prefix(dir).ok().and_then(|p| p.to_str()) {
                files.push(s.to_string());
            }
        }
    }

    // 如果没有找到入口点，尝试查找
    if entry_points.is_empty() {
        for f in &files {
            if f.contains("main") || f.contains("index") || f.contains("app") {
                entry_points.push(f.clone());
            }
        }
    }

    ProjectContext {
        name,
        project_type,
        files,
        dependencies,
        entry_points,
        structure,
    }
}

/// 格式化项目上下文
fn format_project_context(ctx: &ProjectContext) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("📊 Project: {}\n", ctx.name));
    output.push_str(&format!("   Type: {}\n", ctx.project_type));
    output.push_str(&format!("   Files: {} total\n", ctx.files.len()));
    
    if !ctx.structure.is_empty() {
        output.push_str("   Structure:\n");
        let mut sorted: Vec<_> = ctx.structure.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (ext, count) in sorted.iter().take(10) {
            output.push_str(&format!("     .{}: {}\n", ext, count));
        }
    }
    
    if !ctx.dependencies.is_empty() {
        output.push_str(&format!("   Dependencies: {}\n", ctx.dependencies.join(", ")));
    }
    
    if !ctx.entry_points.is_empty() {
        output.push_str(&format!("   Entry points:\n"));
        for ep in &ctx.entry_points {
            output.push_str(&format!("     - {}\n", ep));
        }
    }
    
    output
}

/// 解析命令
fn parse_command(input: &str) -> Command {
    let input = input.trim();

    if input == "/help" || input == "/h" {
        Command::Help
    } else if input == "/quit" || input == "/exit" || input == "/q" {
        Command::Quit
    } else if input == "/context" || input == "/ctx" {
        Command::Context
    } else if let Some(path) = input.strip_prefix("/edit ").or_else(|| input.strip_prefix("/e ")) {
        Command::Edit(path.to_string())
    } else if let Some(path) = input.strip_prefix("/apply ") {
        Command::Apply(path.to_string())
    } else if let Some(cmd) = input.strip_prefix("/run ").or_else(|| input.strip_prefix("/! ")) {
        Command::Run(cmd.to_string())
    } else if let Some(err) = input.strip_prefix("/fix ").or_else(|| input.strip_prefix("/f ")) {
        Command::Fix(err.to_string())
    } else if let Some(file) = input.strip_prefix("/doc ") {
        Command::Doc(file.to_string())
    } else if let Some(msg) = input.strip_prefix("/chat ").or_else(|| input.strip_prefix("/c ")) {
        Command::Chat(msg.to_string())
    } else if let Some(code) = input.strip_prefix("/explain ").or_else(|| input.strip_prefix("/e ")) {
        Command::Explain(code.to_string())
    } else if let Some(desc) = input.strip_prefix("/generate ").or_else(|| input.strip_prefix("/g ")) {
        Command::Generate(desc.to_string())
    } else if let Some(code) = input.strip_prefix("/refactor ").or_else(|| input.strip_prefix("/r ")) {
        Command::Refactor(code.to_string())
    } else if let Some(err) = input.strip_prefix("/debug ").or_else(|| input.strip_prefix("/d ")) {
        Command::Debug(err.to_string())
    } else if let Some(code) = input.strip_prefix("/test ").or_else(|| input.strip_prefix("/t ")) {
        Command::Test(code.to_string())
    } else if let Some(code) = input.strip_prefix("/review ").or_else(|| input.strip_prefix("/rev ")) {
        Command::Review(code.to_string())
    } else if let Some(pattern) = input.strip_prefix("/search ").or_else(|| input.strip_prefix("/s ")) {
        Command::Search(pattern.to_string())
    } else if input == "/files" {
        Command::Search("".to_string())
    } else if input == "/pwd" {
        Command::Chat("pwd".to_string())
    } else {
        // 默认当作 chat 处理
        Command::Chat(input.to_string())
    }
}

/// 提取代码块中的代码
fn extract_code_block(response: &str) -> Option<String> {
    let lines: Vec<&str> = response.lines().collect();
    let mut in_code_block = false;
    let mut code_lines = Vec::new();
    let mut found_block = false;

    for line in lines {
        if line.trim().starts_with("```") {
            if in_code_block {
                // 结束代码块
                found_block = true;
                break;
            } else {
                // 开始代码块
                in_code_block = true;
                continue;
            }
        }
        if in_code_block {
            code_lines.push(line);
        }
    }

    if found_block && !code_lines.is_empty() {
        Some(code_lines.join("\n"))
    } else {
        None
    }
}

/// 构建系统提示词
fn build_system_prompt() -> String {
    "You are QwenCode, an AI-powered coding assistant. You help developers with:
- Writing and explaining code
- Debugging and fixing errors
- Refactoring and improving code quality
- Generating tests
- Code reviews
- Answering technical questions

Always provide clear, concise, and accurate responses.
Include code examples when helpful.
Format code with proper indentation."
    .to_string()
}

fn interactive_mode(config: &Config) {
    if config.yolo {
        print_banner_yolo();
        println!("⚡ YOLO Mode: Active");
    } else {
        print_banner();
    }
    println!("Provider: {:?}, Model: {}", config.provider, config.model);
    println!("Working directory: {}", config.working_dir);
    println!();

    let mut messages: Vec<Message> = Vec::new();
    messages.push(Message {
        role: "system".to_string(),
        content: build_system_prompt(),
    });

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("🤖 > ");
        let _ = stdout.flush();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let command = parse_command(input);

        match command {
            Command::Help => {
                print_help();
            }
            Command::Quit => {
                println!("Goodbye!");
                break;
            }
            Command::Chat(msg) => {
                if msg == "pwd" {
                    println!("📁 {}", config.working_dir);
                    continue;
                }

                messages.push(Message {
                    role: "user".to_string(),
                    content: msg.to_string(),
                });

                print!("🤖 ");
                let _ = stdout.flush();

                match chat(config.provider, config, &messages) {
                    Ok(response) => {
                        println!("{}", response);
                        messages.push(Message {
                            role: "assistant".to_string(),
                            content: response,
                        });
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Command::Explain(code) => {
                let prompt = format!("Please explain this code:\n\n```{}\n```", code);
                messages.push(Message { role: "user".to_string(), content: prompt });

                match chat(config.provider, config, &messages) {
                    Ok(response) => {
                        println!("{}", response);
                        messages.push(Message { role: "assistant".to_string(), content: response });
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Command::Generate(desc) => {
                let prompt = format!("Generate code for: {}\n\nProvide the complete implementation.", desc);
                messages.push(Message { role: "user".to_string(), content: prompt });

                match chat(config.provider, config, &messages) {
                    Ok(response) => {
                        println!("{}", response);
                        messages.push(Message { role: "assistant".to_string(), content: response });
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Command::Refactor(code) => {
                let prompt = format!("Suggest improvements and refactoring for this code:\n\n```{}\n```", code);
                messages.push(Message { role: "user".to_string(), content: prompt });

                match chat(config.provider, config, &messages) {
                    Ok(response) => {
                        println!("{}", response);
                        messages.push(Message { role: "assistant".to_string(), content: response });
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Command::Debug(err) => {
                let prompt = format!("Help me debug this error:\n\n{}", err);
                messages.push(Message { role: "user".to_string(), content: prompt });

                match chat(config.provider, config, &messages) {
                    Ok(response) => {
                        println!("{}", response);
                        messages.push(Message { role: "assistant".to_string(), content: response });
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Command::Test(code) => {
                let prompt = format!("Generate comprehensive tests for this code:\n\n```{}\n```", code);
                messages.push(Message { role: "user".to_string(), content: prompt });

                match chat(config.provider, config, &messages) {
                    Ok(response) => {
                        println!("{}", response);
                        messages.push(Message { role: "assistant".to_string(), content: response });
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Command::Review(code) => {
                let prompt = format!("Review this code and provide feedback:\n\n```{}\n```", code);
                messages.push(Message { role: "user".to_string(), content: prompt });

                match chat(config.provider, config, &messages) {
                    Ok(response) => {
                        println!("{}", response);
                        messages.push(Message { role: "assistant".to_string(), content: response });
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Command::Search(pattern) => {
                if pattern.is_empty() {
                    // 列出项目文件
                    println!("📁 Project files:");
                    for entry in WalkDir::new(&config.working_dir)
                        .max_depth(2)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(s) = path.to_str() {
                                if !s.contains("/target/") && !s.contains("/.git/") {
                                    println!("  {}", s);
                                }
                            }
                        }
                    }
                } else {
                    // 搜索内容
                    println!("🔍 Searching for '{}'...", pattern);
                    for entry in WalkDir::new(&config.working_dir)
                        .max_depth(3)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        let path = entry.path();
                        if let Ok(content) = fs::read_to_string(path) {
                            if content.contains(&pattern) {
                                println!("  Found in: {}", path.display());
                            }
                        }
                    }
                }
            }
            Command::Context => {
                println!("🔍 Analyzing project...");
                let ctx = analyze_project(&config.working_dir);
                println!("{}", format_project_context(&ctx));
            }
            Command::Edit(file) => {
                println!("📝 Edit file: {}", file);

                // 读取文件内容
                match fs::read_to_string(&file) {
                    Ok(content) => {
                        println!("  File has {} bytes", content.len());

                        // 在 YOLO 模式下，直接请求 AI 生成改进建议并自动应用
                        if config.yolo {
                            println!("  ⚡ YOLO Mode: Requesting AI improvements...");

                            let edit_prompt = format!(
                                "Please review and improve this code. Provide the complete improved version in a code block.\n\nFile: {}\n\n```rust\n{}\n```",
                                file, content
                            );
                            messages.push(Message { role: "user".to_string(), content: edit_prompt });

                            match chat(config.provider, config, &messages) {
                                Ok(response) => {
                                    println!("\n🤖 AI Suggestions:\n");
                                    // 提取代码块
                                    let improved_code = extract_code_block(&response);
                                    if let Some(code) = improved_code {
                                        println!("{}", code);
                                        println!("\n💡 To apply: copy the code above, or use /apply with a patch file");
                                    } else {
                                        println!("{}", response);
                                    }
                                    messages.push(Message { role: "assistant".to_string(), content: response });
                                }
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        } else {
                            println!("  Use /refactor with the file content to suggest changes");
                        }
                    }
                    Err(e) => {
                        if config.yolo {
                            // YOLO 模式下创建新文件
                            println!("  ⚡ YOLO Mode: File doesn't exist, creating new file");
                            let create_prompt = format!(
                                "Generate a complete {} file with best practices. Provide only the code in a code block.",
                                Path::new(&file).extension().unwrap_or_default().to_string_lossy()
                            );
                            messages.push(Message { role: "user".to_string(), content: create_prompt });

                            match chat(config.provider, config, &messages) {
                                Ok(response) => {
                                    if let Some(code) = extract_code_block(&response) {
                                        match fs::write(&file, &code) {
                                            Ok(()) => println!("  ✅ Created {} with AI-generated content", file),
                                            Err(e) => eprintln!("  ❌ Error creating file: {}", e),
                                        }
                                        messages.push(Message { role: "assistant".to_string(), content: response });
                                    } else {
                                        println!("{}", response);
                                    }
                                }
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        } else {
                            eprintln!("  Error reading file: {}", e)
                        }
                    }
                }
            }
            Command::Apply(patch) => {
                println!("🔧 Apply patch: {}", patch);
                
                // 检查补丁文件是否存在
                if Path::new(&patch).exists() {
                    println!("  Patch file found");
                    println!("  Use: /root/patch/target/debug/patch {}", patch);
                } else {
                    eprintln!("  Patch file not found: {}", patch);
                }
            }
            Command::Run(cmd) => {
                if config.yolo {
                    println!("⚡ YOLO Mode: Executing without confirmation");
                }
                println!("🚀 Running: {}", cmd);

                // 执行 shell 命令
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .output();

                match output {
                    Ok(result) => {
                        let stdout = String::from_utf8_lossy(&result.stdout);
                        let stderr = String::from_utf8_lossy(&result.stderr);

                        if !stdout.is_empty() {
                            println!("  Output:");
                            for line in stdout.lines().take(50) {
                                println!("    {}", line);
                            }
                            if stdout.lines().count() > 50 {
                                println!("    ... (truncated, {} lines total)", stdout.lines().count());
                            }
                        }

                        if !stderr.is_empty() {
                            println!("  Errors:");
                            for line in stderr.lines().take(20) {
                                eprintln!("    {}", line);
                            }
                        }

                        if result.status.success() {
                            println!("  ✅ Exit code: 0");
                        } else {
                            println!("  ⚠️  Exit code: {:?}", result.status.code());
                        }
                    }
                    Err(e) => {
                        eprintln!("  ❌ Error executing command: {}", e);
                    }
                }
            }
            Command::Fix(error) => {
                println!("🔧 Diagnose error: {}", error);
                
                // 如果是文件路径，尝试读取错误
                if Path::new(&error).exists() {
                    match fs::read_to_string(&error) {
                        Ok(content) => {
                            println!("  Error file content ({} bytes):", content.len());
                            for line in content.lines().take(10) {
                                println!("    {}", line);
                            }
                            println!("  Use /refactor with this content to get fix suggestions");
                        }
                        Err(e) => eprintln!("  Error reading file: {}", e),
                    }
                } else {
                    // 直接分析错误信息
                    println!("  Analyzing error message...");
                    
                    // 常见错误模式匹配
                    let error_lower = error.to_lowercase();
                    if error_lower.contains("borrow of moved value") {
                        println!("  💡 Rust borrow checker error:");
                        println!("     The value was moved and cannot be borrowed");
                        println!("     Consider using .clone() or &reference");
                    } else if error_lower.contains("cannot find") || error_lower.contains("not found") {
                        println!("  💡 Name resolution error:");
                        println!("     Check imports and variable scope");
                        println!("     Use 'cargo check' for detailed diagnostics");
                    } else if error_lower.contains("expected") && error_lower.contains("found") {
                        println!("  💡 Type mismatch:");
                        println!("     Check function signatures and return types");
                        println!("     Consider type annotations");
                    } else {
                        println!("  Use /refactor or /explain with the code to get AI assistance");
                    }
                }
            }
            Command::Doc(file) => {
                println!("📚 Generate documentation for: {}", file);
                
                // 检查文件是否存在
                if Path::new(&file).exists() {
                    match fs::read_to_string(&file) {
                        Ok(content) => {
                            println!("  File: {} ({} bytes)", file, content.len());
                            println!("  Use /generate with 'Generate Rust docs for:' to create documentation");
                            
                            // 检测文件类型
                            let ext = Path::new(&file).extension().and_then(|e| e.to_str()).unwrap_or("");
                            match ext {
                                "rs" => println!("  Type: Rust source - use rustdoc style comments (///)"),
                                "py" => println!("  Type: Python source - use docstrings"),
                                "js" | "ts" => println!("  Type: JavaScript/TypeScript - use JSDoc comments"),
                                "go" => println!("  Type: Go source - use Go doc comments"),
                                _ => println!("  Type: {} source", ext),
                            }
                        }
                        Err(e) => eprintln!("  Error reading file: {}", e),
                    }
                } else {
                    eprintln!("  File not found: {}", file);
                }
            }
        }
    }
}

fn print_help_and_exit() {
    println!(r#"
QwenCode CLI - AI-Powered Coding Assistant

Usage: qwencode [OPTIONS]

Options:
  -p, --provider <PROVIDER>  Model provider (ollama, openai, anthropic, gemini, deepseek, moonshot, zhipu)
  -m, --model <MODEL>        Model name
  -i, --interactive          Interactive mode (REPL)
  -y, --yolo                 YOLO mode: auto-execute without confirmation
  --base-url <URL>           Custom API endpoint
  -h, --help                 Show this help

Environment Variables:
  OPENAI_API_KEY      OpenAI API Key
  ANTHROPIC_API_KEY   Anthropic API Key
  GEMINI_API_KEY      Google Gemini API Key
  DEEPSEEK_API_KEY    DeepSeek API Key
  MOONSHOT_API_KEY    Moonshot API Key
  ZHIPU_API_KEY       Zhipu API Key

Examples:
  qwencode -i                    # Start interactive mode with Ollama
  qwencode -p openai -i          # Start with OpenAI
  qwencode -p ollama -m qwen2.5-coder:7b -i
  qwencode -y -i                 # Start in YOLO mode (auto-execute)
"#);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 || args.iter().any(|a| a == "-h" || a == "--help") {
        print_help_and_exit();
        return;
    }

    let mut provider: Option<Provider> = None;
    let mut model: Option<String> = None;
    let mut base_url: Option<String> = None;
    let mut interactive = false;
    let mut yolo = false;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "-p" | "--provider" => {
                if i + 1 < args.len() {
                    i += 1;
                    provider = Provider::from_str(&args[i]);
                }
            }
            "-m" | "--model" => {
                if i + 1 < args.len() {
                    i += 1;
                    model = Some(args[i].clone());
                }
            }
            "--base-url" => {
                if i + 1 < args.len() {
                    i += 1;
                    base_url = Some(args[i].clone());
                }
            }
            "-y" | "--yolo" => {
                yolo = true;
            }
            "-i" | "--interactive" => {
                interactive = true;
            }
            "-h" | "--help" => {
                print_help_and_exit();
                return;
            }
            _ => {}
        }
        i += 1;
    }

    let provider = provider.unwrap_or(Provider::Ollama);
    let model = model.unwrap_or_else(|| provider.default_model().to_string());
    let api_key = get_api_key(provider);
    let working_dir = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let config = Config {
        provider,
        api_key,
        model,
        base_url,
        working_dir,
        yolo,
    };

    if interactive {
        interactive_mode(&config);
    }
}
