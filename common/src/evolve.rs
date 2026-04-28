use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub path: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub lines_of_code: usize,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EvolutionReport {
    pub tool_name: String,
    pub diagnosis: String,
    pub improvements: Vec<String>,
    pub new_features: Vec<String>,
    pub priority: EvolutionPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvolutionPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub enum EvolutionAction {
    Diagnose(String),
    Improve(String, String),
    Create(String, String),
    Remove(String),
    Merge(Vec<String>, String),
}

pub struct EvolutionEngine {
    tools: HashMap<String, ToolInfo>,
    base_dir: String,
}

impl EvolutionEngine {
    pub fn new(base_dir: &str) -> Self {
        Self {
            tools: HashMap::new(),
            base_dir: base_dir.to_string(),
        }
    }

    pub fn scan_tools(&mut self) -> Result<usize, String> {
        self.tools.clear();
        let mut count = 0;

        let entries = fs::read_dir(&self.base_dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if name.starts_with('.') || name == "target" || name == "common" || name == "code" {
                continue;
            }

            let cargo_toml = path.join("Cargo.toml");
            if !cargo_toml.exists() {
                continue;
            }

            let tool_info = self.analyze_tool(&name, &path)?;
            self.tools.insert(name.clone(), tool_info);
            count += 1;
        }

        Ok(count)
    }

    fn analyze_tool(&self, name: &str, path: &Path) -> Result<ToolInfo, String> {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        let mut dependencies = Vec::new();
        let mut lines_of_code = 0;
        let mut description = String::new();

        let cargo_path = path.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            for line in content.lines() {
                if line.starts_with("description") {
                    description = line.split('=').nth(1)
                        .unwrap_or("")
                        .trim_matches('"')
                        .trim()
                        .to_string();
                }
                if line.contains("dependencies") || line.contains("dep:") {
                    if line.contains('=') && !line.starts_with('[') {
                        let dep = line.split('=').next().unwrap_or("").trim();
                        if !dep.is_empty() && !["version", "optional"].contains(&dep) {
                            dependencies.push(dep.to_string());
                        }
                    }
                }
            }

            if dependencies.is_empty() {
                issues.push("No dependencies declared".to_string());
                suggestions.push("Consider using 'common' library for shared code".to_string());
            }

            if description.is_empty() {
                issues.push("Missing description".to_string());
                suggestions.push("Add a description to Cargo.toml".to_string());
            }
        }

        let src_dir = path.join("src");
        if src_dir.exists() {
            for entry in walkdir::WalkDir::new(&src_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let file_path = entry.path();
                if file_path.is_file() && file_path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Ok(content) = fs::read_to_string(file_path) {
                        lines_of_code += content.lines().count();

                        if content.contains("TODO") || content.contains("FIXME") {
                            issues.push("Contains TODO/FIXME comments".to_string());
                        }

                        if content.contains("unwrap()") && !content.contains("expect(") {
                            suggestions.push("Replace unwrap() with expect() or proper error handling".to_string());
                        }

                        if content.contains("print!") && !content.contains("eprint!") {
                            suggestions.push("Consider using eprintln! for errors instead of print!".to_string());
                        }
                    }
                }
            }
        }

        Ok(ToolInfo {
            name: name.to_string(),
            path: path.display().to_string(),
            description,
            dependencies,
            lines_of_code,
            issues,
            suggestions,
        })
    }

    pub fn diagnose_tool(&self, name: &str) -> Option<EvolutionReport> {
        let tool = self.tools.get(name)?;

        let mut improvements = Vec::new();
        let mut new_features = Vec::new();

        for issue in &tool.issues {
            match issue.as_str() {
                "No dependencies declared" => {
                    improvements.push("Add common library dependency for shared utilities".to_string());
                }
                "Missing description" => {
                    improvements.push("Add description to Cargo.toml".to_string());
                }
                "Contains TODO/FIXME comments" => {
                    improvements.push("Address TODO/FIXME comments before production".to_string());
                }
                _ => {
                    improvements.push(format!("Investigate issue: {}", issue));
                }
            }
        }

        improvements.extend(tool.suggestions.clone());

        if tool.lines_of_code > 1000 {
            new_features.push("Consider splitting into smaller modules".to_string());
        }

        if tool.dependencies.len() > 10 {
            new_features.push("Review dependencies - some may be unnecessary".to_string());
        }

        let priority = if !tool.issues.is_empty() {
            EvolutionPriority::High
        } else {
            EvolutionPriority::Medium
        };

        Some(EvolutionReport {
            tool_name: tool.name.clone(),
            diagnosis: format!("Analyzed {} ({} lines of code, {} dependencies)",
                tool.name, tool.lines_of_code, tool.dependencies.len()),
            improvements,
            new_features,
            priority,
        })
    }

    pub fn diagnose_all(&self) -> Vec<EvolutionReport> {
        self.tools.keys()
            .filter_map(|name| self.diagnose_tool(name))
            .collect()
    }

    pub fn get_tools(&self) -> &HashMap<String, ToolInfo> {
        &self.tools
    }

    pub fn generate_improved_code(&self, name: &str, improvement: &str) -> Option<String> {
        let tool = self.tools.get(name)?;
        let src_dir = Path::new(&tool.path).join("src");
        let main_rs = src_dir.join("main.rs");

        let current_code = if main_rs.exists() {
            fs::read_to_string(&main_rs).ok()?
        } else {
            return None;
        };

        Some(format!(
            "/* Improved version of {} */\n/* Improvement: {} */\n\n{}",
            name, improvement, current_code
        ))
    }

    pub fn create_new_tool(&self, name: &str, description: &str, template: &str) -> String {
        let tool_dir = Path::new(&self.base_dir).join(name);
        let src_dir = tool_dir.join("src");

        let _ = fs::create_dir_all(&src_dir);

        let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
description = "{}"

[dependencies]
common = {{ path = "../common" }}
"#,
            name, description
        );

        let main_rs = format!(r#"use common::{{Colors, is_terminal}};

fn main() {{
    let colors = Colors::auto();
    
    println!("{{}}Tool: {}{}", colors.blue(), colors.reset());
    println!("{{}}Description: {}{}", colors.cyan(), colors.reset());
    
    // TODO: Implement the tool functionality
    // Template: {}
}}

#[cfg(test)]
mod tests {{
    #[test]
    fn test_basic() {{
        assert!(true);
    }}
}}
"#,
            name, description, template
        );

        let _ = fs::write(tool_dir.join("Cargo.toml"), cargo_toml);
        let _ = fs::write(src_dir.join("main.rs"), main_rs);

        format!("Created tool '{}' at {}\n\nFiles created:\n  Cargo.toml\n  src/main.rs", name, tool_dir.display())
    }

    pub fn suggest_new_tools(&self) -> Vec<(String, String)> {
        let mut suggestions = Vec::new();

        let has_grep = self.tools.contains_key("grep");
        let has_find = self.tools.contains_key("find");

        if !has_grep {
            suggestions.push((
                "rg".to_string(),
                "Ripgrep-compatible search tool with regex support".to_string()
            ));
        }

        if !has_find {
            suggestions.push((
                "locate".to_string(),
                "Fast file search using database index".to_string()
            ));
        }

        let has_json_tool = self.tools.values().any(|t| t.description.contains("json"));
        if !has_json_tool {
            suggestions.push((
                "jq".to_string(),
                "JSON processor and query tool".to_string()
            ));
        }

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_info_creation() {
        let tool = ToolInfo {
            name: "test".to_string(),
            path: "/test".to_string(),
            description: "A test tool".to_string(),
            dependencies: vec!["common".to_string()],
            lines_of_code: 100,
            issues: vec![],
            suggestions: vec!["Add tests".to_string()],
        };

        assert_eq!(tool.name, "test");
        assert_eq!(tool.lines_of_code, 100);
    }

    #[test]
    fn test_evolution_priority() {
        assert!(EvolutionPriority::Low < EvolutionPriority::Medium);
        assert!(EvolutionPriority::Medium < EvolutionPriority::High);
        assert!(EvolutionPriority::High < EvolutionPriority::Critical);
    }
}
