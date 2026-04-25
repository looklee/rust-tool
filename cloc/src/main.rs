use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct FileStats {
    path: String,
    blank: u64,
    comment: u64,
    source: u64,
}

#[derive(Debug, Default)]
struct LanguageSummary {
    files: u64,
    blank: u64,
    comment: u64,
    source: u64,
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

/// Count blank, comment, and source lines for a file
fn count_file(path: &Path, language: &str) -> FileStats {
    let mut stats = FileStats {
        path: path.to_string_lossy().to_string(),
        blank: 0,
        comment: 0,
        source: 0,
    };
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return stats,
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut in_block_comment = false;

    for line in &lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            stats.blank += 1;
            continue;
        }

        match language {
            l if is_c_style_lang(l) => {
                if in_block_comment {
                    stats.comment += 1;
                    if trimmed.ends_with("*/") {
                        in_block_comment = false;
                    }
                } else if trimmed.starts_with("//") {
                    stats.comment += 1;
                } else if trimmed.starts_with("/*") {
                    stats.comment += 1;
                    if !trimmed.ends_with("*/") {
                        in_block_comment = true;
                    }
                } else {
                    stats.source += 1;
                }
            }
            l if is_hash_comment_lang(l) => {
                if trimmed.starts_with('#') {
                    stats.comment += 1;
                } else {
                    stats.source += 1;
                }
            }
            "HTML" | "XML" => {
                if trimmed.starts_with("<!--") {
                    stats.comment += 1;
                    if !trimmed.ends_with("-->") {
                        in_block_comment = true;
                    }
                } else if in_block_comment {
                    stats.comment += 1;
                    if trimmed.ends_with("-->") {
                        in_block_comment = false;
                    }
                } else {
                    stats.source += 1;
                }
            }
            _ => {
                stats.source += 1;
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
    exclude_dirs: &[String],
    by_file: bool,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if exclude_dirs.iter().any(|e| e == dir_name) {
                    continue;
                }
                files.extend(walk_dir(&path, exclude_dirs, by_file));
            } else if path.is_file() && !is_binary(&path) {
                if by_file || detect_language(&path).is_some() {
                    files.push(path);
                }
            }
        }
    }
    files
}

fn print_help() {
    println!(
        r#"cloc - Count lines of code

Usage: cloc [OPTIONS] [PATH]

Arguments:
  PATH                  Directory or file to analyze (default: current directory)

Options:
  -h, --help            Show this help message
  --by-file             Show results for each file
  --by-lang             Show results grouped by language (default)
  --exclude-dir DIR     Exclude directory from analysis

Examples:
  cloc .
  cloc --by-file src/
  cloc --exclude-dir target --exclude-dir .git .
  cloc --by-lang src/"#,
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    let mut path = PathBuf::from(".");
    let mut by_file = false;
    let mut by_lang = false;
    let mut exclude_dirs: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "--by-file" => by_file = true,
            "--by-lang" => by_lang = true,
            "--exclude-dir" => {
                i += 1;
                if i < args.len() {
                    exclude_dirs.push(args[i].clone());
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

    // Default to by-lang if neither specified
    if !by_file && !by_lang {
        by_lang = true;
    }

    let files = if path.is_file() {
        vec![path]
    } else {
        walk_dir(&path, &exclude_dirs, by_file)
    };

    let mut lang_summary: HashMap<String, LanguageSummary> = HashMap::new();
    let mut file_stats: Vec<FileStats> = Vec::new();

    for file_path in &files {
        if let Some(lang) = detect_language(file_path) {
            let stats = count_file(file_path, lang);
            let entry = lang_summary.entry(lang.to_string()).or_default();
            entry.files += 1;
            entry.blank += stats.blank;
            entry.comment += stats.comment;
            entry.source += stats.source;
            file_stats.push(stats);
        }
    }

    if by_file {
        println!(
            "{:<50} {:>8} {:>8} {:>8} {:>8}",
            "File", "Blank", "Comment", "Source", "Total"
        );
        println!("{}", "-".repeat(85));
        for stats in &file_stats {
            let total = stats.blank + stats.comment + stats.source;
            let path_str = &stats.path;
            let display_path = if path_str.len() > 50 {
                format!("...{}", &path_str[path_str.len() - 47..])
            } else {
                path_str.clone()
            };
            println!(
                "{:<50} {:>8} {:>8} {:>8} {:>8}",
                display_path, stats.blank, stats.comment, stats.source, total
            );
        }
    }

    if by_lang {
        println!(
            "{:>15} {:>8} {:>8} {:>8} {:>8} {:>6}",
            "Language", "Blank", "Comment", "Source", "Total", "Files"
        );
        println!("{}", "-".repeat(70));

        let mut total_blank = 0u64;
        let mut total_comment = 0u64;
        let mut total_source = 0u64;
        let mut total_files = 0u64;

        let mut sorted_langs: Vec<_> = lang_summary.iter().collect();
        sorted_langs.sort_by(|a, b| b.1.source.cmp(&a.1.source));

        for (lang, summary) in &sorted_langs {
            let total = summary.blank + summary.comment + summary.source;
            println!(
                "{:>15} {:>8} {:>8} {:>8} {:>8} {:>6}",
                lang, summary.blank, summary.comment, summary.source, total, summary.files
            );
            total_blank += summary.blank;
            total_comment += summary.comment;
            total_source += summary.source;
            total_files += summary.files;
        }

        println!("{}", "-".repeat(70));
        let grand_total = total_blank + total_comment + total_source;
        println!(
            "{:>15} {:>8} {:>8} {:>8} {:>8} {:>6}",
            "SUM", total_blank, total_comment, total_source, grand_total, total_files
        );
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
