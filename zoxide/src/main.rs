use std::env;
use std::fs;
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

struct Config {
    command: String,
    args: Vec<String>,
    help: bool,
}

fn print_help() {
    println!("zoxide - smarter cd (simplified Rust implementation)

USAGE:
    zoxide <COMMAND> [OPTIONS] [ARGS]

COMMANDS:
    add [DIR...]          Add directories to the database
    query [KEYWORD...]    Search the database and print the best match
    edit                  Remove dead entries from the database
    init <SHELL>          Print shell init script
    help                  Print this help message

OPTIONS:
    -h, --help            Print help information

DESCRIPTION:
    zoxide is a smarter cd command. It remembers which directories you use
    most frequently, so you can jump to them quickly.

EXAMPLES:
    zoxide add /home/user/projects
    zoxide query proj          # jumps to /home/user/projects
    zoxide query p r o j       # same, with fuzzy matching
    zoxide edit                # clean up dead entries
    zoxide init bash           # print bash init script");
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut help = false;
    let mut command = String::new();
    let mut cmd_args: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => help = true,
            "add" | "query" | "edit" | "init" | "help" => {
                command = args[i].clone();
            }
            _ => {
                if command.is_empty() {
                    command = "query".to_string();
                }
                cmd_args.push(args[i].clone());
            }
        }
        i += 1;
    }

    if command.is_empty() {
        command = "help".to_string();
    }

    Config {
        command,
        args: cmd_args,
        help,
    }
}

fn database_path() -> String {
    env::var("HOME")
        .map(|h| format!("{}/.zoxide", h))
        .unwrap_or_else(|_| ".zoxide".to_string())
}

/// Database entry: directory path with score and last access time
#[derive(Clone, Debug)]
struct DirEntry {
    path: String,
    score: f64,
    last_accessed: u64,
}

/// Load database from file
fn load_database() -> Vec<DirEntry> {
    let path = database_path();
    let mut entries = Vec::new();

    if let Ok(content) = fs::read_to_string(&path) {
        for line in content.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                if let (Ok(score), Ok(time)) = (parts[0].parse::<f64>(), parts[1].parse::<u64>()) {
                    entries.push(DirEntry {
                        path: parts[2].to_string(),
                        score,
                        last_accessed: time,
                    });
                }
            }
        }
    }

    entries
}

/// Save database to file
fn save_database(entries: &[DirEntry]) {
    let path = database_path();
    if let Some(parent) = Path::new(&path).parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut content = String::new();
    for entry in entries {
        content.push_str(&format!("{}|{}|{}\n", entry.score, entry.last_accessed, entry.path));
    }

    let _ = fs::write(&path, content);
}

/// Calculate score for a directory based on access frequency and recency
fn calculate_score(entry: &DirEntry) -> f64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let age_hours = ((now - entry.last_accessed) as f64) / 3600.0;
    let recency_bonus = (-age_hours / 168.0).exp(); // half-life of 1 week
    entry.score * (1.0 + recency_bonus)
}

/// Fuzzy match score
fn fuzzy_score(pattern: &str, text: &str) -> i32 {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    if pattern_chars.is_empty() {
        return 0;
    }

    let mut score: i32 = 0;
    let mut last_match = -1i32;
    let mut pi = 0;

    for (ti, tc) in text_chars.iter().enumerate() {
        if pi >= pattern_chars.len() {
            break;
        }

        let pc = pattern_chars[pi].to_lowercase().next().unwrap_or(pattern_chars[pi]);
        let tlc = tc.to_lowercase().next().unwrap_or(*tc);

        if pc == tlc {
            if last_match >= 0 && ti as i32 == last_match + 1 {
                score += 3;
            } else {
                score += 1;
            }

            if ti == 0 || text_chars[ti - 1] == '/' || text_chars[ti - 1] == '_' || text_chars[ti - 1] == '-' {
                score += 2;
            }

            last_match = ti as i32;
            pi += 1;
        }
    }

    if pi == pattern_chars.len() {
        score
    } else {
        -1
    }
}

fn cmd_add(args: &[String]) {
    let mut entries = load_database();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for dir in args {
        let path = if dir.starts_with('~') {
            env::var("HOME")
                .map(|h| dir.replacen('~', &h, 1))
                .unwrap_or_else(|_| dir.clone())
        } else {
            dir.clone()
        };

        // Check if path exists
        if !Path::new(&path).is_dir() {
            eprintln!("zoxide: '{}' is not a directory", path);
            continue;
        }

        // Update existing entry or add new one
        if let Some(entry) = entries.iter_mut().find(|e| e.path == path) {
            entry.score += 1.0;
            entry.last_accessed = now;
        } else {
            entries.push(DirEntry {
                path,
                score: 1.0,
                last_accessed: now,
            });
        }
    }

    save_database(&entries);
}

fn cmd_query(args: &[String]) {
    let entries = load_database();

    if entries.is_empty() {
        eprintln!("zoxide: database is empty. Add directories with 'zoxide add <dir>'");
        process::exit(1);
    }

    // If no query, show top entries
    if args.is_empty() {
        let mut sorted: Vec<_> = entries.iter().collect();
        sorted.sort_by(|a, b| calculate_score(b).partial_cmp(&calculate_score(a)).unwrap());

        for entry in sorted.iter().take(20) {
            println!("{:>8.1} {}", entry.score, entry.path);
        }
        return;
    }

    // Build search pattern from args (space-separated keywords)
    let pattern = args.join(" ");

    // Score and sort entries
    let mut scored: Vec<(&DirEntry, f64, i32)> = Vec::new();

    for entry in &entries {
        // Check if path exists
        if !Path::new(&entry.path).is_dir() {
            continue;
        }

        let dir_score = calculate_score(entry);
        let fuzzy = fuzzy_score(&pattern, &entry.path);

        if fuzzy >= 0 {
            scored.push((entry, dir_score, fuzzy));
        }
    }

    // Sort by combined score (fuzzy match + frequency score)
    scored.sort_by(|a, b| {
        let score_a = (a.2 as f64) * 10.0 + a.1;
        let score_b = (b.2 as f64) * 10.0 + b.1;
        score_b.partial_cmp(&score_a).unwrap()
    });

    if scored.is_empty() {
        eprintln!("zoxide: no match found for '{}'", pattern);
        process::exit(1);
    }

    // Print the best match
    println!("{}", scored[0].0.path);
}

fn cmd_edit() {
    let mut entries = load_database();
    let original_count = entries.len();

    // Remove entries for directories that no longer exist
    entries.retain(|e| Path::new(&e.path).is_dir());

    let removed = original_count - entries.len();
    save_database(&entries);

    println!("Removed {} dead entries", removed);
}

fn cmd_init(shell: &str) {
    match shell {
        "bash" => {
            println!(r#"__zoxide_cd() {{
    cd "$@" || return
}}

__zoxide_query() {{
    local result
    result="$(zoxide query "$@" 2>/dev/null)" && __zoxide_cd "$result"
}}

zoxide() {{
    if [ $# -eq 0 ]; then
        __zoxide_query
    elif [ "$1" = "-" ]; then
        __zoxide_query -
    else
        __zoxide_query "$@"
    fi
}}

__zoxide_add() {{
    zoxide add "$(pwd)"
}}

# Hook into cd
alias cd='__zoxide_add; __zoxide_cd'"#);
        }
        "zsh" => {
            println!(r#"__zoxide_cd() {{
    cd "$@" || return
}}

__zoxide_query() {{
    local result
    result="$(zoxide query "$@" 2>/dev/null)" && __zoxide_cd "$result"
}}

zoxide() {{
    if [ $# -eq 0 ]; then
        __zoxide_query
    elif [ "$1" = "-" ]; then
        __zoxide_query -
    else
        __zoxide_query "$@"
    fi
}}

__zoxide_add() {{
    zoxide add "$(pwd)"
}}

# Hook into cd
alias cd='__zoxide_add; __zoxide_cd'"#);
        }
        _ => {
            eprintln!("zoxide: unsupported shell '{}'. Use 'bash' or 'zsh'", shell);
            process::exit(1);
        }
    }
}

fn main() {
    let config = parse_args();

    if config.help {
        print_help();
        process::exit(0);
    }

    match config.command.as_str() {
        "add" => cmd_add(&config.args),
        "query" => cmd_query(&config.args),
        "edit" => cmd_edit(),
        "init" => {
            let shell = config.args.first().map(|s| s.as_str()).unwrap_or("bash");
            cmd_init(shell);
        }
        "help" => print_help(),
        _ => {
            eprintln!("zoxide: unknown command '{}'", config.command);
            eprintln!("Try 'zoxide help' for usage information");
            process::exit(1);
        }
    }
}
