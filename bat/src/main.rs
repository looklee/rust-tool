use std::env;
use std::fs;
use std::io::{self, BufRead, Write, BufReader, BufWriter};
use std::path::Path;
use std::process;

struct Config {
    language: Option<String>,
    line_numbers: bool,
    plain: bool,
    theme: String,
    help: bool,
    files: Vec<String>,
}

fn print_help() {
    println!("bat - better cat with syntax highlighting (simplified Rust implementation)

USAGE:
    bat [OPTIONS] [FILE...]
    cat file | bat [OPTIONS]

OPTIONS:
    -l, --language LANG    Specify language for syntax highlighting
    -n, --line-numbers     Show line numbers
    -p, --plain            Plain mode (no highlighting, no decorations)
    --theme THEME          Color theme: default, dark, light (default: default)
    -h, --help             Print help information

DESCRIPTION:
    bat is a cat clone with syntax highlighting and Git integration.
    It automatically detects the language and applies appropriate highlighting.

SUPPORTED LANGUAGES:
    Rust (.rs), Python (.py), JavaScript (.js), TypeScript (.ts),
    Go (.go), C (.c), C++ (.cpp), Java (.java), HTML (.html),
    CSS (.css), JSON (.json), YAML (.yml/.yaml), Markdown (.md),
    Shell (.sh), TOML (.toml), XML (.xml), SQL (.sql)

EXAMPLES:
    bat file.rs
    bat -n file.py
    bat --theme dark src/main.rs
    cat file | bat -l rust
    bat -p file.txt");
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut language = None;
    let mut line_numbers = false;
    let mut plain = false;
    let mut theme = "default".to_string();
    let mut help = false;
    let mut files: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-l" | "--language" => {
                i += 1;
                if i < args.len() {
                    language = Some(args[i].clone());
                }
            }
            "-n" | "--line-numbers" => line_numbers = true,
            "-p" | "--plain" => plain = true,
            "--theme" => {
                i += 1;
                if i < args.len() {
                    theme = args[i].clone();
                }
            }
            "-h" | "--help" => help = true,
            _ => {
                if !args[i].starts_with('-') {
                    files.push(args[i].clone());
                }
            }
        }
        i += 1;
    }

    Config {
        language,
        line_numbers,
        plain,
        theme,
        help,
        files,
    }
}

/// Detect language from file extension
fn detect_language(path: &str) -> Option<&'static str> {
    let ext = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())?;

    match ext.as_str() {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" => Some("javascript"),
        "ts" => Some("typescript"),
        "go" => Some("go"),
        "c" => Some("c"),
        "cpp" | "cc" | "cxx" => Some("cpp"),
        "h" | "hpp" => Some("cpp"),
        "java" => Some("java"),
        "html" | "htm" => Some("html"),
        "css" => Some("css"),
        "json" => Some("json"),
        "yml" | "yaml" => Some("yaml"),
        "md" => Some("markdown"),
        "sh" | "bash" => Some("shell"),
        "toml" => Some("toml"),
        "xml" => Some("xml"),
        "sql" => Some("sql"),
        "rb" => Some("ruby"),
        "lua" => Some("lua"),
        _ => None,
    }
}

// ANSI color codes
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const WHITE: &str = "\x1b[37m";
const GRAY: &str = "\x1b[90m";
const BRIGHT_RED: &str = "\x1b[91m";
const BRIGHT_GREEN: &str = "\x1b[92m";
const BRIGHT_YELLOW: &str = "\x1b[93m";
const BRIGHT_BLUE: &str = "\x1b[94m";

/// Syntax highlight a line based on language
fn highlight_line(line: &str, language: &str, theme: &str) -> String {
    if theme == "light" {
        return highlight_line_light(line, language);
    }
    highlight_line_dark(line, language)
}

/// Dark theme syntax highlighting
fn highlight_line_dark(line: &str, language: &str) -> String {
    let mut result = String::new();
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut in_comment = false;
    let mut in_line_comment = false;

    while let Some(c) = chars.next() {
        // Handle line comments
        if !in_string && !in_line_comment {
            if c == '/' && chars.peek() == Some(&'/') {
                in_line_comment = true;
                result.push_str(GRAY);
                result.push(c);
                continue;
            }
        }

        if in_line_comment {
            result.push(c);
            if c == '\n' {
                in_line_comment = false;
                result.push_str(RESET);
            }
            continue;
        }

        // Handle block comments
        if !in_string && !in_comment {
            if c == '/' && chars.peek() == Some(&'*') {
                in_comment = true;
                result.push_str(GRAY);
                result.push(c);
                continue;
            }
        }

        if in_comment {
            result.push(c);
            if c == '*' && chars.peek() == Some(&'/') {
                result.push('/');
                chars.next();
                in_comment = false;
                result.push_str(RESET);
            }
            continue;
        }

        // Handle strings
        if (c == '"' || c == '\'' || c == '`') && !in_string {
            in_string = true;
            result.push_str(BRIGHT_GREEN);
            result.push(c);
            continue;
        }

        if in_string {
            result.push(c);
            if c == '"' || c == '\'' || c == '`' {
                in_string = false;
                result.push_str(RESET);
            }
            continue;
        }

        // Handle numbers
        if c.is_ascii_digit() && (result.is_empty() || !result.ends_with(|ch: char| ch.is_alphabetic())) {
            let mut num = String::from(c);
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() || next == '.' || next == '_' {
                    num.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            result.push_str(BRIGHT_YELLOW);
            result.push_str(&num);
            result.push_str(RESET);
            continue;
        }

        // Handle keywords and identifiers
        if c.is_alphabetic() || c == '_' {
            let mut word = String::from(c);
            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '_' {
                    word.push(next);
                    chars.next();
                } else {
                    break;
                }
            }

            let highlighted = match language {
                "rust" | "go" | "c" | "cpp" | "java" => highlight_c_like_word(&word),
                "python" | "shell" | "ruby" => highlight_script_word(&word, language),
                "javascript" | "typescript" => highlight_js_word(&word),
                "json" | "yaml" | "toml" | "xml" | "html" | "css" => highlight_data_word(&word, language),
                "sql" => highlight_sql_word(&word),
                _ => word,
            };

            result.push_str(&highlighted);
            continue;
        }

        result.push(c);
    }

    result
}

/// Light theme syntax highlighting
fn highlight_line_light(line: &str, language: &str) -> String {
    // Simplified light theme - use darker colors for contrast
    let mut result = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'/') {
            result.push_str("\x1b[38;5;242m"); // dark gray
            result.push(c);
            while let Some(&next) = chars.peek() {
                if next != '\n' {
                    result.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            result.push_str(RESET);
            continue;
        }

        if c == '"' || c == '\'' {
            result.push_str("\x1b[38;5;28m"); // dark green
            result.push(c);
            while let Some(&next) = chars.peek() {
                if next != c {
                    result.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Some(&next) = chars.peek() {
                if next == c {
                    result.push(next);
                    chars.next();
                }
            }
            result.push_str(RESET);
            continue;
        }

        if c.is_alphabetic() || c == '_' {
            let mut word = String::from(c);
            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '_' {
                    word.push(next);
                    chars.next();
                } else {
                    break;
                }
            }

            let is_keyword = match language {
                "rust" | "go" | "c" | "cpp" | "java" => is_c_like_keyword(&word),
                "python" | "shell" | "ruby" => is_script_keyword(&word, language),
                "javascript" | "typescript" => is_js_keyword(&word),
                "sql" => is_sql_keyword(&word),
                _ => false,
            };

            if is_keyword {
                result.push_str("\x1b[38;5;203m"); // orange-red
                result.push_str(&word);
                result.push_str(RESET);
            } else {
                result.push_str(&word);
            }
            continue;
        }

        result.push(c);
    }

    result
}

fn highlight_c_like_word(word: &str) -> String {
    if is_c_like_keyword(word) {
        format!("{}{}{}", BRIGHT_BLUE, word, RESET)
    } else if word.starts_with(|c: char| c.is_uppercase()) && word.len() > 1 {
        format!("{}{}{}", YELLOW, word, RESET) // Types
    } else {
        word.to_string()
    }
}

fn highlight_script_word(word: &str, language: &str) -> String {
    if is_script_keyword(word, language) {
        format!("{}{}{}", BRIGHT_BLUE, word, RESET)
    } else {
        word.to_string()
    }
}

fn highlight_js_word(word: &str) -> String {
    if is_js_keyword(word) {
        format!("{}{}{}", BRIGHT_BLUE, word, RESET)
    } else {
        word.to_string()
    }
}

fn highlight_data_word(word: &str, language: &str) -> String {
    match language {
        "json" => {
            if word.ends_with(':') {
                format!("{}{}{}", CYAN, word, RESET)
            } else {
                word.to_string()
            }
        }
        "html" | "xml" => {
            if word.starts_with('<') || word.starts_with('>') || word.starts_with('/') {
                format!("{}{}{}", RED, word, RESET)
            } else {
                word.to_string()
            }
        }
        _ => word.to_string(),
    }
}

fn highlight_sql_word(word: &str) -> String {
    if is_sql_keyword(word) {
        format!("{}{}{}", BRIGHT_BLUE, word, RESET)
    } else {
        word.to_string()
    }
}

fn is_c_like_keyword(word: &str) -> bool {
    matches!(
        word,
        "fn" | "let" | "mut" | "const" | "static" | "impl" | "struct" | "enum" | "trait" | "mod" | "use" | "pub"
        | "if" | "else" | "match" | "loop" | "while" | "for" | "in" | "return" | "break" | "continue"
        | "async" | "await" | "move" | "ref" | "self" | "Self" | "super" | "where" | "type"
        | "int" | "float" | "double" | "char" | "void" | "bool" | "string" | "true" | "false" | "null"
        | "new" | "class" | "interface" | "extends" | "implements" | "package" | "import" | "public" | "private" | "protected"
        | "try" | "catch" | "throw" | "finally" | "switch" | "case" | "default" | "goto"
        | "include" | "define" | "ifdef" | "ifndef" | "endif"
        | "func" | "var" | "chan" | "defer" | "go" | "select" | "map" | "range"
        | "println" | "print" | "panic" | "vec" | "Some" | "None" | "Ok" | "Err" | "Box" | "Vec" | "String" | "Option" | "Result"
    )
}

fn is_script_keyword(word: &str, language: &str) -> bool {
    match language {
        "python" => matches!(
            word,
            "def" | "class" | "import" | "from" | "as" | "if" | "elif" | "else" | "for" | "while"
            | "return" | "yield" | "try" | "except" | "finally" | "raise" | "with" | "pass"
            | "True" | "False" | "None" | "and" | "or" | "not" | "in" | "is" | "lambda"
            | "print" | "self" | "global" | "nonlocal" | "assert" | "del"
        ),
        "shell" => matches!(
            word,
            "if" | "then" | "else" | "elif" | "fi" | "for" | "while" | "do" | "done"
            | "case" | "esac" | "function" | "return" | "exit" | "echo" | "export" | "source"
            | "local" | "readonly" | "shift" | "set" | "unset"
        ),
        "ruby" => matches!(
            word,
            "def" | "class" | "module" | "require" | "include" | "if" | "elsif" | "else"
            | "unless" | "while" | "until" | "for" | "do" | "end" | "return" | "yield"
            | "begin" | "rescue" | "ensure" | "raise" | "true" | "false" | "nil"
            | "and" | "or" | "not" | "in" | "self"
        ),
        _ => false,
    }
}

fn is_js_keyword(word: &str) -> bool {
    matches!(
        word,
        "const" | "let" | "var" | "function" | "async" | "await" | "return"
        | "if" | "else" | "switch" | "case" | "default" | "for" | "while" | "do"
        | "try" | "catch" | "finally" | "throw" | "new" | "class" | "extends"
        | "import" | "export" | "from" | "of" | "in" | "typeof" | "instanceof"
        | "true" | "false" | "null" | "undefined" | "this" | "super"
        | "console" | "log" | "require" | "module"
    )
}

fn is_sql_keyword(word: &str) -> bool {
    matches!(
        word.to_uppercase().as_str(),
        "SELECT" | "FROM" | "WHERE" | "INSERT" | "INTO" | "VALUES"
        | "UPDATE" | "SET" | "DELETE" | "CREATE" | "TABLE" | "DROP"
        | "ALTER" | "ADD" | "INDEX" | "JOIN" | "LEFT" | "RIGHT" | "INNER" | "OUTER"
        | "ON" | "AND" | "OR" | "NOT" | "NULL" | "IS" | "AS" | "ORDER" | "BY"
        | "GROUP" | "HAVING" | "LIMIT" | "OFFSET" | "UNION" | "ALL" | "DISTINCT"
    )
}

/// Format line number with appropriate width
fn format_line_number(num: usize, max_num: usize) -> String {
    let width = max_num.to_string().len();
    format!("{:>width$}", num, width = width)
}

/// Print file content with optional syntax highlighting
fn print_file(path: &str, config: &Config) {
    let language = config.language.clone()
        .or_else(|| detect_language(path).map(|s| s.to_string()));

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bat: {}: {}", path, e);
            return;
        }
    };

    let lines: Vec<&str> = content.lines().collect();
    let max_num = lines.len();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for (i, line) in lines.iter().enumerate() {
        if config.line_numbers && !config.plain {
            let line_num = format_line_number(i + 1, max_num);
            write!(out, "{} {} │ ", GRAY, line_num).unwrap();
        }

        if config.plain {
            writeln!(out, "{}", line).unwrap();
        } else {
            let lang = language.as_deref().unwrap_or("");
            let highlighted = highlight_line(line, lang, &config.theme);
            writeln!(out, "{}", highlighted).unwrap();
        }
    }
}

fn main() {
    let config = parse_args();

    if config.help {
        print_help();
        process::exit(0);
    }

    if config.files.is_empty() {
        // Read from stdin
        let language = config.language.clone();
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = BufWriter::new(stdout.lock());

        let mut line_num = 0;
        for line in stdin.lock().lines() {
            line_num += 1;
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            if config.line_numbers && !config.plain {
                write!(out, "{} {:>4} │ ", GRAY, line_num).unwrap();
            }

            if config.plain {
                writeln!(out, "{}", line).unwrap();
            } else {
                let lang = language.as_deref().unwrap_or("");
                let highlighted = highlight_line(&line, lang, &config.theme);
                writeln!(out, "{}", highlighted).unwrap();
            }
        }
    } else {
        for file in &config.files {
            if config.files.len() > 1 {
                println!("{}{}{}", BOLD, file, RESET);
            }
            print_file(file, &config);
        }
    }
}
