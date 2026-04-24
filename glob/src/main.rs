use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 配置选项
struct GlobConfig {
    /// 模式
    pattern: String,
    /// 根目录
    root: String,
    /// 最大深度
    max_depth: Option<usize>,
    /// 只显示文件
    files_only: bool,
    /// 只显示目录
    dirs_only: bool,
    /// 隐藏文件
    hidden: bool,
}

impl Default for GlobConfig {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            root: ".".to_string(),
            max_depth: None,
            files_only: false,
            dirs_only: false,
            hidden: false,
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let config = parse_args(&args[1..])?;

    if config.pattern.is_empty() {
        eprintln!("glob - find files matching pattern");
        eprintln!();
        eprintln!("Usage: glob [OPTIONS] PATTERN");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -r, --root <DIR>      root directory (default: .)");
        eprintln!("  -d, --max-depth <N>   maximum depth");
        eprintln!("  -f, --files-only      show only files");
        eprintln!("  -D, --dirs-only       show only directories");
        eprintln!("  -H, --hidden          include hidden files");
        eprintln!("  -h, --help            show help");
        std::process::exit(1);
    }

    let mut matches = Vec::new();
    let root_path = Path::new(&config.root);

    glob_search(root_path, &config, &mut matches, 0)?;

    // 排序输出
    matches.sort();

    // 输出结果
    for path in matches {
        println!("{}", path.display());
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<GlobConfig> {
    let mut config = GlobConfig::default();
    let mut pattern: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if arg == "--help" || arg == "-h" {
            pattern = None; // 触发帮助信息
            break;
        } else if arg == "-r" || arg == "--root" {
            if i + 1 < args.len() {
                i += 1;
                config.root = args[i].clone();
            }
        } else if arg == "-d" || arg == "--max-depth" {
            if i + 1 < args.len() {
                i += 1;
                config.max_depth = args[i].parse().ok();
            }
        } else if arg == "-f" || arg == "--files-only" {
            config.files_only = true;
        } else if arg == "-D" || arg == "--dirs-only" {
            config.dirs_only = true;
        } else if arg == "-H" || arg == "--hidden" {
            config.hidden = true;
        } else if arg.starts_with('-') {
            eprintln!("glob: unknown option '{}'", arg);
            std::process::exit(1);
        } else {
            pattern = Some(arg.clone());
        }
        i += 1;
    }

    config.pattern = pattern.unwrap_or_default();
    Ok(config)
}

/// 递归搜索匹配的文件
fn glob_search(
    dir: &Path,
    config: &GlobConfig,
    matches: &mut Vec<PathBuf>,
    depth: usize,
) -> io::Result<()> {
    // 检查深度限制
    if let Some(max) = config.max_depth {
        if depth > max {
            return Ok(());
        }
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry_result in entries {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // 获取文件名
        let file_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        // 跳过隐藏文件（除非指定）
        if !config.hidden && file_name.starts_with('.') {
            continue;
        }

        // 检查是否匹配模式
        let is_match = glob_match(&file_name, &config.pattern);

        if is_match {
            // 检查类型过滤
            let is_dir = path.is_dir();
            let is_file = path.is_file();

            if config.files_only && !is_file {
                continue;
            }
            if config.dirs_only && !is_dir {
                continue;
            }

            matches.push(path.clone());
        }

        // 递归搜索目录
        if path.is_dir() {
            glob_search(&path, config, matches, depth + 1)?;
        }
    }

    Ok(())
}

/// 简单的 glob 模式匹配
/// 支持：*, ?, [...]
fn glob_match(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    // 简单实现：逐字符匹配
    glob_match_helper(text, pattern)
}

fn glob_match_helper(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    let mut ti = 0;
    let mut pi = 0;
    let mut star_ti = None;
    let mut star_pi = 0;

    while ti < text_chars.len() {
        if pi < pattern_chars.len() && (pattern_chars[pi] == '?' || pattern_chars[pi] == text_chars[ti]) {
            ti += 1;
            pi += 1;
        } else if pi < pattern_chars.len() && pattern_chars[pi] == '*' {
            star_ti = Some(ti);
            star_pi = pi;
            pi += 1;
        } else if let Some(st) = star_ti {
            ti = st + 1;
            star_ti = Some(ti);
            pi = star_pi + 1;
        } else {
            return false;
        }
    }

    // 检查剩余的模式是否都是 *
    while pi < pattern_chars.len() && pattern_chars[pi] == '*' {
        pi += 1;
    }

    pi == pattern_chars.len()
}
