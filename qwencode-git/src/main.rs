use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
enum Provider {
    Ollama,
    OpenAI,
    Anthropic,
}

impl Provider {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ollama" => Some(Provider::Ollama),
            "openai" | "gpt" => Some(Provider::OpenAI),
            "anthropic" | "claude" => Some(Provider::Anthropic),
            _ => None,
        }
    }

    fn api_key_env(&self) -> &'static str {
        match self {
            Provider::Ollama => "",
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
        }
    }

    fn base_url(&self) -> &'static str {
        match self {
            Provider::Ollama => "http://localhost:11434",
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::Anthropic => "https://api.anthropic.com/v1",
        }
    }

    fn default_model(&self) -> &'static str {
        match self {
            Provider::Ollama => "qwen2.5-coder:7b",
            Provider::OpenAI => "gpt-4o-mini",
            Provider::Anthropic => "claude-sonnet-4-20250514",
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
    repo_path: String,
}

fn get_api_key(provider: Provider) -> Option<String> {
    match provider {
        Provider::Ollama => None,
        _ => env::var(provider.api_key_env()).ok(),
    }
}

/// 运行 git 命令
fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(&env::current_dir().unwrap_or_default())
        .output()
        .context("Failed to run git")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(anyhow::anyhow!("Git: {}\n{}", stdout, stderr))
    }
}

/// 检查是否是 git 仓库
fn is_git_repo() -> bool {
    git(&["rev-parse", "--git-dir"]).is_ok()
}

/// 获取 git status
fn get_status() -> Result<String> {
    git(&["status", "--short"])
}

/// 获取 diff
fn get_diff() -> Result<String> {
    git(&["diff", "HEAD"])
}

/// 获取当前分支
fn get_branch() -> Result<String> {
    git(&["branch", "--show-current"])
}

/// 获取最近提交
fn get_log(count: usize) -> Result<String> {
    git(&["log", "--oneline", &format!("-{}", count)])
}

/// 获取暂存区 diff
fn get_staged_diff() -> Result<String> {
    git(&["diff", "--cached"])
}

/// 调用 AI 生成 commit message
fn generate_commit_message(config: &Config, diff: &str) -> Result<String> {
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: "You are an expert at writing git commit messages. \
                     Follow conventional commits format: feat:, fix:, docs:, style:, refactor:, test:, chore:\n\
                     Be concise but descriptive. One line subject, blank line, then body if needed."
                .to_string(),
        },
        Message {
            role: "user".to_string(),
            content: format!("Generate a commit message for these changes:\n\n{}", diff),
        },
    ];

    chat(config, &messages)
}

/// AI 聊天
fn chat(config: &Config, messages: &[Message]) -> Result<String> {
    let api_key = config.api_key.as_ref();
    let base_url = config.provider.base_url();
    let model = &config.model;

    match config.provider {
        Provider::Ollama => {
            let url = format!("{}/api/chat", base_url);
            let body = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": false
            });

            let response = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_json(body)?;

            let json: serde_json::Value = response.into_json()?;
            Ok(json["message"]["content"].as_str().unwrap_or("").to_string())
        }
        Provider::OpenAI => {
            let key = api_key.ok_or_else(|| anyhow::anyhow!("Set OPENAI_API_KEY"))?;
            let url = format!("{}/chat/completions", base_url);
            let body = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": false
            });

            let response = ureq::post(&url)
                .set("Authorization", &format!("Bearer {}", key))
                .set("Content-Type", "application/json")
                .send_json(body)?;

            let json: serde_json::Value = response.into_json()?;
            Ok(json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
        }
        Provider::Anthropic => {
            let key = api_key.ok_or_else(|| anyhow::anyhow!("Set ANTHROPIC_API_KEY"))?;
            let url = format!("{}/messages", base_url);
            let body = serde_json::json!({
                "model": model,
                "max_tokens": 1024,
                "messages": messages
            });

            let response = ureq::post(&url)
                .set("x-api-key", key)
                .set("anthropic-version", "2023-06-01")
                .set("Content-Type", "application/json")
                .send_json(body)?;

            let json: serde_json::Value = response.into_json()?;
            Ok(json["content"][0]["text"].as_str().unwrap_or("").to_string())
        }
    }
}

fn print_help() {
    println!(
        r#"
Git Commands:
  status, st          Show git status
  diff, d             Show changes (unstaged)
  diff-staged, ds     Show staged changes
  log, l [N]          Show commit history
  branch, b           Show current branch
  commit, c [msg]     Commit with AI-generated message
  suggest, s          Suggest what to commit
  add, a <file>       Stage file
  reset, r <file>     Unstage file

AI Commands:
  explain, e          Explain changes
  review, rev         Review changes

Other Commands:
  help, h             Show this help
  quit, q, exit       Exit
  <git command>       Run any git command directly

Examples:
  qwencode-git st
  qwencode-git commit "Add feature"
  qwencode-git log 5
"#
    );
}

fn interactive_mode(config: &Config) {
    println!("╔════════════════════════════════════════════════╗");
    println!("║         QwenCode Git Assistant                 ║");
    println!("╚════════════════════════════════════════════════╝");
    println!();

    // 检查是否是 git 仓库
    if !is_git_repo() {
        eprintln!("❌ Not a git repository: {}", config.repo_path);
        eprintln!("Run 'git init' to initialize a repository.");
        return;
    }

    println!("Repository: {}", config.repo_path);
    println!("Provider: {:?}, Model: {}", config.provider, config.model);
    println!();

    // 显示当前分支
    if let Ok(branch) = get_branch() {
        println!("📍 Branch: {}", if branch.is_empty() { "DETACHED HEAD" } else { &branch });
    }

    // 显示简要状态
    if let Ok(status) = get_status() {
        if status.is_empty() {
            println!("✅ Working tree clean");
        } else {
            let lines: Vec<_> = status.lines().collect();
            println!("📝 {} file(s) changed", lines.len());
        }
    }

    println!();
    println!("Type /help for commands, /quit to exit");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("🔀 > ");
        let _ = stdout.flush();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let cmd = parts.first().unwrap_or(&"");

        match *cmd {
            "status" | "st" => {
                match git(&["status"]) {
                    Ok(s) => println!("{}", s),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "diff" | "d" => {
                match get_diff() {
                    Ok(d) => println!("{}", d),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "diff-staged" | "ds" => {
                match get_staged_diff() {
                    Ok(d) => println!("{}", d),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "log" | "l" => {
                let count = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
                match get_log(count) {
                    Ok(l) => println!("{}", l),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "branch" | "b" => {
                match get_branch() {
                    Ok(b) => println!("📍 {}", if b.is_empty() { "DETACHED HEAD" } else { &b }),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "commit" | "c" => {
                let msg = if parts.len() > 1 {
                    parts[1..].join(" ")
                } else {
                    // 自动生成
                    print!("🤖 Generating commit message... ");
                    let _ = stdout.flush();

                    let diff = get_diff().unwrap_or_default();
                    if diff.is_empty() {
                        eprintln!("\nNo changes to commit");
                        continue;
                    }

                    match generate_commit_message(config, &diff) {
                        Ok(msg) => {
                            println!("\n\n📝 {}", msg);
                            msg
                        }
                        Err(e) => {
                            eprintln!("\nError: {}", e);
                            continue;
                        }
                    }
                };

                // 询问是否提交
                print!("\nCommit with this message? [y/N]: ");
                let _ = stdout.flush();
                let mut confirm = String::new();
                if stdin.read_line(&mut confirm).is_ok() && confirm.trim().to_lowercase() == "y" {
                    match git(&["commit", "-m", &msg]) {
                        Ok(out) => println!("✅ {}\n", out),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    println!("Commit cancelled");
                }
            }
            "suggest" | "s" => {
                match get_status() {
                    Ok(status) => {
                        let prompt = format!(
                            "Based on this git status, suggest logical commit groupings:\n\n{}",
                            status
                        );
                        let messages = vec![Message {
                            role: "user".to_string(),
                            content: prompt,
                        }];

                        match chat(config, &messages) {
                            Ok(suggestion) => println!("{}", suggestion),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "explain" | "e" => {
                match get_diff() {
                    Ok(diff) => {
                        if diff.is_empty() {
                            println!("No changes to explain");
                            continue;
                        }

                        let messages = vec![Message {
                            role: "user".to_string(),
                            content: format!("Explain these git changes in a clear way:\n\n{}", diff),
                        }];

                        match chat(config, &messages) {
                            Ok(explanation) => println!("{}", explanation),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "review" | "rev" => {
                match get_diff() {
                    Ok(diff) => {
                        if diff.is_empty() {
                            println!("No changes to review");
                            continue;
                        }

                        let messages = vec![Message {
                            role: "user".to_string(),
                            content: format!(
                                "Review these code changes for:\n\
                                 1. Bugs or issues\n\
                                 2. Code style problems\n\
                                 3. Suggestions for improvement\n\n{}",
                                diff
                            ),
                        }];

                        match chat(config, &messages) {
                            Ok(review) => println!("{}", review),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "add" | "a" => {
                if parts.len() > 1 {
                    match git(&["add", &parts[1..].join(" ")]) {
                        Ok(_) => println!("✅ Staged"),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    eprintln!("Usage: add <file>");
                }
            }
            "reset" | "r" => {
                if parts.len() > 1 {
                    match git(&["reset", "HEAD", &parts[1..].join(" ")]) {
                        Ok(_) => println!("✅ Unstaged"),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    eprintln!("Usage: reset <file>");
                }
            }
            "help" | "h" => {
                print_help();
            }
            "quit" | "q" | "exit" => {
                println!("Goodbye!");
                break;
            }
            _ => {
                // 尝试作为 git 命令执行
                let git_args: Vec<&str> = parts.iter().map(|s| *s).collect();
                match git(&git_args) {
                    Ok(out) => println!("{}", out),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
    }
}

fn print_help_and_exit() {
    println!(
        r#"
QwenCode Git Assistant - AI-powered Git helper

Usage: qwencode-git [OPTIONS]

Options:
  -p, --provider <PROVIDER>  Model provider (ollama, openai, anthropic)
  -m, --model <MODEL>        Model name
  -r, --repo <PATH>          Repository path (default: current dir)
  -i, --interactive          Interactive mode (default)
  -h, --help                 Show this help

Examples:
  qwencode-git -i                    # Start interactive mode
  qwencode-git status                # Show status
  qwencode-git commit "Add feature"  # Commit with message
  qwencode-git log 5                 # Show last 5 commits
"#
    );
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 || args.iter().any(|a| a == "-h" || a == "--help") {
        print_help_and_exit();
        return Ok(());
    }

    let mut provider: Option<Provider> = None;
    let mut model: Option<String> = None;
    let mut repo_path = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());

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
            "-r" | "--repo" => {
                if i + 1 < args.len() {
                    i += 1;
                    repo_path = args[i].clone();
                    env::set_current_dir(&args[i]).ok();
                }
            }
            "-i" | "--interactive" => {
                // 默认交互模式
            }
            // 直接执行 git 命令
            "status" | "st" | "diff" | "d" | "log" | "l" | "branch" | "b" | "commit" | "c" | "add" | "a" => {
                let cmd_args: Vec<&str> = args.iter().skip(i).map(|s| s.as_str()).collect();
                let output = git(&cmd_args)?;
                println!("{}", output);
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    let provider = provider.unwrap_or(Provider::Ollama);
    let model = model.unwrap_or_else(|| provider.default_model().to_string());
    let api_key = get_api_key(provider);

    let config = Config {
        provider,
        api_key,
        model,
        repo_path,
    };

    interactive_mode(&config);

    Ok(())
}
