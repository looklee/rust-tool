use std::env;
use std::fs;
use std::path::Path;

/// 命令补全器
pub struct Completer {
    /// 内置命令列表
    builtins: Vec<String>,
    /// 历史命令
    history: Vec<String>,
}

impl Completer {
    pub fn new() -> Self {
        Self {
            builtins: vec![
                "exit".into(), "cd".into(), "pwd".into(), "echo".into(),
                "cat".into(), "head".into(), "tail".into(), "wc".into(),
                "sed".into(), "git".into(), "gstatus".into(), "gdiff".into(),
                "glog".into(), "find".into(), "grep".into(), "codeanalyze".into(),
                "ask".into(), "explain".into(), "fix".into(), "ollama".into(),
                "help".into(), "export".into(), "unset".into(), "jobs".into(),
                "fg".into(), "bg".into(), "kill".into(), "type".into(),
                "test".into(), "alias".into(), "if".into(), "for".into(),
                "while".into(),
            ],
            history: Vec::new(),
        }
    }

    /// 添加历史命令
    pub fn add_history(&mut self, cmd: String) {
        if !self.history.contains(&cmd) {
            self.history.push(cmd);
            // 保留最近 100 条
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
    }

    /// 获取补全建议
    pub fn complete(&self, input: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let input = input.to_lowercase();

        // 补全内置命令
        for builtin in &self.builtins {
            if builtin.to_lowercase().starts_with(&input) {
                suggestions.push(builtin.clone());
            }
        }

        // 补全历史命令
        for hist in &self.history {
            if hist.to_lowercase().starts_with(&input) && !suggestions.contains(hist) {
                suggestions.push(hist.clone());
            }
        }

        // 补全文件/目录
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.to_lowercase().starts_with(&input) && !suggestions.contains(&name) {
                        if entry.path().is_dir() {
                            suggestions.push(format!("{}/", name));
                        } else {
                            suggestions.push(name);
                        }
                    }
                }
            }
        }

        suggestions.sort();
        suggestions.dedup();
        suggestions
    }

    /// 智能提示（基于上下文）
    pub fn get_smart_suggestions(&self, partial_cmd: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // 如果输入包含 git，推荐 git 子命令
        if partial_cmd.contains("git") {
            suggestions.extend([
                "status", "diff", "log", "add", "commit", "push", "pull", "branch"
            ].iter().map(|s| format!("git {}", s)));
        }

        // 如果输入包含 cargo，推荐 cargo 子命令
        if partial_cmd.contains("cargo") {
            suggestions.extend([
                "build", "run", "test", "check", "fmt", "clippy"
            ].iter().map(|s| format!("cargo {}", s)));
        }

        suggestions
    }
}

/// 尝试读取当前 PATH 中的可执行文件
pub fn get_path_commands() -> Vec<String> {
    let mut commands = Vec::new();
    
    if let Ok(path_var) = env::var("PATH") {
        for path_dir in path_var.split(':') {
            let path = Path::new(path_dir);
            if path.is_dir() {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        if let Ok(name) = entry.file_name().into_string() {
                            // 只添加看起来像命令的文件
                            if !name.starts_with('.') && !name.contains('.') {
                                commands.push(name);
                            }
                        }
                    }
                }
            }
        }
    }
    
    commands.sort();
    commands.dedup();
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_builtins() {
        let completer = Completer::new();
        let suggestions = completer.complete("gi");
        assert!(suggestions.contains(&"git".to_string()));
    }
}
