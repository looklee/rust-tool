use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiProvider {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
    DeepSeek,
    Moonshot,
    Zhipu,
    Qwen,
    MiniMax,
}

impl AiProvider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" | "gpt" | "chatgpt" => Some(AiProvider::OpenAI),
            "anthropic" | "claude" => Some(AiProvider::Anthropic),
            "gemini" | "google" => Some(AiProvider::Gemini),
            "ollama" => Some(AiProvider::Ollama),
            "deepseek" => Some(AiProvider::DeepSeek),
            "moonshot" | "kimi" => Some(AiProvider::Moonshot),
            "zhipu" | "glm" | "chatglm" => Some(AiProvider::Zhipu),
            "qwen" | "aliyun" | "dashscope" | "百炼" => Some(AiProvider::Qwen),
            "minimax" | "cm" | "minimax-m2" => Some(AiProvider::MiniMax),
            _ => None,
        }
    }

    pub fn api_key_env(&self) -> &'static str {
        match self {
            AiProvider::OpenAI => "OPENAI_API_KEY",
            AiProvider::Anthropic => "ANTHROPIC_API_KEY",
            AiProvider::Gemini => "GEMINI_API_KEY",
            AiProvider::Ollama => "",
            AiProvider::DeepSeek => "DEEPSEEK_API_KEY",
            AiProvider::Moonshot => "MOONSHOT_API_KEY",
            AiProvider::Zhipu => "ZHIPU_API_KEY",
            AiProvider::Qwen => "DASHSCOPE_API_KEY",
            AiProvider::MiniMax => "MINIMAX_API_KEY",
        }
    }

    pub fn base_url(&self) -> &'static str {
        match self {
            AiProvider::OpenAI => "https://api.openai.com/v1",
            AiProvider::Anthropic => "https://api.anthropic.com/v1",
            AiProvider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            AiProvider::Ollama => "http://localhost:11434",
            AiProvider::DeepSeek => "https://api.deepseek.com",
            AiProvider::Moonshot => "https://api.moonshot.cn/v1",
            AiProvider::Zhipu => "https://open.bigmodel.cn/api/paas/v4",
            AiProvider::Qwen => "https://coding.dashscope.aliyuncs.com/v1",
            AiProvider::MiniMax => "https://zhenze-huhehaote.cmecloud.cn/api/coding/v1",
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            AiProvider::OpenAI => "gpt-4o-mini",
            AiProvider::Anthropic => "claude-sonnet-4-20250514",
            AiProvider::Gemini => "gemini-2.0-flash",
            AiProvider::Ollama => "qwen2.5-coder:7b",
            AiProvider::DeepSeek => "deepseek-coder",
            AiProvider::Moonshot => "moonshot-v1-8k",
            AiProvider::Zhipu => "code-glm",
            AiProvider::Qwen => "qwen3.5-plus",
            AiProvider::MiniMax => "cm-code-latest",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AiProvider::OpenAI => "openai",
            AiProvider::Anthropic => "anthropic",
            AiProvider::Gemini => "gemini",
            AiProvider::Ollama => "ollama",
            AiProvider::DeepSeek => "deepseek",
            AiProvider::Moonshot => "moonshot",
            AiProvider::Zhipu => "zhipu",
            AiProvider::Qwen => "qwen",
            AiProvider::MiniMax => "minimax",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: &str) -> Self {
        Message {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Message {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }

    pub fn system(content: &str) -> Self {
        Message {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub provider: AiProvider,
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: Option<String>,
    pub max_tokens: usize,
    pub temperature: f64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::Ollama,
            api_key: None,
            model: AiProvider::Ollama.default_model().to_string(),
            base_url: None,
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

pub struct AiClient {
    config: AiConfig,
}

impl AiClient {
    pub fn new(config: AiConfig) -> Self {
        AiClient { config }
    }

    pub fn with_provider(mut self, provider: AiProvider) -> Self {
        self.config.provider = provider;
        self.config.model = provider.default_model().to_string();
        self.config.api_key = Self::get_api_key(provider);
        self
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.config.model = model.to_string();
        self
    }

    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.config.api_key = Some(api_key.to_string());
        self
    }

    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.config.base_url = Some(base_url.to_string());
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.config.max_tokens = max_tokens;
        self
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.config.temperature = temperature;
        self
    }

    fn get_api_key(provider: AiProvider) -> Option<String> {
        match provider {
            AiProvider::Ollama => None,
            _ => env::var(provider.api_key_env()).ok(),
        }
    }

    pub fn chat(&self, messages: &[Message]) -> Result<String, String> {
        let provider = self.config.provider;
        let api_key = self.config.api_key.as_ref();
        let base_url = self.config.base_url.as_deref().unwrap_or(provider.base_url());
        let model = &self.config.model;

        match provider {
            AiProvider::Ollama => self.chat_ollama(base_url, model, messages),
            AiProvider::OpenAI | AiProvider::DeepSeek | AiProvider::Moonshot | AiProvider::Zhipu | AiProvider::Qwen | AiProvider::MiniMax => {
                let key = api_key.ok_or_else(|| format!("API key required. Set {}", provider.api_key_env()))?;
                self.chat_compatible(key, base_url, model, messages)
            }
            AiProvider::Anthropic => {
                let key = api_key.ok_or_else(|| format!("API key required. Set {}", provider.api_key_env()))?;
                self.chat_anthropic(key, base_url, model, messages)
            }
            AiProvider::Gemini => {
                let key = api_key.ok_or_else(|| format!("API key required. Set {}", provider.api_key_env()))?;
                self.chat_gemini(key, model, messages)
            }
        }
    }

    fn chat_compatible(&self, api_key: &str, base_url: &str, model: &str, messages: &[Message]) -> Result<String, String> {
        let url = format!("{}/chat/completions", base_url);

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
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

    fn chat_anthropic(&self, api_key: &str, base_url: &str, model: &str, messages: &[Message]) -> Result<String, String> {
        let url = format!("{}/messages", base_url);

        let body = serde_json::json!({
            "model": model,
            "max_tokens": self.config.max_tokens,
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

    fn chat_gemini(&self, api_key: &str, model: &str, messages: &[Message]) -> Result<String, String> {
        let last = messages.last().ok_or_else(|| "No messages".to_string())?;
        let body = serde_json::json!({
            "contents": [{"parts": [{"text": last.content}]}]
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

    fn chat_ollama(&self, base_url: &str, model: &str, messages: &[Message]) -> Result<String, String> {
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

    pub fn explain_code(&self, code: &str, language: &str) -> Result<String, String> {
        let prompt = format!(
            "请解释这段{}代码的功能和实现原理：\n\n```{}
{}
```",
            language, language, code
        );

        let messages = vec![
            Message::system("你是一位资深编程导师，擅长解释代码逻辑和设计模式。"),
            Message::user(&prompt),
        ];

        self.chat(&messages)
    }

    pub fn generate_code(&self, description: &str, language: &str) -> Result<String, String> {
        let prompt = format!(
            "请用{}语言实现以下功能：\n\n{}\n\n要求：\n- 代码完整可运行\n- 使用最佳实践\n- 添加必要的注释\n- 提供完整的实现",
            language, description
        );

        let messages = vec![
            Message::system("你是一位专业的软件开发工程师，擅长编写高质量代码。"),
            Message::user(&prompt),
        ];

        self.chat(&messages)
    }

    pub fn refactor_code(&self, code: &str, language: &str) -> Result<String, String> {
        let prompt = format!(
            "请重构这段{}代码，提供改进建议和优化后的代码：\n\n```{}
{}
```\n\n请分析：\n1. 代码中的问题和潜在改进点\n2. 性能优化建议\n3. 代码可读性改进\n4. 提供重构后的完整代码",
            language, language, code
        );

        let messages = vec![
            Message::system("你是一位资深代码审查专家，擅长代码优化和重构。"),
            Message::user(&prompt),
        ];

        self.chat(&messages)
    }

    pub fn debug_error(&self, error: &str, context: &str) -> Result<String, String> {
        let prompt = format!(
            "请帮助我调试这个错误：\n\n错误信息：\n{}\n\n代码上下文：\n{}\n\n请分析：\n1. 错误原因\n2. 解决方案\n3. 修复代码",
            error, context
        );

        let messages = vec![
            Message::system("你是一位调试专家，擅长分析和解决编程错误。"),
            Message::user(&prompt),
        ];

        self.chat(&messages)
    }

    pub fn generate_tests(&self, code: &str, language: &str) -> Result<String, String> {
        let prompt = format!(
            "请为这段{}代码生成全面的测试用例：\n\n```{}
{}
```\n\n要求：\n- 覆盖主要功能路径\n- 包含边界条件测试\n- 提供完整的测试代码",
            language, language, code
        );

        let messages = vec![
            Message::system("你是一位测试工程师，擅长编写高质量的测试用例。"),
            Message::user(&prompt),
        ];

        self.chat(&messages)
    }

    pub fn review_code(&self, code: &str, language: &str) -> Result<String, String> {
        let prompt = format!(
            "请审查这段{}代码并提供反馈：\n\n```{}
{}
```\n\n请从以下方面进行审查：\n1. 代码质量\n2. 安全性\n3. 性能\n4. 可读性\n5. 最佳实践",
            language, language, code
        );

        let messages = vec![
            Message::system("你是一位资深代码审查员，擅长代码质量评估。"),
            Message::user(&prompt),
        ];

        self.chat(&messages)
    }
}

#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub name: String,
    pub project_type: String,
    pub files: Vec<String>,
    pub dependencies: Vec<String>,
    pub entry_points: Vec<String>,
    pub structure: HashMap<String, usize>,
}

pub fn extract_code_block(response: &str) -> Option<String> {
    let lines: Vec<&str> = response.lines().collect();
    let mut in_code_block = false;
    let mut code_lines = Vec::new();
    let mut found_block = false;

    for line in lines {
        if line.trim().starts_with("```") {
            if in_code_block {
                found_block = true;
                break;
            } else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert_eq!(AiProvider::from_str("qwen"), Some(AiProvider::Qwen));
        assert_eq!(AiProvider::from_str("openai"), Some(AiProvider::OpenAI));
        assert_eq!(AiProvider::from_str("ollama"), Some(AiProvider::Ollama));
        assert_eq!(AiProvider::from_str("unknown"), None);
    }

    #[test]
    fn test_message_constructors() {
        let user = Message::user("Hello");
        assert_eq!(user.role, "user");
        assert_eq!(user.content, "Hello");

        let assistant = Message::assistant("Hi");
        assert_eq!(assistant.role, "assistant");
        assert_eq!(assistant.content, "Hi");

        let system = Message::system("You are a bot");
        assert_eq!(system.role, "system");
        assert_eq!(system.content, "You are a bot");
    }

    #[test]
    fn test_extract_code_block() {
        let response = "Here's your code:\n```rust\nfn main() {}\n```\nThat's it!";
        let code = extract_code_block(response);
        assert_eq!(code, Some("fn main() {}".to_string()));

        let no_code = "Just a message";
        assert_eq!(extract_code_block(no_code), None);
    }
}
