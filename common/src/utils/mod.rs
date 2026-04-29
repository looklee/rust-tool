use std::path::{Path, PathBuf};

pub fn is_terminal() -> bool {
    atty::is(atty::Stream::Stdout)
}

pub fn print_version_and_exit(program: &str, version: &str) {
    println!("{} {}", program, version);
    std::process::exit(0);
}

pub fn print_help_and_exit(help_text: &str) {
    println!("{}", help_text);
    std::process::exit(0);
}

pub fn is_termux() -> bool {
    super::termux::TermuxInfo::detect_termux()
}

pub fn get_termux_info() -> super::termux::TermuxInfo {
    super::termux::TermuxInfo::new()
}

pub fn get_home_dir() -> PathBuf {
    if is_termux() {
        super::termux::TermuxInfo::get_home_dir()
    } else {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    }
}

pub fn get_config_dir() -> PathBuf {
    if is_termux() {
        super::termux::TermuxInfo::get_config_dir()
    } else {
        dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
    }
}

pub fn get_cache_dir() -> PathBuf {
    if is_termux() {
        super::termux::TermuxInfo::get_cache_dir()
    } else {
        dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."))
    }
}

pub fn get_storage_dir() -> PathBuf {
    if is_termux() {
        super::termux::TermuxInfo::get_storage_dir()
    } else {
        get_home_dir()
    }
}

pub fn get_downloads_dir() -> PathBuf {
    if is_termux() {
        super::termux::TermuxInfo::get_downloads_dir()
    } else {
        dirs::download_dir().unwrap_or_else(|| get_home_dir())
    }
}

pub fn get_documents_dir() -> PathBuf {
    if is_termux() {
        super::termux::TermuxInfo::get_documents_dir()
    } else {
        dirs::document_dir().unwrap_or_else(|| get_home_dir())
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if is_termux() {
        super::termux::TermuxInfo::expand_path(path)
    } else if path.starts_with('~') {
        let home = get_home_dir();
        PathBuf::from(path.replace('~', home.to_str().unwrap_or(""))
    } else {
        PathBuf::from(path)
    }
}

pub fn ensure_dir_exists(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn read_file_to_string(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

pub fn write_string_to_file(path: &Path, content: &str) -> std::io::Result<()> {
    std::fs::write(path, content)
}

pub fn copy_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::copy(src, dst)?;
    Ok(())
}

pub fn remove_file(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)
}

pub fn remove_dir(path: &Path) -> std::io::Result<()> {
    std::fs::remove_dir_all(path)
}

pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

pub fn is_file(path: &Path) -> bool {
    path.is_file()
}

pub fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

pub fn get_file_size(path: &Path) -> std::io::Result<u64> {
    Ok(std::fs::metadata(path)?.len())
}

pub fn get_file_modified_time(path: &Path) -> std::io::Result<std::time::SystemTime> {
    Ok(std::fs::metadata(path)?.modified()?)
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let mins = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", mins, secs)
    } else {
        let hours = seconds / 3600;
        let mins = (seconds % 3600) / 60;
        let secs = seconds % 60;
        format!("{}h {}m {}s", hours, mins, secs)
    }
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        s.chars().take(max_len - 3).collect::<String>() + "..."
    }
}

pub fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

pub fn unescape_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some(&'\\') => {
                    result.push('\\');
                    chars.next();
                }
                Some(&'\"') => {
                    result.push('\"');
                    chars.next();
                }
                Some(&'n') => {
                    result.push('\n');
                    chars.next();
                }
                Some(&'r') => {
                    result.push('\r');
                    chars.next();
                }
                Some(&'t') => {
                    result.push('\t');
                    chars.next();
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }
    
    result
}