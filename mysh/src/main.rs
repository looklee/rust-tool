use std::io::{self, Write, BufRead};
use std::fs::{OpenOptions, File};
use std::env;

pub mod builtins;
pub mod command;
pub mod script;
pub mod ai_assistant;
pub mod completer;
pub mod cargo_cmd;
pub mod rust_diagnostic;

use ai_assistant::AIAssistant;
use completer::Completer;

/// Shell 提示符
const DEFAULT_PROMPT: &str = "mysh> ";

/// 展开 PS1 提示符中的特殊字符
fn expand_prompt(prompt: &str) -> String {
    let mut result = String::new();
    let mut chars = prompt.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('u') => {
                    // 用户名
                    result.push_str(&env::var("USER").unwrap_or_else(|_| "user".to_string()));
                }
                Some('h') => {
                    // 主机名
                    result.push_str(&env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string()));
                }
                Some('w') => {
                    // 当前工作目录（完整路径）
                    if let Ok(cwd) = env::current_dir() {
                        result.push_str(&cwd.display().to_string());
                    } else {
                        result.push('?');
                    }
                }
                Some('W') => {
                    // 当前工作目录（仅 basename）
                    if let Ok(cwd) = env::current_dir() {
                        if let Some(name) = cwd.file_name() {
                            result.push_str(&name.to_string_lossy());
                        } else {
                            result.push('/');
                        }
                    } else {
                        result.push('?');
                    }
                }
                Some('s') => {
                    // Shell 名称
                    result.push_str("mysh");
                }
                Some('$') => {
                    // 提示符结尾（root 为#，普通用户为$）
                    if env::var("USER").map(|u| u == "root").unwrap_or(false) {
                        result.push('#');
                    } else {
                        result.push('$');
                    }
                }
                Some('\\') => {
                    result.push('\\');
                }
                Some('n') => {
                    result.push('\n');
                }
                _ => {
                    result.push('\\');
                }
            }
        } else {
            result.push(c);
        }
    }
    
    result
}

/// 获取当前提示符
fn get_prompt() -> String {
    env::var("PS1")
        .map(|p| expand_prompt(&p))
        .unwrap_or_else(|_| DEFAULT_PROMPT.to_string())
}

/// 历史文件路径
fn history_path() -> Option<String> {
    env::var("HOME").map(|h| format!("{}/.mysh_history", h)).ok()
}

/// 加载历史
fn load_history() -> Vec<String> {
    let mut history = Vec::new();
    if let Some(path) = history_path() {
        if let Ok(file) = File::open(&path) {
            for line in io::BufReader::new(file).lines().flatten() {
                history.push(line);
            }
        }
    }
    history
}

/// 保存历史
fn save_history(history: &[String]) {
    if let Some(path) = history_path() {
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
        {
            let mut writer = io::BufWriter::new(file);
            for line in history {
                let _ = writeln!(writer, "{}", line);
            }
        }
    }
}

/// 加载启动文件
fn load_rc_file() {
    let rc_path = env::var("HOME")
        .map(|h| format!("{}/.myshrc", h))
        .ok();
    
    if let Some(path) = rc_path {
        if let Ok(content) = std::fs::read_to_string(&path) {
            println!("加载启动文件：{}", path);
            // 执行启动文件中的每一行
            for line in content.lines() {
                let line = line.trim();
                // 跳过空行和注释
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                // 执行命令（不打印错误）
                if let Err(_e) = command::execute(line) {
                    // 静默失败，继续执行
                }
            }
        }
    }
}

fn main() -> io::Result<()> {
    println!("mysh - 简易 Shell");
    println!("输入 'help' 查看可用命令，'exit' 退出\n");
    println!("提示：按 Ctrl+C 可中断正在运行的命令\n");

    // 加载启动文件
    load_rc_file();

    // 初始化 AI 助手
    let ai = AIAssistant::new();
    if ai.is_enabled() {
        println!("✓ AI 助手已启用：{}\n", ai.config().model_info());
    } else {
        println!("! AI 助手未启用");
        println!("  使用 OpenAI: export OPENAI_API_KEY=sk-...");
        println!("  使用 Ollama: export LLM_MODEL=llama3.2\n");
    }

    // 初始化补全器
    let mut completer = Completer::new();

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    // 简单的历史
    let mut history: Vec<String> = load_history();

    loop {
        // 打印提示符（动态获取）
        let prompt = get_prompt();
        print!("{}", prompt);
        stdout.flush()?;

        // 读取一行输入
        let mut input = String::new();
        let bytes_read = stdin.lock().read_line(&mut input)?;

        // EOF (Ctrl+D)
        if bytes_read == 0 {
            println!();
            break;
        }

        // 检查是否中断（Ctrl+C 会产生空行或只包含控制字符）
        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // 添加到历史
        history.push(input.to_string());
        completer.add_history(input.to_string());
        if history.len() > 100 {
            history.remove(0);
        }
        save_history(&history);

        // 执行命令
        match command::execute(input) {
            Ok(true) => break,  // exit 命令
            Ok(false) => continue,
            Err(e) => eprintln!("mysh: {}", e),
        }
    }

    Ok(())
}
