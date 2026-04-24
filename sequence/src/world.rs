use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

/// Filesystem layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemConfig {
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        FilesystemConfig {
            allowed_paths: vec!["/".to_string()],
            denied_paths: vec![
                "/proc".to_string(),
                "/sys".to_string(),
            ],
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub enabled: bool,
    pub allowed_domains: Vec<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            enabled: true,
            allowed_domains: Vec::new(), // Empty = all allowed
        }
    }
}

/// World interface - bridge between SEQUENCE OS¹ and the external world
pub struct WorldInterface {
    filesystem: FilesystemConfig,
    network: NetworkConfig,
    current_dir: String,
}

impl WorldInterface {
    pub fn new(_data_dir: &str) -> Self {
        let current_dir = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());

        WorldInterface {
            filesystem: FilesystemConfig::default(),
            network: NetworkConfig::default(),
            current_dir,
        }
    }

    /// Check if a path is allowed
    fn is_path_allowed(&self, path: &str) -> bool {
        // Check denied paths first
        for denied in &self.filesystem.denied_paths {
            if path.starts_with(denied) {
                return false;
            }
        }

        // Check allowed paths
        for allowed in &self.filesystem.allowed_paths {
            if path.starts_with(allowed) {
                return true;
            }
        }

        false
    }

    /// List directory contents
    pub fn list_dir(&self, path: &str) -> String {
        if !self.is_path_allowed(path) {
            return format!("⛔ Access denied: {}", path);
        }

        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return format!("❌ Path does not exist: {}", path);
        }

        if !path_obj.is_dir() {
            return format!("❌ Not a directory: {}", path);
        }

        match fs::read_dir(path) {
            Ok(entries) => {
                let mut output = format!("📁 Contents of {}:\n", path);
                let mut count = 0;

                for entry in entries.take(100) {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let name = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "?".to_string());

                        let is_dir = path.is_dir();
                        let marker = if is_dir { "📁" } else { "📄" };
                        let size = if !is_dir {
                            fs::metadata(&path)
                                .map(|m| format!(" ({} bytes)", m.len()))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };

                        output.push_str(&format!("  {} {}{}\n", marker, name, size));
                        count += 1;
                    }
                }

                if count == 0 {
                    output.push_str("  (empty directory)\n");
                }

                output
            }
            Err(e) => format!("❌ Error reading directory: {}", e),
        }
    }

    /// Read a file
    pub fn read_file(&self, path: &str) -> String {
        if !self.is_path_allowed(path) {
            return format!("⛔ Access denied: {}", path);
        }

        match fs::read_to_string(path) {
            Ok(content) => {
                let len = content.len();
                let lines = content.lines().count();
                let preview = if len > 2000 {
                    format!("{}\n... (truncated, {} total bytes)", &content[..2000], len)
                } else {
                    content
                };

                format!(
                    "📄 {} ({} bytes, {} lines)\n{}\n{}",
                    path, len, lines, "─".repeat(40), preview
                )
            }
            Err(e) => format!("❌ Error reading file: {}", e),
        }
    }

    /// Execute a shell command
    pub fn execute_command(&self, cmd: &str) -> String {
        // Safety check - block dangerous commands
        let dangerous = ["rm -rf /", "mkfs", "dd if=", ">:()", "fork bomb"];
        for dangerous_cmd in &dangerous {
            if cmd.contains(dangerous_cmd) {
                return format!("⛔ Blocked dangerous command: {}", dangerous_cmd);
            }
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);

                let mut output = format!("🚀 Command: {}\n", cmd);
                output.push_str("─".repeat(40).as_str());
                output.push('\n');

                if !stdout.is_empty() {
                    let stdout_preview: String = stdout.lines()
                        .take(50)
                        .collect::<Vec<_>>()
                        .join("\n");
                    output.push_str(&format!("Output:\n{}\n", stdout_preview));
                    if stdout.lines().count() > 50 {
                        output.push_str(&format!("... ({} more lines)\n", stdout.lines().count() - 50));
                    }
                }

                if !stderr.is_empty() {
                    output.push_str(&format!("Errors:\n{}\n", stderr));
                }

                if result.status.success() {
                    output.push_str("✅ Exit code: 0\n");
                } else {
                    output.push_str(&format!("⚠️  Exit code: {:?}\n", result.status.code()));
                }

                output
            }
            Err(e) => format!("❌ Error executing command: {}", e),
        }
    }

    /// Search files for a pattern (in content OR filename)
    pub fn search_files(&self, path: &str, pattern: &str) -> String {
        if !self.is_path_allowed(path) {
            return format!("⛔ Access denied: {}", path);
        }

        let pattern_lower = pattern.to_lowercase();
        let mut results = Vec::new();

        for entry in WalkDir::new(path)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            let mut matched = false;

            // Check filename
            if let Some(name) = entry_path.file_name() {
                if name.to_string_lossy().to_lowercase().contains(&pattern_lower) {
                    matched = true;
                }
            }

            // Check file content (only for files)
            if !matched && entry_path.is_file() {
                // Skip binary files and large files
                if let Ok(metadata) = entry_path.metadata() {
                    if metadata.len() > 1_000_000 {
                        continue;
                    }
                }

                // Skip common binary extensions
                if let Some(ext) = entry_path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if ["png", "jpg", "jpeg", "gif", "bmp", "ico", "pdf", "zip", "tar", "gz", "exe", "so", "dylib"].contains(&ext.as_str()) {
                        continue;
                    }
                }

                if let Ok(content) = fs::read_to_string(entry_path) {
                    if content.to_lowercase().contains(&pattern_lower) {
                        matched = true;
                    }
                }
            }

            if matched {
                results.push(entry_path.display().to_string());
                if results.len() >= 50 {
                    break;
                }
            }
        }

        if results.is_empty() {
            format!("🔍 未在 {} 中找到包含 '{}' 的文件", path, pattern)
        } else {
            let mut output = format!("🔍 在 {} 中找到 {} 个匹配 '{}':\n", path, results.len(), pattern);
            for result in &results {
                output.push_str(&format!("  • {}\n", result));
            }
            output
        }
    }

    pub fn current_dir(&self) -> &str {
        &self.current_dir
    }

    pub fn get_disk_usage(&self, path: &str) -> String {
        if !self.is_path_allowed(path) {
            return format!("⛔ Access denied: {}", path);
        }

        let mut total_size: u64 = 0;
        let mut file_count: u64 = 0;
        let mut dir_count: u64 = 0;

        for entry in WalkDir::new(path)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                dir_count += 1;
            } else if entry_path.is_file() {
                file_count += 1;
                if let Ok(metadata) = entry_path.metadata() {
                    total_size += metadata.len();
                }
            }
        }

        let size_str = if total_size > 1_000_000_000 {
            format!("{:.2} GB", total_size as f64 / 1_000_000_000.0)
        } else if total_size > 1_000_000 {
            format!("{:.2} MB", total_size as f64 / 1_000_000.0)
        } else if total_size > 1_000 {
            format!("{:.2} KB", total_size as f64 / 1_000.0)
        } else {
            format!("{} bytes", total_size)
        };

        format!(
            "📊 Disk Usage for {}:\n  Files: {}\n  Directories: {}\n  Total size: {}",
            path, file_count, dir_count, size_str
        )
    }
}

use std::env;
