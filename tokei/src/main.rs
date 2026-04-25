use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct FileStats {
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
    let mut stats = FileStats::default();
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
fn walk_dir(path: &Path, exclude: &[String], include_types: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if exclude.iter().any(|e| e == dir_name) {
                    continue;
                }
                files.extend(walk_dir(&path, exclude, include_types));
            } else if path.is_file() {
                if let Some(lang) = detect_language(&path) {
                    if include_types.is_empty() || include_types.iter().any(|t| t == lang) {
                        if !is_binary(&path) {
                            files.push(path);
                        }
                    }
                }
            }
        }
    }
    files
}

fn print_help() {
    println!(
        r#"tokei - Simplified code counter

Usage: tokei [OPTIONS] [PATH]

Arguments:
  PATH                  Directory or file to analyze (default: current directory)

Options:
  -h, --help            Show this help message
  -f, --files           Show per-file statistics
  -e, --exclude DIR     Exclude directory from analysis
  -t, --type LANG       Only count files of specified language type

Examples:
  tokei .
  tokei -f src/
  tokei -e target -e .git .
  tokei -t Rust -t Python ."#,
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    let mut path = PathBuf::from(".");
    let mut files_only = false;
    let mut exclude_dirs: Vec<String> = Vec::new();
    let mut include_types: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-f" | "--files" => files_only = true,
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-e" | "--exclude" => {
                i += 1;
                if i < args.len() {
                    exclude_dirs.push(args[i].clone());
                }
            }
            "-t" | "--type" => {
                i += 1;
                if i < args.len() {
                    include_types.push(args[i].clone());
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
        walk_dir(&path, &exclude_dirs, &include_types)
    };

    let mut lang_stats: HashMap<String, LanguageStats> = HashMap::new();
    let mut file_results: Vec<(PathBuf, String, FileStats)> = Vec::new();

    for file_path in &files {
        if let Some(lang) = detect_language(file_path) {
            let stats = count_file(file_path, lang);
            let entry = lang_stats.entry(lang.to_string()).or_default();
            entry.files += 1;
            entry.lines += stats.lines;
            entry.code += stats.code;
            entry.comments += stats.comments;
            entry.blanks += stats.blanks;
            file_results.push((file_path.clone(), lang.to_string(), stats));
        }
    }

    if files_only {
        println!("{:<60} {:>8} {:>8} {:>8} {:>8}  {}", "File", "Lines", "Code", "Comments", "Blanks", "Language");
        println!("{}", "-".repeat(105));
        for (path, lang, stats) in &file_results {
            let path_str = path.to_string_lossy();
            println!(
                "{:<60} {:>8} {:>8} {:>8} {:>8}  {}",
                if path_str.len() > 60 {
                    format!("...{}", &path_str[path_str.len() - 57..])
                } else {
                    path_str.to_string()
                },
                stats.lines,
                stats.code,
                stats.comments,
                stats.blanks,
                lang
            );
        }
    }

    // Summary by language
    println!("\n{:>6} {:>8} {:>8} {:>8} {:>8}  {}", "Files", "Lines", "Code", "Comments", "Blanks", "Language");
    println!("{}", "-".repeat(65));

    let mut total_files = 0u64;
    let mut total_lines = 0u64;
    let mut total_code = 0u64;
    let mut total_comments = 0u64;
    let mut total_blanks = 0u64;

    let mut sorted_langs: Vec<_> = lang_stats.iter().collect();
    sorted_langs.sort_by(|a, b| b.1.code.cmp(&a.1.code));

    for (lang, stats) in &sorted_langs {
        println!(
            "{:>6} {:>8} {:>8} {:>8} {:>8}  {}",
            stats.files, stats.lines, stats.code, stats.comments, stats.blanks, lang
        );
        total_files += stats.files;
        total_lines += stats.lines;
        total_code += stats.code;
        total_comments += stats.comments;
        total_blanks += stats.blanks;
    }

    println!("{}", "-".repeat(65));
    println!(
        "{:>6} {:>8} {:>8} {:>8} {:>8}  {}",
        total_files, total_lines, total_code, total_comments, total_blanks, "Total"
    );
}
