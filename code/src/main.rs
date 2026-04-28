use common::{
    AiClient, AiConfig, AiProvider, Colors, EvolutionEngine, EvolutionReport,
    EvolutionPriority, Message, extract_code_block,
};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

enum Mode {
    Chat,
    Ide,
    Manager,
}

fn print_banner(mode: &Mode, config: &AiConfig, colors: &Colors) {
    let title = match mode {
        Mode::Chat => "Chat Mode",
        Mode::Ide => "Code IDE Mode",
        Mode::Manager => "🧬 Tool Evolution Mode",
    };

    println!("{}", colors.blue());
    println!(r#"╔═══════════════════════════════════════════════════════════╗
║              Unified AI Coding Assistant                  ║
║                  All-in-One Solution                      ║
╚═══════════════════════════════════════════════════════════╝"#);
    println!("{}", colors.reset());
    println!("Mode: {}", title);
    println!("Provider: {:?}, Model: {}", config.provider, config.model);
    println!();
}

fn print_help(colors: &Colors) {
    println!("{}", colors.cyan());
    println!(r#"Unified AI Coding Assistant

Usage: code [OPTIONS] [COMMAND]

OPTIONS:
  -p, --provider <PROVIDER>  AI provider (ollama, openai, anthropic, gemini, deepseek, moonshot, zhipu, qwen)
  -m, --model <MODEL>       Model name
  --base-url <URL>          Custom API endpoint
  -y, --yolo                 YOLO mode: auto-apply changes

MODES:
  code                       Chat mode (default)
  code -i, --ide            IDE interactive mode
  code -M, --manager        Tool manager & evolution mode

CHAT MODE:
  code 'Hello'              Simple chat
  code -i                   Interactive chat

IDE MODE (-i):
  :help, :open, :explain, :fix, :refactor, :test, :build, :run

MANAGER MODE (-M):
  code -M list              List all tools
  code -M scan              Scan and analyze all tools
  code -M diagnose <tool>   Diagnose a specific tool
  code -M diagnose-all      Diagnose all tools
  code -M improve <tool>    AI-improve a tool
  code -M evolve            Full AI-powered evolution
  code -M create <name>     Create new tool with AI
  code -M suggest           Suggest new tools

EXAMPLES:
  code '用 Rust 写一个 HTTP 服务器'
  code -i
  code -M scan
  code -M evolve
  code -M create jsontool "JSON processing tool"
"#);
    println!("{}", colors.reset());
}

fn chat_mode(ai_client: &AiClient, prompt: &str, colors: &Colors) {
    let messages = vec![Message::user(prompt)];
    print!("{}AI> {}", colors.magenta(), colors.reset());

    match ai_client.chat(&messages) {
        Ok(response) => println!("{}", response),
        Err(e) => eprintln!("{}Error: {}{}", colors.red(), e, colors.reset()),
    }
}

fn interactive_chat_mode(ai_client: &AiClient, colors: &Colors) {
    println!("{}Interactive Chat Mode (input /quit to exit){}", colors.blue(), colors.reset());
    println!("Provider: {:?}, Model: {}", ai_client.config.provider, ai_client.config.model);
    println!();

    let mut messages: Vec<Message> = Vec::new();

    loop {
        print!("{}User> {}", colors.green(), colors.reset());

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/quit" || input == "/exit" {
            println!("{}Goodbye!{}", colors.green(), colors.reset());
            break;
        }

        messages.push(Message::user(input));
        print!("{}AI> {}", colors.magenta(), colors.reset());

        match ai_client.chat(&messages) {
            Ok(response) => {
                println!("{}", response);
                messages.push(Message::assistant(&response));
            }
            Err(e) => {
                eprintln!("{}Error: {}{}", colors.red(), e, colors.reset());
                messages.pop();
            }
        }
    }
}

fn ide_mode(ai_client: &AiClient, colors: &Colors) {
    print_banner(&Mode::Ide, &ai_client.config, colors);

    let mut messages: Vec<Message> = Vec::new();
    messages.push(Message::system("你是一位专业的编程助手，精通多种编程语言和技术栈。请提供准确、有用的代码建议和解释。"));

    let mut current_file: Option<String> = None;
    let project_root = env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| ".".to_string());

    loop {
        let prompt = if let Some(file) = &current_file {
            format!("{}code:{} {} > {}", colors.magenta(), colors.blue(), file, colors.reset())
        } else {
            format!("{}code > {}", colors.magenta(), colors.reset())
        };

        print!("{}", prompt);

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.starts_with(':') {
            let cmd = &input[1..];
            let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
            let command = parts[0];
            let args = parts.get(1).map(|s| s.trim());

            match command {
                "help" | "h" => print_help(colors),
                "quit" | "exit" | "q" => {
                    println!("{}Goodbye!{}", colors.green(), colors.reset());
                    break;
                }
                "open" => {
                    if let Some(file) = args {
                        let abs_path = if Path::new(file).is_absolute() {
                            file.to_string()
                        } else {
                            Path::new(&project_root).join(file).display().to_string()
                        };
                        current_file = Some(abs_path.clone());
                        println!("{}Opened: {}{}", colors.green(), abs_path, colors.reset());
                    }
                }
                "close" => {
                    current_file = None;
                    println!("{}File closed{}", colors.green(), colors.reset());
                }
                "files" => {
                    println!("{}Files in {}:", colors.blue(), project_root);
                    for entry in WalkDir::new(&project_root).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                        if entry.path().is_file() {
                            println!("  {}", entry.path().file_name().unwrap().to_string_lossy());
                        }
                    }
                }
                "pwd" => println!("{}Current: {}{}", colors.blue(), project_root, colors.reset()),
                "context" => {
                    println!("{}Analyzing project...{}", colors.blue(), colors.reset());
                    let ctx = analyze_project(&project_root, colors);
                    println!("{}", ctx);
                }
                "search" => {
                    if let Some(pattern) = args {
                        println!("{}Searching for '{}'...{}", colors.blue(), pattern, colors.reset());
                        for entry in WalkDir::new(&project_root).max_depth(3).into_iter().filter_map(|e| e.ok()) {
                            let path = entry.path();
                            if let Ok(content) = fs::read_to_string(path) {
                                if content.contains(pattern) {
                                    println!("  {}", path.display());
                                }
                            }
                        }
                    }
                }
                "explain" | "review" | "refactor" | "debug" | "test" | "doc" => {
                    if let Some(file) = &current_file {
                        if let Ok(content) = fs::read_to_string(file) {
                            let action = match command {
                                "explain" => "解释",
                                "review" => "审查",
                                "refactor" => "重构",
                                "debug" => "调试",
                                "test" => "生成测试",
                                "doc" => "生成文档",
                                _ => "分析",
                            };
                            println!("{} {} {}...{}", colors.blue(), action, file, colors.reset());
                            let prompt = format!("请{}这段代码：\n\n```\n{}\n```", action, content);
                            messages.push(Message::user(&prompt));

                            match ai_client.chat(&messages) {
                                Ok(response) => {
                                    println!("{}", response);
                                    messages.push(Message::assistant(&response));
                                }
                                Err(e) => eprintln!("{}Error: {}{}", colors.red(), e, colors.reset()),
                            }
                        }
                    } else {
                        eprintln!("{}No file open. Use :open <file> first{}", colors.red(), colors.reset());
                    }
                }
                "chat" | "gen" | "ask" => {
                    if let Some(msg) = args {
                        let full_prompt = if command == "gen" {
                            format!("请实现以下功能：\n\n{}\n\n提供完整的代码实现。", msg)
                        } else {
                            msg.to_string()
                        };
                        messages.push(Message::user(&full_prompt));

                        match ai_client.chat(&messages) {
                            Ok(response) => {
                                println!("{}", response);
                                messages.push(Message::assistant(&response));
                            }
                            Err(e) => eprintln!("{}Error: {}{}", colors.red(), e, colors.reset()),
                        }
                    }
                }
                "fix" => {
                    if let Some(file) = &current_file {
                        if let Ok(content) = fs::read_to_string(file) {
                            println!("{}Fixing {}...{}", colors.blue(), file, colors.reset());
                            let prompt = format!("请修复这段代码中的错误：\n\n```\n{}\n```", content);
                            messages.push(Message::user(&prompt));

                            match ai_client.chat(&messages) {
                                Ok(response) => {
                                    println!("{}", response);
                                    if let Some(code) = extract_code_block(&response) {
                                        if let Ok(_) = fs::write(file, &code) {
                                            println!("{}Fixed and saved!{}", colors.green(), colors.reset());
                                        }
                                    }
                                    messages.push(Message::assistant(&response));
                                }
                                Err(e) => eprintln!("{}Error: {}{}", colors.red(), e, colors.reset()),
                            }
                        }
                    } else {
                        eprintln!("{}No file open{}", colors.red(), colors.reset());
                    }
                }
                "format" => {
                    if let Some(file) = &current_file {
                        let output = Command::new("rustfmt").arg(file).output();
                        match output {
                            Ok(_) => println!("{}Formatted!{}", colors.green(), colors.reset()),
                            Err(e) => eprintln!("{}Error: {}{}", colors.red(), e, colors.reset()),
                        }
                    } else {
                        eprintln!("{}No file open{}", colors.red(), colors.reset());
                    }
                }
                "run" => {
                    if let Some(file) = &current_file {
                        let ext = Path::new(file).extension().and_then(|e| e.to_str()).unwrap_or("");
                        let output = match ext {
                            "rs" => Command::new("cargo").arg("run").arg("--bin")
                                .arg(Path::new(file).file_stem().unwrap().to_string_lossy().as_ref()).output(),
                            "py" => Command::new("python").arg(file).output(),
                            "js" => Command::new("node").arg(file).output(),
                            _ => {
                                eprintln!("{}Unknown file type: {}{}", colors.red(), ext, colors.reset());
                                continue;
                            }
                        };
                        match output {
                            Ok(result) => {
                                println!("{}", String::from_utf8_lossy(&result.stdout));
                                if !result.stderr.is_empty() {
                                    eprintln!("{}", String::from_utf8_lossy(&result.stderr));
                                }
                            }
                            Err(e) => eprintln!("{}Error: {}{}", colors.red(), e, colors.reset()),
                        }
                    } else {
                        eprintln!("{}No file open{}", colors.red(), colors.reset());
                    }
                }
                "build" => {
                    let output = Command::new("cargo").arg("build").output();
                    match output {
                        Ok(result) => {
                            println!("{}", String::from_utf8_lossy(&result.stdout));
                            if !result.stderr.is_empty() {
                                eprintln!("{}", String::from_utf8_lossy(&result.stderr));
                            }
                        }
                        Err(e) => eprintln!("{}Error: {}{}", colors.red(), e, colors.reset()),
                    }
                }
                _ => {
                    eprintln!("{}Unknown command: {}{}", colors.red(), command, colors.reset());
                }
            }
        } else {
            messages.push(Message::user(input));

            match ai_client.chat(&messages) {
                Ok(response) => {
                    println!("{}", response);
                    messages.push(Message::assistant(&response));
                }
                Err(e) => eprintln!("{}Error: {}{}", colors.red(), e, colors.reset()),
            }
        }
    }
}

fn get_base_dir() -> String {
    env::var("RUST_TOOL_ROOT")
        .or_else(|_| env::var("HOME"))
        .or_else(|_| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
        })
        .unwrap_or_else(|_| ".".to_string())
}

fn manager_mode(ai_client: &AiClient, subcommand: Option<&str>, colors: &Colors, yolo: bool) {
    print_banner(&Mode::Manager, &ai_client.config, colors);

    let base_dir = get_base_dir();
    let mut engine = EvolutionEngine::new(&base_dir);

    match subcommand {
        Some("list") | None => {
            manager_list(&engine, colors);
        }
        Some("scan") => {
            manager_scan(&mut engine, colors);
        }
        Some("diagnose") => {
            println!("{}Usage: code -M diagnose <tool-name>{}", colors.yellow(), colors.reset());
            println!("Example: code -M diagnose grep");
        }
        Some("diagnose-all") => {
            manager_diagnose_all(&mut engine, colors);
        }
        Some("improve") => {
            println!("{}Usage: code -M improve <tool-name>{}", colors.yellow(), colors.reset());
            println!("Example: code -M improve grep");
        }
        Some("evolve") => {
            manager_evolve(&mut engine, ai_client, colors, yolo);
        }
        Some("create") => {
            println!("{}Usage: code -M create <name> <description>{}", colors.yellow(), colors.reset());
            println!("Example: code -M create jsontool \"JSON processing\"");
        }
        Some("suggest") => {
            manager_suggest(&mut engine, ai_client, colors);
        }
        Some(cmd) if cmd.starts_with("diagnose ") => {
            let tool_name = cmd.strip_prefix("diagnose ").unwrap_or("");
            manager_diagnose(&mut engine, tool_name, colors);
        }
        Some(cmd) if cmd.starts_with("improve ") => {
            let tool_name = cmd.strip_prefix("improve ").unwrap_or("");
            manager_improve(&mut engine, ai_client, tool_name, colors, yolo);
        }
        Some(cmd) if cmd.starts_with("create ") => {
            let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                manager_create(&mut engine, ai_client, parts[1], parts[2], colors, yolo);
            } else {
                println!("{}Usage: code -M create <name> <description>{}", colors.yellow(), colors.reset());
            }
        }
        _ => {
            println!("{}Unknown command: {}{}", colors.red(), subcommand.unwrap_or(""), colors.reset());
            println!("{}Use 'code -M list' to see available commands{}", colors.yellow(), colors.reset());
        }
    }
}

fn manager_list(engine: &EvolutionEngine, colors: &Colors) {
    let categories = [
        ("AI Tools", vec!["ai", "qwencode", "code", "evolve"]),
        ("File Tools", vec!["cat", "bat", "head", "tail", "wc", "sort", "uniq", "tr"]),
        ("Search Tools", vec!["grep", "find"]),
        ("Network Tools", vec!["curl", "wget", "ping", "ip"]),
        ("System Tools", vec!["ps", "df", "du", "top"]),
        ("Archive Tools", vec!["7z", "tar", "zip", "unzip", "gzip"]),
    ];

    for (cat, tool_names) in categories {
        println!("{}--- {} ---{}", colors.cyan(), cat, colors.reset());
        for name in tool_names {
            println!("  {}", name);
        }
        println!();
    }
}

fn manager_scan(engine: &mut EvolutionEngine, colors: &Colors) {
    println!("{}🔍 Scanning tools...{}", colors.blue(), colors.reset());

    match engine.scan_tools() {
        Ok(count) => {
            println!("{}✅ Found {} tools{}", colors.green(), count, colors.reset());
            println!();

            for (name, info) in engine.get_tools() {
                let issue_count = info.issues.len();
                let suggestion_count = info.suggestions.len();

                println!("{}📦 {}{}", colors.green(), name, colors.reset());
                println!("   Path: {}", info.path);
                println!("   Lines of code: {}", info.lines_of_code);
                println!("   Dependencies: {}", info.dependencies.len());

                if !info.description.is_empty() {
                    println!("   Description: {}", info.description);
                }

                if issue_count > 0 {
                    println!("{}   ⚠️  Issues: {}{}", colors.yellow(), issue_count, colors.reset());
                }
                if suggestion_count > 0 {
                    println!("{}   💡 Suggestions: {}{}", colors.cyan(), suggestion_count, colors.reset());
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("{}❌ Scan failed: {}{}", colors.red(), e, colors.reset());
        }
    }
}

fn manager_diagnose(engine: &mut EvolutionEngine, tool_name: &str, colors: &Colors) {
    println!("{}🔬 Diagnosing tool: {}{}", colors.blue(), tool_name, colors.reset());

    if let Err(e) = engine.scan_tools() {
        eprintln!("{}❌ Failed to scan tools: {}{}", colors.red(), e, colors.reset());
        return;
    }

    match engine.diagnose_tool(tool_name) {
        Some(report) => {
            print_diagnosis_report(&report, colors);
        }
        None => {
            eprintln!("{}❌ Tool '{}' not found{}", colors.red(), tool_name, colors.reset());
        }
    }
}

fn manager_diagnose_all(engine: &mut EvolutionEngine, colors: &Colors) {
    println!("{}🔬 Diagnosing all tools...{}", colors.blue(), colors.reset());

    if let Err(e) = engine.scan_tools() {
        eprintln!("{}❌ Failed to scan tools: {}{}", colors.red(), e, colors.reset());
        return;
    }

    let reports = engine.diagnose_all();

    for report in reports {
        print_diagnosis_report(&report, colors);
        println!();
    }

    let critical_count = reports.iter().filter(|r| r.priority == EvolutionPriority::Critical).count();
    let high_count = reports.iter().filter(|r| r.priority == EvolutionPriority::High).count();

    if critical_count > 0 || high_count > 0 {
        println!("{}📊 Summary:{} {} critical, {} high priority issues",
            colors.yellow(), colors.reset(), critical_count, high_count);
    } else {
        println!("{}✅ All tools look good!{}", colors.green(), colors.reset());
    }
}

fn print_diagnosis_report(report: &EvolutionReport, colors: &Colors) {
    let priority_str = match report.priority {
        EvolutionPriority::Critical => format!("{}CRITICAL{}", colors.red(), colors.reset()),
        EvolutionPriority::High => format!("{}HIGH{}", colors.yellow(), colors.reset()),
        EvolutionPriority::Medium => format!("{}MEDIUM{}", colors.blue(), colors.reset()),
        EvolutionPriority::Low => format!("{}LOW{}", colors.green(), colors.reset()),
    };

    println!("{}📦 {}{}", colors.green(), report.tool_name, colors.reset());
    println!("   Priority: {}", priority_str);
    println!("   {}", report.diagnosis);

    if !report.improvements.is_empty() {
        println!("\n{}   Improvements:{}", colors.cyan(), colors.reset());
        for imp in &report.improvements {
            println!("{}   - {}{}", colors.green(), imp, colors.reset());
        }
    }

    if !report.new_features.is_empty() {
        println!("\n{}   New Features:{}", colors.magenta(), colors.reset());
        for feat in &report.new_features {
            println!("{}   + {}{}", colors.magenta(), feat, colors.reset());
        }
    }
}

fn manager_improve(engine: &mut EvolutionEngine, ai_client: &AiClient, tool_name: &str, colors: &Colors, yolo: bool) {
    println!("{}🚀 AI-improving tool: {}{}", colors.blue(), tool_name, colors.reset());

    if let Err(e) = engine.scan_tools() {
        eprintln!("{}❌ Failed to scan tools: {}{}", colors.red(), e, colors.reset());
        return;
    }

    let report = match engine.diagnose_tool(tool_name) {
        Some(r) => r,
        None => {
            eprintln!("{}❌ Tool '{}' not found{}", colors.red(), tool_name, colors.reset());
            return;
        }
    };

    if report.improvements.is_empty() && report.new_features.is_empty() {
        println!("{}✅ Tool '{}' is already optimal!{}", colors.green(), tool_name, colors.reset());
        return;
    }

    let improvement_text = report.improvements.join("\n");
    let prompt = format!(
        "请为工具 '{}' 的以下改进点生成具体的代码实现：\n\n{}\n\n\
        请提供完整的、可运行的代码实现，只返回代码，不要有其他解释。",
        tool_name, improvement_text
    );

    println!("{}🤖 Generating improvements...{}", colors.magenta(), colors.reset());

    let messages = vec![Message::user(&prompt)];

    match ai_client.chat(&messages) {
        Ok(response) => {
            println!("\n{}📝 AI-Generated Improvements:{}", colors.green(), colors.reset());
            println!("{}", response);

            if let Some(code) = extract_code_block(&response) {
                if yolo {
                    println!("\n{}⚡ YOLO mode: Would save changes to tool{}", colors.yellow(), colors.reset());
                } else {
                    println!("\n{}💡 Use YOLO mode (-y) to auto-apply changes{}", colors.cyan(), colors.reset());
                }
            }
        }
        Err(e) => {
            eprintln!("{}❌ AI error: {}{}", colors.red(), e, colors.reset());
        }
    }
}

fn manager_evolve(engine: &mut EvolutionEngine, ai_client: &AiClient, colors: &Colors, yolo: bool) {
    println!("{}🧬 Starting full AI evolution...{}", colors.blue(), colors.reset());
    println!();

    if let Err(e) = engine.scan_tools() {
        eprintln!("{}❌ Failed to scan tools: {}{}", colors.red(), e, colors.reset());
        return;
    }

    let tools: Vec<String> = engine.get_tools().keys().cloned().collect();
    println!("{}📊 Analyzing {} tools...{}", colors.blue(), tools.len(), colors.reset());

    let mut all_improvements = Vec::new();

    for tool_name in &tools {
        if let Some(report) = engine.diagnose_tool(tool_name) {
            if !report.improvements.is_empty() || !report.new_features.is_empty() {
                all_improvements.push(report);
            }
        }
    }

    if all_improvements.is_empty() {
        println!("{}✅ All tools are in great shape! No evolution needed.{}", colors.green(), colors.reset());
        return;
    }

    println!("{}📋 Found {} tools that need improvement", colors.yellow(), all_improvements.len());

    let summary: String = all_improvements.iter()
        .map(|r| format!("- {}: {} improvements, {} new features",
            r.tool_name, r.improvements.len(), r.new_features.len()))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "请分析以下工具改进建议，并生成一个完整的进化计划：\n\n{}\n\n\
        对于每个工具，请提供：\n\
        1. 优先级排序\n\
        2. 具体代码改进\n\
        3. 新功能建议\n\n\
        只返回代码和简短说明。",
        summary
    );

    println!("\n{}🤖 AI analyzing evolution plan...{}", colors.magenta(), colors.reset());

    let messages = vec![Message::user(&prompt)];

    match ai_client.chat(&messages) {
        Ok(response) => {
            println!("\n{}🧬 Evolution Plan:{}", colors.green(), colors.reset());
            println!("{}", response);

            if yolo {
                println!("\n{}⚡ YOLO mode: Evolution changes would be applied automatically{}", colors.yellow(), colors.reset());
            } else {
                println!("\n{}💡 Use -y flag for YOLO mode (auto-apply changes){}", colors.cyan(), colors.reset());
            }
        }
        Err(e) => {
            eprintln!("{}❌ AI error: {}{}", colors.red(), e, colors.reset());
        }
    }

    let suggestions = engine.suggest_new_tools();
    if !suggestions.is_empty() {
        println!("\n{}🆕 Suggested New Tools:{}", colors.cyan(), colors.reset());
        for (name, desc) in &suggestions {
            println!("  - {}: {}", name, desc);
        }
    }
}

fn manager_create(engine: &mut EvolutionEngine, ai_client: &AiClient, name: &str, description: &str, colors: &Colors, yolo: bool) {
    println!("{}🆕 Creating new tool: {}{}", colors.blue(), name, colors.reset());
    println!("Description: {}", description);

    let prompt = format!(
        "请为 '{}' 工具生成完整的 Rust 实现代码。\n\n\
        工具名称: {}\n\
        功能描述: {}\n\n\
        要求：\n\
        1. 使用 'common' 库获取共享功能\n\
        2. 遵循 Rust 最佳实践\n\
        3. 添加基本的命令行参数解析\n\
        4. 包含测试代码\n\
        5. 只返回 Cargo.toml 和 main.rs 的内容",
        name, name, description
    );

    println!("\n{}🤖 AI generating tool code...{}", colors.magenta(), colors.reset());

    let messages = vec![Message::user(&prompt)];

    match ai_client.chat(&messages) {
        Ok(response) => {
            println!("\n{}📝 Generated Code:{}", colors.green(), colors.reset());
            println!("{}", response);

            if yolo {
                println!("\n{}⚡ YOLO mode: Would create tool files automatically{}", colors.yellow(), colors.reset());
                let result = engine.create_new_tool(name, description, &response);
                println!("{}", result);
            } else {
                println!("\n{}💡 Use -y flag to create the tool files{}", colors.cyan(), colors.reset());
            }
        }
        Err(e) => {
            eprintln!("{}❌ AI error: {}{}", colors.red(), e, colors.reset());
        }
    }
}

fn manager_suggest(engine: &mut EvolutionEngine, ai_client: &AiClient, colors: &Colors) {
    println!("{}🔮 AI suggesting new tools...{}", colors.blue(), colors.reset());

    if let Err(e) = engine.scan_tools() {
        eprintln!("{}❌ Failed to scan tools: {}{}", colors.red(), e, colors.reset());
        return;
    }

    let existing: Vec<String> = engine.get_tools().keys().cloned().collect();
    let prompt = format!(
        "当前项目已有以下工具：\n{}\n\n\
        请建议 3-5 个可以补充的新工具，并说明它们的功能和实现难度。\n\
        只返回工具名称和简短描述。",
        existing.join(", ")
    );

    let messages = vec![Message::user(&prompt)];

    match ai_client.chat(&messages) {
        Ok(response) => {
            println!("\n{}🆕 Suggested Tools:{}", colors.green(), colors.reset());
            println!("{}", response);
        }
        Err(e) => {
            eprintln!("{}❌ AI error: {}{}", colors.red(), e, colors.reset());
        }
    }

    let built_in = engine.suggest_new_tools();
    if !built_in.is_empty() {
        println!("\n{}📋 Built-in Suggestions:{}", colors.cyan(), colors.reset());
        for (name, desc) in &built_in {
            println!("  - {}: {}", name, desc);
        }
    }
}

fn analyze_project(dir: &str, colors: &Colors) -> String {
    let mut files = 0;
    let mut rust_files = 0;
    let mut deps = Vec::new();
    let mut project_type = "unknown";

    for entry in WalkDir::new(dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s.starts_with('.') && s != ".github" || s == "target" || s == "node_modules"
        }) {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "rs" {
            rust_files += 1;
        }
        files += 1;

        if path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
            project_type = "Rust";
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines() {
                    if line.starts_with("[dependencies]") {
                        break;
                    }
                    if line.contains('=') && !line.starts_with('[') && !line.starts_with('#') {
                        let dep = line.split('=').next().unwrap_or("").trim();
                        if !dep.is_empty() && !["name", "version", "edition"].contains(&dep) {
                            deps.push(dep.to_string());
                        }
                    }
                }
            }
        }
    }

    format!(
        "{}📊 Project Analysis:{}\n\
         Type: {}\n\
         Files: {} ({} .rs)\n\
         Dependencies: {}{}",
        colors.green(),
        colors.reset(),
        project_type,
        files,
        rust_files,
        deps.len(),
        if deps.is_empty() {
            String::new()
        } else {
            format!("\n  {}", deps.join(", "))
        }
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 || args.iter().any(|a| a == "-h" || a == "--help") {
        let colors = Colors::auto();
        print_help(&colors);
        return;
    }

    let mut mode = Mode::Chat;
    let mut provider: Option<AiProvider> = None;
    let mut model: Option<String> = None;
    let mut base_url: Option<String> = None;
    let mut prompt: Option<String> = None;
    let mut manager_subcommand: Option<String> = None;
    let mut yolo = false;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "-p" | "--provider" => {
                if i + 1 < args.len() {
                    i += 1;
                    provider = AiProvider::from_str(&args[i]);
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
            "-i" | "--ide" => mode = Mode::Ide,
            "-M" | "--manager" => mode = Mode::Manager,
            "-y" | "--yolo" => yolo = true,
            "-h" | "--help" => {
                let colors = Colors::auto();
                print_help(&colors);
                return;
            }
            _ if !arg.starts_with('-') => {
                prompt = Some(arg.clone());
            }
            _ => {}
        }
        i += 1;
    }

    if let Mode::Manager = mode {
        if args.len() > 2 {
            let idx = args.iter().position(|a| a == "-M" || a == "--manager").unwrap_or(1);
            if idx + 1 < args.len() {
                manager_subcommand = Some(args[idx + 1].clone());
            }
        }
    }

    let provider = provider.unwrap_or(AiProvider::Ollama);

    let mut ai_config = AiConfig::default();
    ai_config.provider = provider;
    ai_config.model = model.unwrap_or_else(|| provider.default_model().to_string());
    ai_config.base_url = base_url;

    let ai_client = AiClient::new(ai_config);
    let colors = Colors::auto();

    match mode {
        Mode::Chat => {
            if let Some(p) = prompt {
                chat_mode(&ai_client, &p, &colors);
            } else {
                interactive_chat_mode(&ai_client, &colors);
            }
        }
        Mode::Ide => {
            ide_mode(&ai_client, &colors);
        }
        Mode::Manager => {
            manager_mode(&ai_client, manager_subcommand.as_deref(), &colors, yolo);
        }
    }
}
