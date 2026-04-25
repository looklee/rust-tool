use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Code execution sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSandbox {
    pub dir: String,
    pub max_execution_time: u64,
    pub max_output_size: usize,
    pub allowed_languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub language: String,
}

impl CodeSandbox {
    pub fn new(dir: &str) -> Self {
        let sandbox_dir = format!("{}/sandbox", dir);
        fs::create_dir_all(&sandbox_dir).ok();

        CodeSandbox {
            dir: sandbox_dir,
            max_execution_time: 30,
            max_output_size: 1024 * 1024, // 1MB
            allowed_languages: vec![
                "bash".to_string(),
                "python3".to_string(),
                "rust".to_string(),
                "node".to_string(),
            ],
        }
    }

    /// Detect language from file extension or code content
    fn detect_language(&self, code: &str, filename: &str) -> Option<String> {
        if filename.ends_with(".rs") {
            return Some("rust".to_string());
        } else if filename.ends_with(".py") {
            return Some("python3".to_string());
        } else if filename.ends_with(".js") {
            return Some("node".to_string());
        } else if filename.ends_with(".sh") {
            return Some("bash".to_string());
        }

        // Try to detect from code content
        if code.starts_with("#!/bin/bash") || code.starts_with("#!/usr/bin/env bash") {
            return Some("bash".to_string());
        } else if code.starts_with("#!/usr/bin/env python") || code.starts_with("#!/usr/bin/python") {
            return Some("python3".to_string());
        } else if code.starts_with("#!/usr/bin/env node") {
            return Some("node".to_string());
        }

        // Heuristic detection
        if code.contains("fn main()") || code.contains("use std::") {
            return Some("rust".to_string());
        } else if code.contains("import ") || code.contains("def ") || code.contains("print(") {
            return Some("python3".to_string());
        } else if code.contains("console.log") || code.contains("const ") || code.contains("let ") {
            return Some("node".to_string());
        } else if code.contains("echo ") || code.contains("if [") || code.contains("for ") {
            return Some("bash".to_string());
        }

        None
    }

    /// Execute code in the sandbox
    pub fn execute(&self, code: &str, language: Option<&str>) -> ExecutionResult {
        let start = std::time::Instant::now();

        let lang = language
            .map(|l| l.to_string())
            .or_else(|| self.detect_language(code, "code"))
            .unwrap_or_else(|| "bash".to_string());

        if !self.allowed_languages.contains(&lang) {
            return ExecutionResult {
                success: false,
                output: String::new(),
                error: format!("Language '{}' is not allowed", lang),
                exit_code: -1,
                duration_ms: start.elapsed().as_millis() as u64,
                language: lang,
            };
        }

        // Create temp file
        let filename = match lang.as_str() {
            "rust" => "code.rs",
            "python3" => "code.py",
            "node" => "code.js",
            "bash" => "code.sh",
            _ => "code.txt",
        };

        let file_path = PathBuf::from(&self.dir).join(filename);

        // Write code to file
        if let Err(e) = fs::write(&file_path, code) {
            return ExecutionResult {
                success: false,
                output: String::new(),
                error: format!("Failed to write file: {}", e),
                exit_code: -1,
                duration_ms: start.elapsed().as_millis() as u64,
                language: lang,
            };
        }

        // Execute based on language
        let result = match lang.as_str() {
            "bash" => self.execute_bash(&file_path),
            "python3" => self.execute_python(&file_path),
            "rust" => self.execute_rust(&file_path),
            "node" => self.execute_node(&file_path),
            _ => {
                return ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: format!("Unsupported language: {}", lang),
                    exit_code: -1,
                    duration_ms: start.elapsed().as_millis() as u64,
                    language: lang,
                }
            }
        };

        // Clean up
        fs::remove_file(&file_path).ok();

        let mut result = result;
        result.language = lang;
        result.duration_ms = start.elapsed().as_millis() as u64;

        // Truncate output if too large
        if result.output.len() > self.max_output_size {
            result.output.truncate(self.max_output_size);
            result.output.push_str("\n... [output truncated]");
        }

        result
    }

    fn execute_bash(&self, file_path: &PathBuf) -> ExecutionResult {
        let output = Command::new("bash")
            .arg(file_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(result) => ExecutionResult {
                success: result.status.success(),
                output: String::from_utf8_lossy(&result.stdout).to_string(),
                error: String::from_utf8_lossy(&result.stderr).to_string(),
                exit_code: result.status.code().unwrap_or(-1),
                duration_ms: 0,
                language: "bash".to_string(),
            },
            Err(e) => ExecutionResult {
                success: false,
                output: String::new(),
                error: format!("Execution failed: {}", e),
                exit_code: -1,
                duration_ms: 0,
                language: "bash".to_string(),
            },
        }
    }

    fn execute_python(&self, file_path: &PathBuf) -> ExecutionResult {
        let output = Command::new("python3")
            .arg(file_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(result) => ExecutionResult {
                success: result.status.success(),
                output: String::from_utf8_lossy(&result.stdout).to_string(),
                error: String::from_utf8_lossy(&result.stderr).to_string(),
                exit_code: result.status.code().unwrap_or(-1),
                duration_ms: 0,
                language: "python3".to_string(),
            },
            Err(e) => ExecutionResult {
                success: false,
                output: String::new(),
                error: format!("Execution failed: {}", e),
                exit_code: -1,
                duration_ms: 0,
                language: "python3".to_string(),
            },
        }
    }

    fn execute_rust(&self, file_path: &PathBuf) -> ExecutionResult {
        let binary_path = file_path.with_extension("");

        // Compile
        let compile = Command::new("rustc")
            .arg(file_path)
            .arg("-o")
            .arg(&binary_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match compile {
            Ok(result) if !result.status.success() => {
                return ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: String::from_utf8_lossy(&result.stderr).to_string(),
                    exit_code: result.status.code().unwrap_or(-1),
                    duration_ms: 0,
                    language: "rust".to_string(),
                };
            }
            Err(e) => {
                return ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: format!("Compilation failed: {}", e),
                    exit_code: -1,
                    duration_ms: 0,
                    language: "rust".to_string(),
                };
            }
            _ => {}
        }

        // Run
        let output = Command::new(&binary_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        fs::remove_file(&binary_path).ok();

        match output {
            Ok(result) => ExecutionResult {
                success: result.status.success(),
                output: String::from_utf8_lossy(&result.stdout).to_string(),
                error: String::from_utf8_lossy(&result.stderr).to_string(),
                exit_code: result.status.code().unwrap_or(-1),
                duration_ms: 0,
                language: "rust".to_string(),
            },
            Err(e) => ExecutionResult {
                success: false,
                output: String::new(),
                error: format!("Execution failed: {}", e),
                exit_code: -1,
                duration_ms: 0,
                language: "rust".to_string(),
            },
        }
    }

    fn execute_node(&self, file_path: &PathBuf) -> ExecutionResult {
        let output = Command::new("node")
            .arg(file_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(result) => ExecutionResult {
                success: result.status.success(),
                output: String::from_utf8_lossy(&result.stdout).to_string(),
                error: String::from_utf8_lossy(&result.stderr).to_string(),
                exit_code: result.status.code().unwrap_or(-1),
                duration_ms: 0,
                language: "node".to_string(),
            },
            Err(e) => ExecutionResult {
                success: false,
                output: String::new(),
                error: format!("Execution failed: {}", e),
                exit_code: -1,
                duration_ms: 0,
                language: "node".to_string(),
            },
        }
    }

    /// Format execution result for display
    pub fn format_result(&self, result: &ExecutionResult) -> String {
        let status = if result.success { "✅" } else { "❌" };
        let mut output = format!(
            "{} {} execution ({}ms)\n",
            status, result.language, result.duration_ms
        );

        if !result.output.is_empty() {
            output.push_str(&format!("Output:\n{}\n", result.output));
        }

        if !result.error.is_empty() {
            output.push_str(&format!("Error:\n{}\n", result.error));
        }

        output.push_str(&format!("Exit code: {}", result.exit_code));
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_bash() {
        let sandbox = CodeSandbox::new("/tmp");
        assert_eq!(sandbox.detect_language("echo hello", "code.sh"), Some("bash".to_string()));
        assert_eq!(sandbox.detect_language("#!/bin/bash\necho hello", "code"), Some("bash".to_string()));
    }

    #[test]
    fn test_detect_language_python() {
        let sandbox = CodeSandbox::new("/tmp");
        assert_eq!(sandbox.detect_language("print('hello')", "code.py"), Some("python3".to_string()));
        assert_eq!(sandbox.detect_language("import os\nprint('hello')", "code"), Some("python3".to_string()));
    }

    #[test]
    fn test_detect_language_rust() {
        let sandbox = CodeSandbox::new("/tmp");
        assert_eq!(sandbox.detect_language("fn main() {}", "code.rs"), Some("rust".to_string()));
        assert_eq!(sandbox.detect_language("use std::fs;\nfn main() {}", "code"), Some("rust".to_string()));
    }

    #[test]
    fn test_detect_language_node() {
        let sandbox = CodeSandbox::new("/tmp");
        assert_eq!(sandbox.detect_language("console.log('hello')", "code.js"), Some("node".to_string()));
    }

    #[test]
    fn test_execute_bash() {
        let sandbox = CodeSandbox::new("/tmp");
        let result = sandbox.execute("echo 'Hello World'", Some("bash"));
        assert!(result.success);
        assert!(result.output.contains("Hello World"));
    }

    #[test]
    fn test_execute_python() {
        let sandbox = CodeSandbox::new("/tmp");
        let result = sandbox.execute("print('Hello from Python')", Some("python3"));
        assert!(result.success);
        assert!(result.output.contains("Hello from Python"));
    }

    #[test]
    fn test_execute_rust() {
        let sandbox = CodeSandbox::new("/tmp");
        let code = r#"fn main() { println!("Hello from Rust"); }"#;
        let result = sandbox.execute(code, Some("rust"));
        assert!(result.success);
        assert!(result.output.contains("Hello from Rust"));
    }

    #[test]
    fn test_unsupported_language() {
        let sandbox = CodeSandbox::new("/tmp");
        let result = sandbox.execute("code", Some("java"));
        assert!(!result.success);
        assert!(result.error.contains("not allowed"));
    }

    #[test]
    fn test_format_result() {
        let sandbox = CodeSandbox::new("/tmp");
        let result = ExecutionResult {
            success: true,
            output: "Hello".to_string(),
            error: String::new(),
            exit_code: 0,
            duration_ms: 100,
            language: "bash".to_string(),
        };
        let formatted = sandbox.format_result(&result);
        assert!(formatted.contains("✅"));
        assert!(formatted.contains("bash"));
        assert!(formatted.contains("Hello"));
    }
}
