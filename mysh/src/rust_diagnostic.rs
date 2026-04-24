use std::process::Command;

/// Rust 编译器错误诊断器
pub struct RustDiagnostic;

impl RustDiagnostic {
    /// 运行 cargo check 并分析错误
    pub fn check() -> Vec<Diagnostic> {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--message-format=json")
            .output();

        match output {
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                parse_diagnostics(&stderr)
            }
            Err(_) => vec![],
        }
    }

    /// 分析编译错误并给出建议
    pub fn analyze_error(error_msg: &str) -> String {
        let mut suggestions = Vec::new();

        // 常见错误模式匹配
        if error_msg.contains("cannot find value") {
            suggestions.push("检查变量名是否拼写正确");
            suggestions.push("检查变量是否在当前作用域内定义");
        }

        if error_msg.contains("cannot find function") {
            suggestions.push("检查函数名是否拼写正确");
            suggestions.push("检查是否需要导入相应的模块");
        }

        if error_msg.contains("expected") && error_msg.contains("found") {
            suggestions.push("检查类型是否匹配");
            suggestions.push("考虑使用类型转换或泛型");
        }

        if error_msg.contains("borrowed value does not live long enough") {
            suggestions.push("考虑延长变量的生命周期");
            suggestions.push("使用 Clone 或 to_owned() 创建独立副本");
        }

        if error_msg.contains("use of moved value") {
            suggestions.push("值已被移动，考虑使用引用 (&)");
            suggestions.push("或者实现 Clone trait 并调用.clone()");
        }

        if error_msg.contains("trait bound") {
            suggestions.push("实现所需的 trait");
            suggestions.push("检查泛型约束是否正确");
        }

        if error_msg.contains("unresolved import") {
            suggestions.push("检查模块路径是否正确");
            suggestions.push("检查 Cargo.toml 中是否添加了对应的依赖");
        }

        if error_msg.contains("unused variable") {
            suggestions.push("如果不需要该变量，使用 _ 前缀：_var");
            suggestions.push("或者使用 #[allow(unused_variables)] 属性");
        }

        if error_msg.contains("mutable reference") {
            suggestions.push("检查是否需要可变绑定：let mut");
            suggestions.push("确保没有多个可变引用同时存在");
        }

        if suggestions.is_empty() {
            suggestions.push("查看完整的错误信息");
            suggestions.push("参考 Rust 错误索引：https://doc.rust-lang.org/error-index.html");
        }

        format!("错误分析:\n{}\n\n建议:\n{}", 
            error_msg.lines().take(5).collect::<Vec<_>>().join("\n"),
            suggestions.iter().map(|s| format!("  • {}", s)).collect::<Vec<_>>().join("\n")
        )
    }

    /// 运行 cargo clippy 并获取建议
    pub fn clippy() -> Vec<String> {
        let output = Command::new("cargo")
            .arg("clippy")
            .arg("--message-format=short")
            .output();

        match output {
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                stderr.lines()
                    .filter(|l| l.contains("warning:") || l.contains("error:"))
                    .map(|l| l.to_string())
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    /// 获取 Rust 诊断帮助
    pub fn get_rust_help() -> &'static str {
        r#"Rust 编译错误诊断

常用命令:
  rustfix         - 自动修复可修复的错误
  cargo clippy    - 代码风格检查
  cargo fmt       - 代码格式化

常见错误:
  E0382 - use of moved value
  E0597 - borrowed value does not live long enough
  E0425 - cannot find value
  E0433 - cannot find function/type

在线资源:
  Rust 错误索引：https://doc.rust-lang.org/error-index.html
  Rust by Example: https://doc.rust-lang.org/rust-by-example/
"#
    }
}

/// 诊断信息
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
}

/// 解析 JSON 格式的诊断信息
fn parse_diagnostics(output: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for line in output.lines() {
        if line.trim().starts_with('{') {
            // 简单的 JSON 解析
            if let Some(msg) = extract_json_string(line, "message") {
                let mut diag = Diagnostic {
                    message: msg,
                    file: String::new(),
                    line: 0,
                    column: 0,
                    severity: "error".to_string(),
                };

                if let Some(span) = extract_json_object(line, "span") {
                    if let Some(file) = extract_json_string(span, "file_name") {
                        diag.file = file;
                    }
                    if let Some(line) = extract_json_number(span, "line_start") {
                        diag.line = line as u32;
                    }
                }

                diagnostics.push(diag);
            }
        }
    }

    diagnostics
}

/// 从 JSON 行中提取字符串值
fn extract_json_string(line: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = line.find(&pattern) {
        let rest = &line[pos + pattern.len()..];
        let rest = rest.trim_start();
        if rest.starts_with('"') {
            let rest = &rest[1..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// 从 JSON 行中提取对象
fn extract_json_object<'a>(line: &'a str, key: &'a str) -> Option<&'a str> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = line.find(&pattern) {
        let rest = &line[pos + pattern.len()..];
        let rest = rest.trim_start();
        if rest.starts_with('{') {
            let mut depth = 0;
            for (i, c) in rest.chars().enumerate() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&rest[..=i]);
                    }
                }
            }
        }
    }
    None
}

/// 从 JSON 行中提取数字值
fn extract_json_number(line: &str, key: &str) -> Option<i64> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = line.find(&pattern) {
        let rest = &line[pos + pattern.len()..];
        let rest = rest.trim_start();
        let num_str: String = rest.chars()
            .take_while(|c| c.is_numeric() || *c == '-')
            .collect();
        if !num_str.is_empty() {
            return num_str.parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_string() {
        let line = r#"{"message":"error","span":{"file_name":"test.rs"}}"#;
        assert_eq!(extract_json_string(line, "message"), Some("error".to_string()));
        assert_eq!(extract_json_string(line, "file_name"), Some("test.rs".to_string()));
    }
}
