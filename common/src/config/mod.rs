use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

use super::errors::{ToolError, ToolResult};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ToolConfig {
    pub ai: AiConfig,
    pub ui: UiConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_tokens: usize,
    pub temperature: f64,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct UiConfig {
    pub colors: bool,
    pub verbose: bool,
    pub yolo_mode: bool,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file: Option<String>,
}

impl ToolConfig {
    pub fn load() -> ToolResult<Self> {
        let mut config = Self::default();
        
        config.load_from_env();
        
        if let Ok(path) = Self::find_config_file() {
            config.load_from_file(&path)?;
        }
        
        Ok(config)
    }
    
    fn load_from_env(&mut self) {
        if let Ok(provider) = env::var("AI_PROVIDER") {
            self.ai.provider = provider;
        }
        if let Ok(model) = env::var("AI_MODEL") {
            self.ai.model = model;
        }
        if let Ok(api_key) = env::var("AI_API_KEY") {
            self.ai.api_key = Some(api_key);
        }
        if let Ok(base_url) = env::var("AI_BASE_URL") {
            self.ai.base_url = Some(base_url);
        }
        if let Ok(level) = env::var("LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(colors) = env::var("NO_COLOR") {
            self.ui.colors = colors.is_empty();
        }
    }
    
    fn load_from_file(&mut self, path: &Path) -> ToolResult<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::Config(format!("Failed to read config file: {}", e)))?;
        
        let file_config: Self = toml::from_str(&content)
            .map_err(|e| ToolError::Serialization(format!("Failed to parse config: {}", e)))?;
        
        if !file_config.ai.provider.is_empty() {
            self.ai.provider = file_config.ai.provider;
        }
        if !file_config.ai.model.is_empty() {
            self.ai.model = file_config.ai.model;
        }
        if file_config.ai.api_key.is_some() {
            self.ai.api_key = file_config.ai.api_key;
        }
        if file_config.ai.base_url.is_some() {
            self.ai.base_url = file_config.ai.base_url;
        }
        
        Ok(())
    }
    
    fn find_config_file() -> ToolResult<std::path::PathBuf> {
        let home_dir = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|e| ToolError::Config(format!("Cannot find home directory: {}", e)))?;
        
        let paths = [
            "./rust-tool.toml",
            "./.rust-tool.toml",
            &format!("{}/.rust-tool/config.toml", home_dir),
            &format!("{}/.config/rust-tool/config.toml", home_dir),
        ];
        
        for path in paths {
            let path = Path::new(path);
            if path.exists() {
                return Ok(path.to_path_buf());
            }
        }
        
        Err(ToolError::Config("Config file not found".to_string()))
    }
    
    pub fn save(&self, path: &Path) -> ToolResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ToolError::Serialization(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(path, content)
            .map_err(|e| ToolError::Config(format!("Failed to write config file: {}", e)))
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "qwen2.5-coder:7b".to_string(),
            api_key: None,
            base_url: None,
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
            file: None,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            colors: true,
            verbose: false,
            yolo_mode: false,
        }
    }
}