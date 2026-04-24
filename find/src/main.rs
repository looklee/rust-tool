use chrono::{DateTime, Local};
use regex::Regex;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

/// 配置选项
struct FindConfig {
    /// 搜索路径
    paths: Vec<String>,
    /// 名称模式
    name: Option<String>,
    /// 正则名称模式
    regex_name: Option<String>,
    /// 文件类型：file/dir
    file_type: Option<FileType>,
    /// 最大深度
    max_depth: Option<usize>,
    /// 最小深度
    min_depth: Option<usize>,
    /// 修改时间（天数）
    mtime: Option<i64>,
    /// 文件大小（字节）
    size_gt: Option<u64>,
    size_lt: Option<u64>,
    /// 执行命令（简化：只打印）
    print: bool,
}

#[derive(Clone, Copy)]
enum FileType {
    File,
    Dir,
    Link,
}

impl Default for FindConfig {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            name: None,
            regex_name: None,
            file_type: None,
            max_depth: None,
            min_depth: None,
            mtime: None,
            size_gt: None,
            size_lt: None,
            print: true, // 默认打印
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let config = parse_args(&args[1..])?;

    if config.paths.is_empty() {
        eprintln!("find - search for files");
        eprintln!();
        eprintln!("Usage: find [PATH]... [OPTIONS]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -name <PATTERN>      match file name against pattern");
        eprintln!("  -regex <PATTERN>     match file name against regex pattern");
        eprintln!("  -type <f|d|l>        match by type (file, dir, link)");
        eprintln!("  -maxdepth <N>        maximum depth");
        eprintln!("  -mindepth <N>        minimum depth");
        eprintln!("  -mtime <N>           modified N*24 hours ago");
        eprintln!("  -size +<N>           file size greater than N bytes");
        eprintln!("  -size -<N>           file size less than N bytes");
        eprintln!("  -print               print results (default)");
        eprintln!("  -h, --help           show help");
        std::process::exit(1);
    }

    for path_str in &config.paths {
        let path = Path::new(path_str);
        if !path.exists() {
            eprintln!("find: '{}': No such file or directory", path_str);
            continue;
        }
        find_path(path, &config, 0)?;
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<FindConfig> {
    let mut config = FindConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "--help" || arg == "-h" {
            config.paths.clear(); // 触发帮助信息
            break;
        } else if arg == "-name" {
            if i + 1 < args.len() {
                i += 1;
                config.name = Some(args[i].clone());
            }
        } else if arg == "-regex" {
            if i + 1 < args.len() {
                i += 1;
                config.regex_name = Some(args[i].clone());
            }
        } else if arg == "-type" {
            if i + 1 < args.len() {
                i += 1;
                config.file_type = match args[i].as_str() {
                    "f" | "file" => Some(FileType::File),
                    "d" | "dir" | "directory" => Some(FileType::Dir),
                    "l" | "link" | "symlink" => Some(FileType::Link),
                    _ => None,
                };
            }
        } else if arg == "-maxdepth" {
            if i + 1 < args.len() {
                i += 1;
                config.max_depth = args[i].parse().ok();
            }
        } else if arg == "-mindepth" {
            if i + 1 < args.len() {
                i += 1;
                config.min_depth = args[i].parse().ok();
            }
        } else if arg == "-mtime" {
            if i + 1 < args.len() {
                i += 1;
                config.mtime = args[i].parse().ok();
            }
        } else if arg == "-size" {
            if i + 1 < args.len() {
                i += 1;
                let size_str = &args[i];
                if let Some(s) = size_str.strip_prefix('+') {
                    config.size_gt = s.parse().ok();
                } else if let Some(s) = size_str.strip_prefix('-') {
                    config.size_lt = s.parse().ok();
                } else {
                    config.size_gt = size_str.parse().ok();
                }
            }
        } else if arg == "-print" {
            config.print = true;
        } else if !arg.starts_with('-') {
            config.paths.push(arg.clone());
        } else {
            eprintln!("find: unknown option '{}'", arg);
        }
        i += 1;
    }

    Ok(config)
}

/// 递归搜索
fn find_path(path: &Path, config: &FindConfig, depth: usize) -> io::Result<()> {
    // 检查深度
    if let Some(max) = config.max_depth {
        if depth > max {
            return Ok(());
        }
    }

    if let Some(min) = config.min_depth {
        if depth < min {
            // 继续搜索但不输出
        }
    }

    // 检查当前路径
    let matches = check_path(path, config);

    if matches && depth >= config.min_depth.unwrap_or(0) {
        println!("{}", path.display());
    }

    // 递归搜索目录
    if path.is_dir() {
        let entries = match fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry_result in entries {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let entry_path = entry.path();
            find_path(&entry_path, config, depth + 1)?;
        }
    }

    Ok(())
}

/// 检查路径是否匹配条件
fn check_path(path: &Path, config: &FindConfig) -> bool {
    // 获取文件名
    let file_name = match path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => return false,
    };

    // 名称匹配
    if let Some(ref pattern) = config.name {
        if !glob_match(&file_name, pattern) {
            return false;
        }
    }

    // 正则匹配
    if let Some(ref pattern) = config.regex_name {
        let re = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => return false,
        };
        if !re.is_match(&file_name) {
            return false;
        }
    }

    // 类型匹配
    if let Some(ftype) = config.file_type {
        let is_link = path.is_symlink();
        let is_dir = path.is_dir();
        let is_file = path.is_file();

        match ftype {
            FileType::Link => {
                if !is_link {
                    return false;
                }
            }
            FileType::Dir => {
                if !is_dir {
                    return false;
                }
            }
            FileType::File => {
                if !is_file {
                    return false;
                }
            }
        }
    }

    // 修改时间匹配
    if let Some(mtime_days) = config.mtime {
        if let Ok(meta) = fs::metadata(path) {
            if let Ok(mtime) = meta.modified() {
                let now = Local::now();
                let mtime_dt: DateTime<Local> = mtime.into();
                let days_diff = (now - mtime_dt).num_days();
                if days_diff != mtime_days {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    // 文件大小匹配
    if config.size_gt.is_some() || config.size_lt.is_some() {
        if let Ok(meta) = fs::metadata(path) {
            let size = meta.len();
            if let Some(gt) = config.size_gt {
                if size <= gt {
                    return false;
                }
            }
            if let Some(lt) = config.size_lt {
                if size >= lt {
                    return false;
                }
            }
        } else {
            return false;
        }
    }

    true
}

/// 简单的 glob 匹配
fn glob_match(text: &str, pattern: &str) -> bool {
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

    while pi < pattern_chars.len() && pattern_chars[pi] == '*' {
        pi += 1;
    }

    pi == pattern_chars.len()
}
