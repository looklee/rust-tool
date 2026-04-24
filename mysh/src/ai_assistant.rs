use std::env;
use std::process::Command;

/// AI 模型提供商
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelProvider {
    OpenAI,
    Ollama,
}

/// AI 智能助手配置
#[derive(Debug, Clone)]
pub struct AIConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub enabled: bool,
    pub provider: ModelProvider,
}

impl Default for AIConfig {
    fn default() -> Self {
        let model = env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        let provider = if model.to_lowercase().starts_with("llama") || 
                        model.to_lowercase().starts_with("qwen") ||
                        model.to_lowercase().starts_with("mistral") {
            ModelProvider::Ollama
        } else {
            ModelProvider::OpenAI
        };

        Self {
            api_key: env::var("OPENAI_API_KEY").unwrap_or_default(),
            base_url: env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| {
                    if provider == ModelProvider::Ollama {
                        "http://localhost:11434".to_string()
                    } else {
                        "https://api.openai.com/v1".to_string()
                    }
                }),
            model,
            enabled: !env::var("OPENAI_API_KEY").unwrap_or_default().is_empty() || 
                     provider == ModelProvider::Ollama,
            provider,
        }
    }
}

impl AIConfig {
    /// 检查是否使用 Ollama
    pub fn is_ollama(&self) -> bool {
        self.provider == ModelProvider::Ollama
    }

    /// 获取模型信息字符串
    pub fn model_info(&self) -> String {
        match self.provider {
            ModelProvider::OpenAI => format!("{} (OpenAI 兼容)", self.model),
            ModelProvider::Ollama => format!("{} (Ollama 本地)", self.model),
        }
    }
}

/// 智能 Shell 助手
pub struct AIAssistant {
    config: AIConfig,
    /// 会话历史（最近的命令和结果）
    conversation_history: Vec<ConversationTurn>,
}

/// 会话回合
#[derive(Clone)]
struct ConversationTurn {
    user_input: String,
    ai_response: String,
}

impl AIAssistant {
    pub fn new() -> Self {
        Self {
            config: AIConfig::default(),
            conversation_history: Vec::new(),
        }
    }

    /// 添加会话记录
    pub fn add_to_history(&mut self, input: String, response: String) {
        self.conversation_history.push(ConversationTurn {
            user_input: input,
            ai_response: response,
        });
        // 保留最近 10 条记录
        if self.conversation_history.len() > 10 {
            self.conversation_history.remove(0);
        }
    }

    /// 获取上下文相关的建议
    pub fn get_contextual_suggestion(&self, current_input: &str) -> Option<String> {
        if self.conversation_history.is_empty() {
            return None;
        }

        // 基于最近的对话提供建议
        let recent = self.conversation_history.last()?;
        
        // 简单的上下文匹配逻辑
        if current_input.is_empty() {
            // 用户没有输入，基于上一个命令提供后续建议
            Some(format!("上一个操作是：{}，你可能想要：", recent.user_input))
        } else {
            None
        }
    }

    pub fn config(&self) -> &AIConfig {
        &self.config
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// 自然语言转命令
    pub fn nl_to_command(&mut self, intent: &str) -> Result<String, String> {
        if !self.config.enabled {
            return Err("AI 未启用：请设置 OPENAI_API_KEY 环境变量".to_string());
        }

        // 构建包含上下文的 prompt
        let context_prompt = self.build_context_prompt();
        
        let prompt = format!(
            r#"{}你是一个 Shell 命令生成助手。用户会用自然语言描述他们想做的事，你只需要返回对应的 Shell 命令，不要解释。

示例：
用户：查看当前目录下所有的 Rust 文件
你：ls *.rs

用户：查找包含 "main" 的文件
你：grep -r "main" --include="*.rs"

用户：统计当前目录的文件数量
你：find . -type f | wc -l

用户：{}
你："#,
            context_prompt,
            intent
        );

        let result = self.call_llm(&prompt);
        
        // 记录到历史
        if let Ok(ref cmd) = result {
            self.add_to_history(intent.to_string(), cmd.clone());
        }
        
        result
    }

    /// 构建上下文 prompt
    fn build_context_prompt(&self) -> String {
        if self.conversation_history.is_empty() {
            return String::new();
        }

        let mut context = String::from("最近的对话历史：\n");
        for (i, turn) in self.conversation_history.iter().enumerate() {
            context.push_str(&format!("  {}. 用户：{} -> AI: {}\n", i + 1, turn.user_input, turn.ai_response));
        }
        context.push_str("\n基于以上上下文，请保持命令风格一致。\n\n");
        context
    }

    /// 错误诊断
    pub fn diagnose_error(&self, command: &str, error: &str) -> Result<String, String> {
        if !self.config.enabled {
            return Err("AI 未启用".to_string());
        }

        let prompt = format!(
            r#"你是一个 Shell 错误诊断专家。分析以下命令执行失败的原因，并给出修复建议。

命令：{}
错误信息：{}

请简洁地说明：
1. 错误原因
2. 修复建议（如适用，给出修正后的命令）"#,
            command, error
        );

        self.call_llm(&prompt)
    }

    /// 命令推荐（基于上下文）
    pub fn recommend_command(&self, context: &str, goal: &str) -> Result<String, String> {
        if !self.config.enabled {
            return Err("AI 未启用".to_string());
        }

        let prompt = format!(
            r#"你是一个 Shell 专家。根据用户的上下文和目标，推荐最合适的命令。

当前上下文：{}
用户目标：{}

请推荐 1-3 个命令（每个一行），按推荐度排序。"#,
            context, goal
        );

        self.call_llm(&prompt)
    }

    /// 解释命令
    pub fn explain_command(&self, command: &str) -> Result<String, String> {
        if !self.config.enabled {
            return Err("AI 未启用".to_string());
        }

        let prompt = format!(
            r#"请解释以下 Shell 命令的作用，用简洁的中文说明。

命令：{}

解释："#,
            command
        );

        self.call_llm(&prompt)
    }

    /// 调用 LLM API
    fn call_llm(&self, prompt: &str) -> Result<String, String> {
        if self.config.is_ollama() {
            self.call_ollama(prompt)
        } else {
            self.call_openai(prompt)
        }
    }

    /// 调用 Ollama API
    fn call_ollama(&self, prompt: &str) -> Result<String, String> {
        // Ollama API 格式
        let request_body = format!(
            r#"{{"model":"{}","prompt":"{}","stream":false}}"#,
            self.config.model,
            prompt.replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
        );

        let output = Command::new("curl")
            .arg("-s")
            .arg("-X")
            .arg("POST")
            .arg(&format!("{}/api/generate", self.config.base_url))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(&request_body)
            .output();

        match output {
            Ok(out) => {
                let response = String::from_utf8_lossy(&out.stdout);
                // Ollama 响应格式：{"model":"...","response":"...","done":true}
                if let Some(content) = extract_json_string(&response, "response") {
                    Ok(content)
                } else {
                    Err(format!("Ollama 响应解析失败：{}", response))
                }
            }
            Err(e) => Err(format!("调用 Ollama 失败：{} (请确保 Ollama 正在运行：ollama serve)", e)),
        }
    }

    /// 调用 OpenAI 兼容 API
    fn call_openai(&self, prompt: &str) -> Result<String, String> {
        let request_body = format!(
            r#"{{"model":"{}","messages":[{{"role":"user","content":"{}"}}]}}"#,
            self.config.model,
            prompt.replace('"', "\\\"").replace('\n', "\\n")
        );

        let output = Command::new("curl")
            .arg("-s")
            .arg("-X")
            .arg("POST")
            .arg(&format!("{}/chat/completions", self.config.base_url))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-H")
            .arg(&format!("Authorization: Bearer {}", self.config.api_key))
            .arg("-d")
            .arg(&request_body)
            .output();

        match output {
            Ok(out) => {
                let response = String::from_utf8_lossy(&out.stdout);
                // 简单的 JSON 解析，提取 content
                if let Some(content) = extract_json_string(&response, "content") {
                    Ok(content)
                } else {
                    Err(format!("API 响应解析失败：{}", response))
                }
            }
            Err(e) => Err(format!("调用 API 失败：{}", e)),
        }
    }
}

/// 从 JSON 中提取字符串值
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let key_pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&key_pattern) {
        let rest = &json[pos + key_pattern.len()..];
        let rest = rest.trim_start();
        if rest.starts_with('"') {
            let rest = &rest[1..];
            if let Some(end) = rest.find('"') {
                let value = &rest[..end];
                // 处理转义
                return Some(value.replace("\\n", "\n").replace("\\\"", "\""));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_string() {
        let json = r#"{"choices":[{"message":{"content":"hello world"}}]}"#;
        assert_eq!(extract_json_string(json, "content"), Some("hello world".to_string()));
    }
}
