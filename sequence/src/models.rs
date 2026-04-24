use serde::{Deserialize, Serialize};
use std::env;

/// Supported model providers
#[derive(Debug, Clone, Copy)]
pub enum Provider {
    Ollama,
    OpenAI,
    Anthropic,
    Gemini,
    DeepSeek,
    Qwen,
}

impl Provider {
    fn from_env() -> Self {
        match env::var("SEQUENCE_MODEL_PROVIDER")
            .unwrap_or_else(|_| "ollama".to_string())
            .to_lowercase()
            .as_str()
        {
            "openai" | "gpt" => Provider::OpenAI,
            "anthropic" | "claude" => Provider::Anthropic,
            "gemini" | "google" => Provider::Gemini,
            "deepseek" => Provider::DeepSeek,
            "qwen" | "dashscope" => Provider::Qwen,
            _ => Provider::Ollama,
        }
    }

    fn base_url(&self) -> &'static str {
        match self {
            Provider::Ollama => "http://localhost:11434",
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Provider::DeepSeek => "https://api.deepseek.com",
            Provider::Qwen => "https://coding.dashscope.aliyuncs.com/v1",
        }
    }

    fn default_model(&self) -> &'static str {
        match self {
            Provider::Ollama => "qwen2.5-coder:7b",
            Provider::OpenAI => "gpt-4o-mini",
            Provider::Anthropic => "claude-sonnet-4-20250514",
            Provider::Gemini => "gemini-2.0-flash",
            Provider::DeepSeek => "deepseek-chat",
            Provider::Qwen => "qwen3.5-plus",
        }
    }

    fn api_key_env(&self) -> &'static str {
        match self {
            Provider::Ollama => "",
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::Gemini => "GEMINI_API_KEY",
            Provider::DeepSeek => "DEEPSEEK_API_KEY",
            Provider::Qwen => "BAILIAN_CODING_PLAN_API_KEY",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

/// Chat with the configured AI model
pub fn chat(system_prompt: &str, user_input: &str) -> String {
    let provider = Provider::from_env();
    let model = env::var("SEQUENCE_MODEL")
        .unwrap_or_else(|_| provider.default_model().to_string());

    match provider {
        Provider::Ollama => chat_ollama(&model, system_prompt, user_input),
        Provider::OpenAI | Provider::DeepSeek | Provider::Qwen => {
            chat_compatible(provider, &model, system_prompt, user_input)
        }
        Provider::Anthropic => chat_anthropic(&model, system_prompt, user_input),
        Provider::Gemini => chat_gemini(&model, system_prompt, user_input),
    }
}

fn chat_ollama(model: &str, system_prompt: &str, user_input: &str) -> String {
    let url = format!("{}/api/chat", Provider::Ollama.base_url());

    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": system_prompt
        }),
        serde_json::json!({
            "role": "user",
            "content": user_input
        }),
    ];

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false
    });

    match ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(body)
    {
        Ok(response) => {
            let json: serde_json::Value = response.into_json().unwrap_or_default();
            json["message"]["content"]
                .as_str()
                .unwrap_or("No response")
                .to_string()
        }
        Err(e) => {
            format!("⚠️  Ollama error: {}. Is Ollama running?", e)
        }
    }
}

fn chat_compatible(provider: Provider, model: &str, system_prompt: &str, user_input: &str) -> String {
    let api_key = match env::var(provider.api_key_env()) {
        Ok(key) => key,
        Err(_) => return format!("⚠️  API key not set. Set {}.", provider.api_key_env()),
    };

    let url = format!("{}/chat/completions", provider.base_url());

    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": system_prompt
        }),
        serde_json::json!({
            "role": "user",
            "content": user_input
        }),
    ];

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false
    });

    match ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_json(body)
    {
        Ok(response) => {
            let json: serde_json::Value = response.into_json().unwrap_or_default();
            json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("No response")
                .to_string()
        }
        Err(e) => {
            format!("⚠️  API error: {}", e)
        }
    }
}

fn chat_anthropic(model: &str, system_prompt: &str, user_input: &str) -> String {
    let api_key = match env::var(Provider::Anthropic.api_key_env()) {
        Ok(key) => key,
        Err(_) => return format!("⚠️  API key not set. Set {}.", Provider::Anthropic.api_key_env()),
    };

    let url = format!("{}/messages", Provider::Anthropic.base_url());

    let messages = vec![
        serde_json::json!({
            "role": "user",
            "content": user_input
        }),
    ];

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": messages
    });

    match ureq::post(&url)
        .set("x-api-key", &api_key)
        .set("anthropic-version", "2023-06-01")
        .set("Content-Type", "application/json")
        .send_json(body)
    {
        Ok(response) => {
            let json: serde_json::Value = response.into_json().unwrap_or_default();
            json["content"][0]["text"]
                .as_str()
                .unwrap_or("No response")
                .to_string()
        }
        Err(e) => {
            format!("⚠️  Anthropic error: {}", e)
        }
    }
}

fn chat_gemini(model: &str, _system_prompt: &str, user_input: &str) -> String {
    let api_key = match env::var(Provider::Gemini.api_key_env()) {
        Ok(key) => key,
        Err(_) => return format!("⚠️  API key not set. Set {}.", Provider::Gemini.api_key_env()),
    };

    let url = format!(
        "{}/models/{}:generateContent?key={}",
        Provider::Gemini.base_url(),
        model,
        api_key
    );

    let body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": user_input
            }]
        }]
    });

    match ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(body)
    {
        Ok(response) => {
            let json: serde_json::Value = response.into_json().unwrap_or_default();
            json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or("No response")
                .to_string()
        }
        Err(e) => {
            format!("⚠️  Gemini error: {}", e)
        }
    }
}
