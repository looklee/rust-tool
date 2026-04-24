use std::fs;
use std::path::{Path, PathBuf};

/// Cargo 项目信息
#[derive(Debug, Default)]
pub struct CargoProject {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub has_tests: bool,
    pub has_benchmarks: bool,
    pub dependencies: Vec<String>,
    pub src_files: Vec<PathBuf>,
}

impl CargoProject {
    /// 尝试从当前目录加载 Cargo 项目信息
    pub fn load() -> Option<Self> {
        let manifest_path = Path::new("Cargo.toml");
        if !manifest_path.exists() {
            return None;
        }

        let mut project = CargoProject::default();
        
        // 解析 Cargo.toml
        if let Ok(content) = fs::read_to_string(manifest_path) {
            for line in content.lines() {
                let line = line.trim();
                
                if line.starts_with("name = ") {
                    project.name = extract_string_value(line).to_string();
                } else if line.starts_with("version = ") {
                    project.version = extract_string_value(line).to_string();
                } else if line.starts_with("edition = ") {
                    project.edition = extract_string_value(line).to_string();
                } else if line.starts_with("[dependencies]") {
                    // 开始读取依赖
                    continue;
                } else if !line.starts_with('[') && !line.is_empty() && !line.starts_with('#') {
                    // 可能是依赖项
                    if let Some(dep) = line.split('=').next() {
                        let dep = dep.trim();
                        if !dep.is_empty() && dep != "name" && dep != "version" && dep != "edition" {
                            project.dependencies.push(dep.to_string());
                        }
                    }
                }
            }
        }

        // 检查项目结构
        if let Ok(entries) = fs::read_dir("src") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    project.src_files.push(path);
                }
            }
        }

        // 检查是否有测试
        project.has_tests = Path::new("tests").exists() || 
                           project.src_files.iter().any(|p| p.file_name().map(|n| n.to_string_lossy().contains("test")).unwrap_or(false));
        
        // 检查是否有 benchmarks
        project.has_benchmarks = Path::new("benches").exists() || 
                                 Path::new("benches.rs").exists();

        Some(project)
    }

    /// 获取项目状态摘要
    pub fn summary(&self) -> String {
        let mut summary = format!("Cargo 项目：{}\n", self.name);
        summary.push_str(&format!("  版本：{}\n", self.version));
        summary.push_str(&format!("  Edition：{}\n", self.edition));
        summary.push_str(&format!("  依赖：{}\n", self.dependencies.len()));
        summary.push_str(&format!("  源文件：{}\n", self.src_files.len()));
        
        if self.has_tests {
            summary.push_str("  ✓ 包含测试\n");
        }
        if self.has_benchmarks {
            summary.push_str("  ✓ 包含基准测试\n");
        }
        
        summary
    }

    /// 获取推荐的 cargo 命令
    pub fn get_recommendations(&self) -> Vec<&'static str> {
        let mut recs = Vec::new();
        
        // 始终推荐
        recs.push("cargo check");
        recs.push("cargo build");
        
        // 有测试时推荐
        if self.has_tests {
            recs.push("cargo test");
        }
        
        // 有 benchmarks 时推荐
        if self.has_benchmarks {
            recs.push("cargo bench");
        }
        
        // 推荐格式化
        recs.push("cargo fmt");
        
        // 推荐 clippy
        recs.push("cargo clippy");
        
        recs
    }
}

/// 从 TOML 行中提取字符串值
fn extract_string_value(line: &str) -> &str {
    if let Some(pos) = line.find('=') {
        let value = line[pos + 1..].trim();
        value.trim_matches('"')
    } else {
        ""
    }
}

/// 检查当前目录是否是 Cargo 项目
pub fn is_cargo_project() -> bool {
    Path::new("Cargo.toml").exists()
}

/// 获取 Cargo 项目信息或返回错误消息
pub fn get_cargo_info() -> String {
    match CargoProject::load() {
        Some(project) => project.summary(),
        None => "当前目录不是 Cargo 项目".to_string(),
    }
}

/// 获取推荐的 cargo 命令
pub fn get_cargo_recommendations() -> Vec<String> {
    match CargoProject::load() {
        Some(project) => project.get_recommendations().iter().map(|s| s.to_string()).collect(),
        None => vec!["cargo init".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_string_value() {
        assert_eq!(extract_string_value("name = \"test\""), "test");
        assert_eq!(extract_string_value("version = \"1.0.0\""), "1.0.0");
    }
}
