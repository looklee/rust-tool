use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

struct Config {
    depth: usize,
    show_hidden: bool,
    git_ignore: bool,
    sort: SortMode,
    icons: bool,
    help: bool,
    directory: Option<String>,
}

#[derive(Clone, Copy)]
enum SortMode {
    Name,
    Size,
    Type,
    Modified,
}

fn print_help() {
    println!("broot - file tree viewer (simplified Rust implementation)

USAGE:
    broot [OPTIONS] [DIRECTORY]

OPTIONS:
    -d, --depth NUM       Maximum depth to display (default: 5)
    -g, --git-ignore      Respect .gitignore patterns
    --sort MODE           Sort entries: name, size, type, modified (default: name)
    --icons               Show file type icons
    -a, --all             Show hidden files
    -h, --help            Print help information

DESCRIPTION:
    broot displays a tree view of the directory structure.
    It supports depth limiting, sorting, and gitignore filtering.

EXAMPLES:
    broot
    broot /home/user
    broot -d 3 --icons
    broot --sort size -d 2
    broot -g /path/to/repo");
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut depth = 5;
    let mut show_hidden = false;
    let mut git_ignore = false;
    let mut sort = SortMode::Name;
    let mut icons = false;
    let mut help = false;
    let mut directory = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--depth" => {
                i += 1;
                if i < args.len() {
                    depth = args[i].parse().unwrap_or(5);
                }
            }
            "-g" | "--git-ignore" => git_ignore = true,
            "--sort" => {
                i += 1;
                if i < args.len() {
                    sort = match args[i].as_str() {
                        "size" => SortMode::Size,
                        "type" => SortMode::Type,
                        "modified" => SortMode::Modified,
                        _ => SortMode::Name,
                    };
                }
            }
            "--icons" => icons = true,
            "-a" | "--all" => show_hidden = true,
            "-h" | "--help" => help = true,
            _ => {
                if !args[i].starts_with('-') {
                    directory = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    Config {
        depth,
        show_hidden,
        git_ignore,
        sort,
        icons,
        help,
        directory,
    }
}

/// Check if a file should be ignored based on .gitignore
fn is_git_ignored(path: &Path, gitignore_patterns: &[String]) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    for pattern in gitignore_patterns {
        if pattern.is_empty() || pattern.starts_with('#') {
            continue;
        }

        // Simple pattern matching
        let pat = pattern.trim_start_matches('!');
        if name == pat || name.ends_with(&format!("/{}", pat)) {
            return true;
        }

        // Wildcard support
        if pat.contains('*') {
            let pat_parts: Vec<&str> = pat.split('*').collect();
            if pat_parts.len() == 2 {
                if name.starts_with(pat_parts[0]) && name.ends_with(pat_parts[1]) {
                    return true;
                }
            }
        }
    }

    false
}

/// Load .gitignore patterns from a directory
fn load_gitignore(dir: &Path) -> Vec<String> {
    let gitignore_path = dir.join(".gitignore");
    let mut patterns = Vec::new();

    if let Ok(content) = fs::read_to_string(&gitignore_path) {
        for line in content.lines() {
            patterns.push(line.to_string());
        }
    }

    patterns
}

/// Get file size
fn get_file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Get file modification time
fn get_modified_time(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(0)
}

/// Get file extension for type-based sorting
fn get_file_type(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Sort entries based on sort mode
fn sort_entries(entries: &mut Vec<PathBuf>, sort: SortMode) {
    entries.sort_by(|a, b| {
        // Directories first
        let a_is_dir = a.is_dir();
        let b_is_dir = b.is_dir();

        match (a_is_dir, b_is_dir) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        match sort {
            SortMode::Name => {
                let name_a = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                let name_b = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                name_a.cmp(&name_b)
            }
            SortMode::Size => {
                let size_a = get_file_size(a);
                let size_b = get_file_size(b);
                size_b.cmp(&size_a) // Larger first
            }
            SortMode::Type => {
                let type_a = get_file_type(a);
                let type_b = get_file_type(b);
                type_a.cmp(&type_b)
            }
            SortMode::Modified => {
                let mod_a = get_modified_time(a);
                let mod_b = get_modified_time(b);
                mod_b.cmp(&mod_a) // Newest first
            }
        }
    });
}

/// Get icon for file type
fn get_icon(path: &Path) -> &'static str {
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();

    if path.is_dir() {
        return "\u{1f4c1}"; // folder
    }

    match name.as_str() {
        "Cargo.toml" | "Cargo.lock" => return "\u{1f4e6}",
        "README.md" | "README" => return "\u{1f4d1}",
        ".gitignore" | ".gitconfig" => return "\u{1f517}",
        _ => {}
    }

    match path.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("rs") => "\u{1f986}",
        Some("toml") => "\u{1f4cb}",
        Some("md") | Some("txt") => "\u{1f4d1}",
        Some("json") | Some("yaml") | Some("yml") => "\u{1f4c4}",
        Some("sh") | Some("bash") => "\u{1f577}",
        Some("py") => "\u{1f40d}",
        Some("js") | Some("ts") => "\u{1f4ab}",
        Some("html") | Some("css") => "\u{1f310}",
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") => "\u{1f4f7}",
        Some("zip") | Some("tar") | Some("gz") => "\u{1f4e5}",
        _ => "\u{1f4c4}",
    }
}

/// Format file size for display
fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{}B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1}K", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1}M", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Print tree recursively
fn print_tree(
    path: &Path,
    prefix: &str,
    is_last: bool,
    depth: usize,
    max_depth: usize,
    config: &Config,
    gitignore_patterns: &[String],
) {
    if depth > max_depth {
        return;
    }

    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    // Check if should be hidden
    if !config.show_hidden && name.starts_with('.') && depth > 0 {
        return;
    }

    // Check gitignore
    if config.git_ignore && depth > 0 && is_git_ignored(path, gitignore_patterns) {
        return;
    }

    // Print current entry
    let connector = if depth == 0 { "" } else if is_last { "\u{2514}\u{2500}\u{2500} " } else { "\u{251c}\u{2500}\u{2500} " };
    let icon = if config.icons { get_icon(path) } else { "" };

    if depth == 0 {
        println!("{}{}", icon, name);
    } else {
        let display_name = if config.icons {
            format!("{} {}", icon, name)
        } else {
            name.clone()
        };

        let size_str = if path.is_file() {
            format!(" ({})", format_size(get_file_size(path)))
        } else {
            String::new()
        };

        println!("{}{}{}{}", prefix, connector, display_name, size_str);
    }

    // Recurse into directories
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = Vec::new();

        if let Ok(read_dir) = fs::read_dir(path) {
            for entry in read_dir.flatten() {
                entries.push(entry.path());
            }
        }

        // Filter and sort
        sort_entries(&mut entries, config.sort);

        let child_prefix = if depth == 0 {
            String::new()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}\u{2502}   ", prefix)
        };

        for (i, entry) in entries.iter().enumerate() {
            let is_last_entry = i == entries.len() - 1;
            print_tree(
                entry,
                &child_prefix,
                is_last_entry,
                depth + 1,
                max_depth,
                config,
                gitignore_patterns,
            );
        }
    }
}

fn main() {
    let config = parse_args();

    if config.help {
        print_help();
        process::exit(0);
    }

    let dir = config.directory.clone().unwrap_or_else(|| ".".to_string());
    let path = Path::new(&dir);

    if !path.exists() {
        eprintln!("broot: '{}': No such file or directory", dir);
        process::exit(1);
    }

    if !path.is_dir() {
        eprintln!("broot: '{}' is not a directory", dir);
        process::exit(1);
    }

    // Load gitignore patterns
    let gitignore_patterns = if config.git_ignore {
        load_gitignore(path)
    } else {
        Vec::new()
    };

    print_tree(
        path,
        "",
        true,
        0,
        config.depth,
        &config,
        &gitignore_patterns,
    );
}
