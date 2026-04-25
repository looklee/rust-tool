use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::process::Command;

/// A plugin definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub commands: Vec<String>,
    pub enabled: bool,
    pub config: HashMap<String, String>,
}

/// Plugin Manager - manages external modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManager {
    pub plugins: HashMap<String, Plugin>,
    pub dir: String,
}

impl PluginManager {
    pub fn new(dir: &str) -> Self {
        let path = format!("{}/plugins.json", dir);
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(manager) = serde_json::from_str::<PluginManager>(&content) {
                return manager;
            }
        }

        PluginManager {
            plugins: HashMap::new(),
            dir: dir.to_string(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Plugin) {
        self.plugins.insert(plugin.name.clone(), plugin);
    }

    /// Enable a plugin
    pub fn enable(&mut self, name: &str) -> bool {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a plugin
    pub fn disable(&mut self, name: &str) -> bool {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = false;
            true
        } else {
            false
        }
    }

    /// Execute a plugin command
    pub fn execute(&self, plugin_name: &str, command: &str, args: &[String]) -> Option<String> {
        let plugin = self.plugins.get(plugin_name)?;
        if !plugin.enabled {
            return Some(format!("Plugin '{}' is disabled", plugin_name));
        }

        if !plugin.commands.contains(&command.to_string()) {
            return Some(format!("Command '{}' not found in plugin '{}'", command, plugin_name));
        }

        // Execute via external script/command
        let script_path = format!("{}/plugins/{}/{}", self.dir, plugin_name, command);
        
        if fs::metadata(&script_path).is_ok() {
            let output = Command::new(&script_path)
                .args(args)
                .output();

            match output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
                    Some(format!("{}\n{}", stdout, stderr))
                }
                Err(e) => Some(format!("Execution error: {}", e)),
            }
        } else {
            // Built-in plugin commands
            match (plugin_name, command) {
                ("system", "info") => Some(Self::system_info()),
                ("system", "uptime") => Some(Self::uptime_info()),
                ("system", "net") => Some(Self::network_info()),
                ("code", "lint") => Some(Self::code_lint(args)),
                ("code", "format") => Some(Self::code_format(args)),
                ("web", "fetch") => Some(Self::web_fetch(args)),
                _ => Some(format!("Built-in command not implemented: {}/{}", plugin_name, command)),
            }
        }
    }

    /// List all plugins
    pub fn list(&self) -> String {
        let mut output = "📦 Plugins:\n".to_string();
        for (name, plugin) in &self.plugins {
            let status = if plugin.enabled { "✅" } else { "❌" };
            output.push_str(&format!(
                "  {} {} v{} - {} [{}]\n",
                status, name, plugin.version, plugin.description,
                plugin.commands.join(", ")
            ));
        }
        output
    }

    /// Get enabled plugins with their commands
    pub fn get_commands(&self) -> Vec<(String, String)> {
        let mut commands = Vec::new();
        for (name, plugin) in &self.plugins {
            if plugin.enabled {
                for cmd in &plugin.commands {
                    commands.push((format!("{}/{}", name, cmd), plugin.description.clone()));
                }
            }
        }
        commands
    }

    // Built-in plugin commands
    fn system_info() -> String {
        let output = Command::new("uname").arg("-a").output();
        match output {
            Ok(result) => {
                let info = String::from_utf8_lossy(&result.stdout);
                format!("🖥️ System Info:\n{}", info)
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    fn uptime_info() -> String {
        let output = Command::new("uptime").output();
        match output {
            Ok(result) => {
                let info = String::from_utf8_lossy(&result.stdout);
                format!("⏱️ Uptime:\n{}", info)
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    fn network_info() -> String {
        let output = Command::new("ip").arg("addr").output();
        match output {
            Ok(result) => {
                let info = String::from_utf8_lossy(&result.stdout);
                format!("🌐 Network:\n{}", info)
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    fn code_lint(args: &[String]) -> String {
        if args.is_empty() {
            return "Usage: /plugin code lint <file>".to_string();
        }
        let output = Command::new("cargo")
            .args(&["clippy", "--", "-A", "warnings"])
            .current_dir(&args[0])
            .output();
        match output {
            Ok(result) => {
                let output = String::from_utf8_lossy(&result.stdout);
                format!("🔍 Lint results:\n{}", output)
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    fn code_format(args: &[String]) -> String {
        if args.is_empty() {
            return "Usage: /plugin code format <dir>".to_string();
        }
        let output = Command::new("cargo")
            .args(&["fmt"])
            .current_dir(&args[0])
            .output();
        match output {
            Ok(result) => {
                if result.status.success() {
                    "✅ Formatted successfully".to_string()
                } else {
                    let err = String::from_utf8_lossy(&result.stderr);
                    format!("❌ Format error: {}", err)
                }
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    fn web_fetch(args: &[String]) -> String {
        if args.is_empty() {
            return "Usage: /plugin web fetch <url>".to_string();
        }
        let output = Command::new("curl")
            .args(&["-s", "-L", &args[0]])
            .output();
        match output {
            Ok(result) => {
                let content = String::from_utf8_lossy(&result.stdout);
                let preview = if content.len() > 500 {
                    format!("{}...", &content[..500])
                } else {
                    content.to_string()
                };
                format!("🌐 Fetched {}:\n{}", &args[0], preview)
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Save plugins to disk
    pub fn save(&self) {
        let path = format!("{}/plugins.json", self.dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_plugin() {
        let mut manager = PluginManager::new("/tmp");
        let mut config = HashMap::new();
        config.insert("key".to_string(), "value".to_string());
        manager.register(Plugin {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: "Test plugin".to_string(),
            author: "test".to_string(),
            commands: vec!["cmd1".to_string()],
            enabled: true,
            config,
        });
        assert_eq!(manager.plugins.len(), 1);
    }

    #[test]
    fn test_enable_disable() {
        let mut manager = PluginManager::new("/tmp");
        manager.register(Plugin {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: "Test".to_string(),
            author: "test".to_string(),
            commands: vec![],
            enabled: true,
            config: HashMap::new(),
        });
        manager.disable("test");
        assert!(!manager.plugins["test"].enabled);
        manager.enable("test");
        assert!(manager.plugins["test"].enabled);
    }

    #[test]
    fn test_list_plugins() {
        let mut manager = PluginManager::new("/tmp");
        manager.register(Plugin {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: "Test".to_string(),
            author: "test".to_string(),
            commands: vec!["cmd".to_string()],
            enabled: true,
            config: HashMap::new(),
        });
        let list = manager.list();
        assert!(list.contains("test"));
    }
}
