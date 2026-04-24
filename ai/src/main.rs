use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};

/// 支持的模型提供商
#[derive(Debug, Clone, Copy)]
enum Provider {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
    DeepSeek,
    Moonshot,
    Zhipu,
    Qwen,  // 通义千问 / 百炼
}

impl Provider {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" | "gpt" => Some(Provider::OpenAI),
            "anthropic" | "claude" => Some(Provider::Anthropic),
            "gemini" | "google" => Some(Provider::Gemini),
            "ollama" => Some(Provider::Ollama),
            "deepseek" => Some(Provider::DeepSeek),
            "moonshot" | "kimi" => Some(Provider::Moonshot),
            "zhipu" | "glm" => Some(Provider::Zhipu),
            "qwen" | "aliyun" | "dashscope" | "百炼" => Some(Provider::Qwen),
            _ => None,
        }
    }

    fn api_key_env(&self) -> &'static str {
        match self {
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::Gemini => "GEMINI_API_KEY",
            Provider::Ollama => "",
            Provider::DeepSeek => "DEEPSEEK_API_KEY",
            Provider::Moonshot => "MOONSHOT_API_KEY",
            Provider::Zhipu => "ZHIPU_API_KEY",
            Provider::Qwen => "DASHSCOPE_API_KEY",
        }
    }

    fn base_url(&self) -> &'static str {
        match self {
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Provider::Ollama => "http://localhost:11434",
            Provider::DeepSeek => "https://api.deepseek.com",
            Provider::Moonshot => "https://api.moonshot.cn/v1",
            Provider::Zhipu => "https://open.bigmodel.cn/api/paas/v4",
            Provider::Qwen => "https://coding.dashscope.aliyuncs.com/v1",
        }
    }

    fn default_model(&self) -> &'static str {
        match self {
            Provider::OpenAI => "gpt-4o-mini",
            Provider::Anthropic => "claude-sonnet-4-20250514",
            Provider::Gemini => "gemini-2.0-flash",
            Provider::Ollama => "llama3",
            Provider::DeepSeek => "deepseek-chat",
            Provider::Moonshot => "moonshot-v1-8k",
            Provider::Zhipu => "glm-4",
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
    system_prompt: Option<String>,
}

fn get_api_key(provider: Provider) -> Option<String> {
    match provider {
        Provider::Ollama => None,
        _ => env::var(provider.api_key_env()).ok(),
    }
}

/// 通用 OpenAI 兼容 API 调用（支持 OpenAI、DeepSeek、Moonshot、Zhipu、Qwen）
fn chat_compatible(api_key: &str, base_url: &str, model: &str, messages: &[Message]) -> Result<String, String> {
    let url = format!("{}/chat/completions", base_url);

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false
    });

    let response = ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;

    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid response format".to_string())
}

fn chat_anthropic(api_key: &str, base_url: &str, model: &str, messages: &[Message], system: &str) -> Result<String, String> {
    let url = format!("{}/messages", base_url);

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system,
        "messages": messages
    });

    let response = ureq::post(&url)
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;

    json["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid response format".to_string())
}

fn chat_gemini(api_key: &str, model: &str, _messages: &[Message]) -> Result<String, String> {
    let body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": "Hello"
            }]
        }]
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;

    json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid response format".to_string())
}

fn chat_ollama(base_url: &str, model: &str, messages: &[Message]) -> Result<String, String> {
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
        .ok_or_else(|| "Invalid response format".to_string())
}

fn chat(provider: Provider, config: &Config, messages: &[Message]) -> Result<String, String> {
    let api_key = config.api_key.as_ref();
    let base_url = config.base_url.as_deref().unwrap_or(provider.base_url());
    let model = &config.model;

    match provider {
        Provider::Ollama => chat_ollama(base_url, model, messages),
        Provider::OpenAI | Provider::DeepSeek | Provider::Moonshot | Provider::Zhipu | Provider::Qwen => {
            let key = api_key.ok_or_else(|| format!("API key required. Set {}", provider.api_key_env()))?;
            chat_compatible(key, base_url, model, messages)
        }
        Provider::Anthropic => {
            let key = api_key.ok_or_else(|| format!("API key required. Set {}", provider.api_key_env()))?;
            let system = config.system_prompt.as_deref().unwrap_or("");
            chat_anthropic(key, base_url, model, messages, system)
        }
        Provider::Gemini => {
            let key = api_key.ok_or_else(|| format!("API key required. Set {}", provider.api_key_env()))?;
            chat_gemini(key, model, messages)
        }
    }
}

fn print_help() {
    println!(r#"
ai - AI 助手，支持多个大模型提供商

Usage: ai [OPTIONS] [PROMPT]

Options:
  -p, --provider <PROVIDER>  模型提供商 (openai, anthropic, gemini, ollama, deepseek, moonshot, zhipu, qwen)
  -m, --model <MODEL>        模型名称
  -s, --system <PROMPT>      系统提示词
  -i, --interactive          交互模式
  --base-url <URL>           自定义 API 地址
  -h, --help                 显示帮助

Environment Variables:
  OPENAI_API_KEY      OpenAI API Key
  ANTHROPIC_API_KEY   Anthropic API Key
  GEMINI_API_KEY      Google Gemini API Key
  DEEPSEEK_API_KEY    DeepSeek API Key
  MOONSHOT_API_KEY    Moonshot API Key
  ZHIPU_API_KEY       Zhipu API Key
  DASHSCOPE_API_KEY   通义千问/百炼 API Key ✨

Supported Models:
  openai     - gpt-4o-mini (default), gpt-4o, gpt-4-turbo
  anthropic  - claude-sonnet-4-20250514 (default), claude-opus
  gemini     - gemini-2.0-flash (default), gemini-pro
  ollama     - llama3 (default), qwen2.5-coder:7b
  deepseek   - deepseek-chat (default), deepseek-coder
  moonshot   - moonshot-v1-8k (default), moonshot-v1-32k
  zhipu      - glm-4 (default), glm-3-turbo
  qwen       - qwen-coder-plus-latest (default) ✨
               qwen-coder-turbo-latest, qwen-plus, qwen-max, qwen-turbo, qwen-flash, qwen-long

Examples:
  ai -p openai 'Hello, world!'
  ai -p qwen -m qwen-coder-plus '用 Rust 写一个 HTTP 服务器'
  ai -p ollama -i  # 交互模式（本地）
  export DASHSCOPE_API_KEY="sk-..." && ai -p qwen 'Hello'
"#);
}

fn interactive_mode(config: &Config) {
    println!("AI 助手交互模式 (输入 /quit 退出)");
    println!("提供商：{:?}, 模型：{}", config.provider, config.model);
    println!();

    let mut messages: Vec<Message> = Vec::new();

    if let Some(ref system) = config.system_prompt {
        messages.push(Message {
            role: "system".to_string(),
            content: system.clone(),
        });
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("User> ");
        let _ = stdout.flush();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/quit" || input == "/exit" {
            println!("Goodbye!");
            break;
        }

        messages.push(Message {
            role: "user".to_string(),
            content: input.to_string(),
        });

        print!("AI> ");
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
                messages.pop();
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 || args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }

    let mut provider: Option<Provider> = None;
    let mut model: Option<String> = None;
    let mut system_prompt: Option<String> = None;
    let mut base_url: Option<String> = None;
    let mut interactive = false;
    let mut prompt: Option<String> = None;

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
            "-s" | "--system" => {
                if i + 1 < args.len() {
                    i += 1;
                    system_prompt = Some(args[i].clone());
                }
            }
            "--base-url" => {
                if i + 1 < args.len() {
                    i += 1;
                    base_url = Some(args[i].clone());
                }
            }
            "-i" | "--interactive" => {
                interactive = true;
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            _ if !arg.starts_with('-') => {
                prompt = Some(arg.clone());
            }
            _ => {
                eprintln!("Unknown option: {}", arg);
            }
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
        base_url,
        system_prompt,
    };

    if interactive {
        interactive_mode(&config);
    } else if let Some(prompt) = prompt {
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
        }];

        match chat(config.provider, &config, &messages) {
            Ok(response) => {
                println!("{}", response);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    } else {
        eprintln!("Please provide a prompt or use -i for interactive mode");
    }
}
