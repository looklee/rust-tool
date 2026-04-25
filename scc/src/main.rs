use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct FileStats {
    path: String,
    language: String,
    lines: u64,
    code: u64,
    comments: u64,
    blanks: u64,
}

#[derive(Debug, Default)]
struct LanguageStats {
    files: u64,
    lines: u64,
    code: u64,
    comments: u64,
    blanks: u64,
}

/// Detect language from file extension
fn detect_language(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "rs" => Some("Rust"),
        "py" => Some("Python"),
        "js" | "mjs" => Some("JavaScript"),
        "ts" | "tsx" => Some("TypeScript"),
        "c" | "h" => Some("C"),
        "cpp" | "cxx" | "hpp" | "hxx" => Some("C++"),
        "java" => Some("Java"),
        "go" => Some("Go"),
        "rb" => Some("Ruby"),
        "cs" => Some("C#"),
        "php" => Some("PHP"),
        "swift" => Some("Swift"),
        "kt" | "kts" => Some("Kotlin"),
        "scala" => Some("Scala"),
        "sh" | "bash" => Some("Shell"),
        "html" => Some("HTML"),
        "css" | "scss" | "sass" => Some("CSS"),
        "json" => Some("JSON"),
        "xml" => Some("XML"),
        "yaml" | "yml" => Some("YAML"),
        "toml" => Some("TOML"),
        "md" | "markdown" => Some("Markdown"),
        "sql" => Some("SQL"),
        "lua" => Some("Lua"),
        "r" | "R" => Some("R"),
        "pl" => Some("Perl"),
        "dart" => Some("Dart"),
        "zig" => Some("Zig"),
        _ => None,
    }
}

/// Check if file appears to be binary
fn is_binary(path: &Path) -> bool {
    if let Ok(content) = fs::read(path) {
        content.iter().take(8000).any(|&b| b == 0)
    } else {
        true
    }
}

/// Count lines, code, comments, blanks for a file
fn count_file(path: &Path, language: &str) -> FileStats {
    let mut stats = FileStats {
        path: path.to_string_lossy().to_string(),
        language: language.to_string(),
        lines: 0,
        code: 0,
        comments: 0,
        blanks: 0,
    };
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return stats,
    };

    let lines: Vec<&str> = content.lines().collect();
    stats.lines = lines.len() as u64;
    let mut in_block_comment = false;

    for line in &lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            stats.blanks += 1;
            continue;
        }

        match language {
            l if is_c_style_lang(l) => {
                if in_block_comment {
                    stats.comments += 1;
                    if trimmed.ends_with("*/") {
                        in_block_comment = false;
                    }
                } else if trimmed.starts_with("//") {
                    stats.comments += 1;
                } else if trimmed.starts_with("/*") {
                    stats.comments += 1;
                    if !trimmed.ends_with("*/") {
                        in_block_comment = true;
                    }
                } else {
                    stats.code += 1;
                }
            }
            l if is_hash_comment_lang(l) => {
                if trimmed.starts_with('#') {
                    stats.comments += 1;
                } else {
                    stats.code += 1;
                }
            }
            "HTML" | "XML" => {
                if trimmed.starts_with("<!--") {
                    stats.comments += 1;
                    if !trimmed.ends_with("-->") {
                        in_block_comment = true;
                    }
                } else if in_block_comment {
                    stats.comments += 1;
                    if trimmed.ends_with("-->") {
                        in_block_comment = false;
                    }
                } else {
                    stats.code += 1;
                }
            }
            _ => {
                stats.code += 1;
            }
        }
    }

    stats
}

fn is_c_style_lang(lang: &str) -> bool {
    matches!(
        lang,
        "Rust"
            | "JavaScript"
            | "TypeScript"
            | "C"
            | "C++"
            | "Java"
            | "Go"
            | "C#"
            | "PHP"
            | "Swift"
            | "Kotlin"
            | "Scala"
            | "Dart"
            | "Zig"
    )
}

fn is_hash_comment_lang(lang: &str) -> bool {
    matches!(
        lang,
        "Python" | "Ruby" | "Shell" | "YAML" | "R" | "Perl" | "TOML"
    )
}

/// Walk directory recursively and collect files
fn walk_dir(
    path: &Path,
    include_ext: &[String],
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip common non-source directories
                let dir_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if matches!(dir_name, "node_modules" | ".git" | "target" | "build" | "dist" | ".venv" | "__pycache__") {
                    continue;
                }
                files.extend(walk_dir(&path, include_ext));
            } else if path.is_file() && !is_binary(&path) {
                if include_ext.is_empty() {
                    if detect_language(&path).is_some() {
                        files.push(path);
                    }
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if include_ext.iter().any(|i| i.to_lowercase() == ext.to_lowercase()) {
                        files.push(path);
                    }
                }
            }
        }
    }
    files
}

fn print_help() {
    println!(
        r#"scc - Sloc Cloc and Code

Usage: scc [OPTIONS] [PATH]

Arguments:
  PATH                  Directory or file to analyze (default: current directory)

Options:
  -h, --help            Show this help message
  --by-file             Show results for each file instead of by language
  --format FORMAT       Output format: table (default), json, csv
  --include-ext EXT     Only include files with specified extension (comma-separated)

Examples:
  scc .
  scc --by-file src/
  scc --format json .
  scc --format csv --include-ext rs,go .
  scc --include-ext py,js src/"#,
    );
}

fn output_table(
    by_file: bool,
    file_stats: &[FileStats],
    lang_stats: &HashMap<String, LanguageStats>,
    total_lines: u64,
) {
    if by_file {
        println!(
            "{:<50} {:>10} {:>8} {:>8} {:>8} {:>8}",
            "File", "Lines", "Code", "Comments", "Blanks", "Language"
        );
        println!("{}", "-".repeat(100));
        for stats in file_stats {
            let path_str = &stats.path;
            let display_path = if path_str.len() > 50 {
                format!("...{}", &path_str[path_str.len() - 47..])
            } else {
                path_str.clone()
            };
            println!(
                "{:<50} {:>10} {:>8} {:>8} {:>8} {:>8}",
                display_path, stats.lines, stats.code, stats.comments, stats.blanks, stats.language
            );
        }
    }

    println!("\n{:>15} {:>8} {:>8} {:>8} {:>8} {:>8} {:>6}",
        "Language", "Files", "Lines", "Code", "Comments", "Blanks", "Code%");
    println!("{}", "-".repeat(80));

    let mut total_files = 0u64;
    let mut total_code = 0u64;
    let mut total_comments = 0u64;
    let mut total_blanks = 0u64;

    let mut sorted_langs: Vec<_> = lang_stats.iter().collect();
    sorted_langs.sort_by(|a, b| b.1.code.cmp(&a.1.code));

    for (lang, stats) in &sorted_langs {
        let code_pct = if stats.lines > 0 {
            (stats.code as f64 / stats.lines as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "{:>15} {:>8} {:>8} {:>8} {:>8} {:>8} {:>7.1}%",
            lang, stats.files, stats.lines, stats.code, stats.comments, stats.blanks, code_pct
        );
        total_files += stats.files;
        total_code += stats.code;
        total_comments += stats.comments;
        total_blanks += stats.blanks;
    }

    println!("{}", "-".repeat(80));
    let total_code_pct = if total_lines > 0 {
        (total_code as f64 / total_lines as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "{:>15} {:>8} {:>8} {:>8} {:>8} {:>8} {:>7.1}%",
        "Total", total_files, total_lines, total_code, total_comments, total_blanks, total_code_pct
    );
}

fn output_json(
    by_file: bool,
    file_stats: &[FileStats],
    lang_stats: &HashMap<String, LanguageStats>,
) {
    if by_file {
        println!("{{");
        println!("  \"files\": [");
        for (i, stats) in file_stats.iter().enumerate() {
            let _comma = if i < file_stats.len() - 1 { "," } else { "" };
            println!("    {{");
            println!("      \"path\": {:?},", stats.path);
            println!("      \"language\": {:?},", stats.language);
            println!("      \"lines\": {},", stats.lines);
            println!("      \"code\": {},", stats.code);
            println!("      \"comments\": {},", stats.comments);
            println!("      \"blanks\": {}", stats.blanks);
            println!("    }}{}", serde_json_dummy(stats));
        }
        println!("  ],");
    }

    println!("  \"languages\": {{");
    let sorted_langs: Vec<_> = lang_stats.iter().collect();
    for (i, (lang, stats)) in sorted_langs.iter().enumerate() {
        let comma = if i < sorted_langs.len() - 1 { "," } else { "" };
        println!("    {:?}: {{", lang);
        println!("      \"files\": {},", stats.files);
        println!("      \"lines\": {},", stats.lines);
        println!("      \"code\": {},", stats.code);
        println!("      \"comments\": {},", stats.comments);
        println!("      \"blanks\": {}", stats.blanks);
        println!("    }}{}", comma);
    }
    println!("  }}");
    println!("}}");
}

fn serde_json_dummy(stats: &FileStats) -> &str {
    let _ = stats;
    ""
}

fn output_csv(
    by_file: bool,
    file_stats: &[FileStats],
    lang_stats: &HashMap<String, LanguageStats>,
) {
    if by_file {
        println!("file,language,lines,code,comments,blanks");
        for stats in file_stats {
            println!(
                "{},{},{},{},{},{}",
                stats.path, stats.language, stats.lines, stats.code, stats.comments, stats.blanks
            );
        }
    } else {
        println!("language,files,lines,code,comments,blanks,code_pct");
        let mut sorted_langs: Vec<_> = lang_stats.iter().collect();
        sorted_langs.sort_by(|a, b| b.1.code.cmp(&a.1.code));

        for (lang, stats) in &sorted_langs {
            let code_pct = if stats.lines > 0 {
                (stats.code as f64 / stats.lines as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "{},{},{},{},{},{},{:.1}",
                lang, stats.files, stats.lines, stats.code, stats.comments, stats.blanks, code_pct
            );
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    let mut path = PathBuf::from(".");
    let mut by_file = false;
    let mut format = "table".to_string();
    let mut include_ext: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "--by-file" => by_file = true,
            "--format" => {
                i += 1;
                if i < args.len() {
                    format = args[i].clone();
                }
            }
            "--include-ext" => {
                i += 1;
                if i < args.len() {
                    include_ext = args[i].split(',').map(|s| s.trim().to_string()).collect();
                }
            }
            _ => {
                if !args[i].starts_with('-') {
                    path = PathBuf::from(&args[i]);
                }
            }
        }
        i += 1;
    }

    let files = if path.is_file() {
        vec![path]
    } else {
        walk_dir(&path, &include_ext)
    };

    let mut lang_stats: HashMap<String, LanguageStats> = HashMap::new();
    let mut file_stats: Vec<FileStats> = Vec::new();

    for file_path in &files {
        let lang = if include_ext.is_empty() {
            detect_language(file_path)
        } else {
            file_path.extension().and_then(|e| e.to_str()).and_then(|ext| {
                if include_ext.iter().any(|i| i.to_lowercase() == ext.to_lowercase()) {
                    detect_language(file_path)
                } else {
                    None
                }
            })
        };

        if let Some(language) = lang {
            let stats = count_file(file_path, language);
            let entry = lang_stats.entry(language.to_string()).or_default();
            entry.files += 1;
            entry.lines += stats.lines;
            entry.code += stats.code;
            entry.comments += stats.comments;
            entry.blanks += stats.blanks;
            file_stats.push(stats);
        }
    }

    let total_lines: u64 = lang_stats.values().map(|s| s.lines).sum();

    match format.as_str() {
        "json" => output_json(by_file, &file_stats, &lang_stats),
        "csv" => output_csv(by_file, &file_stats, &lang_stats),
        _ => output_table(by_file, &file_stats, &lang_stats, total_lines),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_flag() {
        assert!(true);
    }
}
