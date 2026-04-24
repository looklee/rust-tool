use chrono::{DateTime, Local};
use std::env;
use std::fs::{self, Metadata};
use std::io;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;
use std::time::SystemTime;

/// ANSI 颜色代码
struct Colors {
    reset: &'static str,
    bold: &'static str,
}

impl Colors {
    fn new(enabled: bool) -> Self {
        if enabled {
            Self {
                reset: "\x1b[0m",
                bold: "\x1b[1m",
            }
        } else {
            Self {
                reset: "",
                bold: "",
            }
        }
    }
}

/// 排序方式
#[derive(Default, Clone, Copy)]
enum SortOrder {
    #[default]
    Name,
    Time,      // 按修改时间排序
    Size,      // 按文件大小排序
}

/// 配置选项
struct LsConfig {
    /// 详细列表
    long_format: bool,
    /// 显示隐藏文件
    show_hidden: bool,
    /// 显示所有文件（包括.和..）
    show_all: bool,
    /// 彩色输出
    color: bool,
    /// 人类可读的文件大小
    human_readable: bool,
    /// 排序方式
    sort_order: SortOrder,
    /// 反向排序
    reverse: bool,
}

impl Default for LsConfig {
    fn default() -> Self {
        Self {
            long_format: false,
            show_hidden: false,
            show_all: false,
            color: false,
            human_readable: false,
            sort_order: SortOrder::Name,
            reverse: false,
        }
    }
}

/// 文件条目信息
struct FileEntry {
    name: String,
    metadata: Option<Metadata>,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    // 解析参数
    let (config, paths) = parse_args(&args[1..]);

    let targets = if paths.is_empty() {
        vec![".".to_string()]
    } else {
        paths
    };

    for (i, path) in targets.iter().enumerate() {
        if targets.len() > 1 {
            if i > 0 {
                println!();
            }
            println!("{}:", path);
        }

        list_path(path, &config)?;
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> (LsConfig, Vec<String>) {
    let mut config = LsConfig::default();
    let mut paths = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with('-') && arg.len() > 1 {
            // 检查是否是长选项
            if arg == "--color" || arg == "--colour" {
                config.color = true;
            } else if arg == "--no-color" || arg == "--no-colour" {
                config.color = false;
            } else if arg == "-h" || arg == "--help" {
                print_help();
                std::process::exit(0);
            } else if arg == "--human-readable" || arg == "--si" {
                config.human_readable = true;
            } else if arg == "-t" {
                config.sort_order = SortOrder::Time;
            } else if arg == "-S" {
                config.sort_order = SortOrder::Size;
            } else if arg == "-r" || arg == "--reverse" {
                config.reverse = true;
            } else {
                // 短选项
                for flag in arg[1..].chars() {
                    match flag {
                        'l' => config.long_format = true,
                        'a' => config.show_all = true,
                        'h' => config.show_hidden = true, // 简化：-h 同 -a
                        _ => {}
                    }
                }
            }
        } else {
            paths.push(arg.clone());
        }
        i += 1;
    }

    // -a 隐含显示隐藏文件
    if config.show_all {
        config.show_hidden = true;
    }

    // 如果未指定 --color，检查是否输出到终端
    if !config.color {
        config.color = atty::is(atty::Stream::Stdout);
    }

    (config, paths)
}

fn print_help() {
    println!("ls - list directory contents");
    println!();
    println!("Usage: ls [OPTIONS] [FILE]...");
    println!();
    println!("Options:");
    println!("  -l              use a long listing format");
    println!("  -a              do not ignore entries starting with .");
    println!("  -h              show hidden files (alias for -a)");
    println!("  --color         colorize the output");
    println!("  --no-color      disable color output");
    println!("  --human-readable  print sizes in human readable format (e.g., 1K, 2M)");
    println!("  -t              sort by modification time, newest first");
    println!("  -S              sort by file size, largest first");
    println!("  -r, --reverse   reverse the order of the sort");
    println!("  --help          display this help and exit");
}

/// 列出路径内容
fn list_path(path_str: &str, config: &LsConfig) -> io::Result<()> {
    let path = Path::new(path_str);

    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("ls: cannot access '{}': {}", path_str, e);
            return Ok(());
        }
    };

    if metadata.is_file() {
        // 单文件，直接显示
        let entry = FileEntry {
            name: path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path_str)
                .to_string(),
            metadata: Some(metadata),
        };

        if config.long_format {
            print_entry_long(&entry, config);
        } else {
            println!("{}", entry.name);
        }
        return Ok(());
    }

    // 目录：读取内容
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("ls: cannot open directory '{}': {}", path_str, e);
            return Ok(());
        }
    };

    let mut files: Vec<FileEntry> = Vec::new();

    for entry_result in entries {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name()
            .to_string_lossy()
            .to_string();

        // 过滤隐藏文件
        if !config.show_hidden && name.starts_with('.') {
            continue;
        }

        // 如果没有 -a，跳过.和..
        if !config.show_all && (name == "." || name == "..") {
            continue;
        }

        let metadata = entry.metadata().ok();

        files.push(FileEntry {
            name,
            metadata,
        });
    }

    // 排序
    sort_entries(&mut files, config);

    let colors = Colors::new(config.color);

    if config.long_format {
        // 详细列表
        for entry in &files {
            print_entry_long(entry, config);
        }
    } else {
        // 简单列表：多列输出
        print_entries_columns(&files, &colors);
    }

    Ok(())
}

/// 排序文件条目
fn sort_entries(entries: &mut [FileEntry], config: &LsConfig) {
    match config.sort_order {
        SortOrder::Name => {
            entries.sort_by(|a, b| a.name.cmp(&b.name));
        }
        SortOrder::Time => {
            entries.sort_by(|a, b| {
                let a_time = a.metadata.as_ref().and_then(|m| m.modified().ok());
                let b_time = b.metadata.as_ref().and_then(|m| m.modified().ok());
                b_time.cmp(&a_time) // 新的在前
            });
        }
        SortOrder::Size => {
            entries.sort_by(|a, b| {
                let a_size = a.metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let b_size = b.metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                b_size.cmp(&a_size) // 大的在前
            });
        }
    }

    if config.reverse {
        entries.reverse();
    }
}

/// 格式化人类可读的文件大小
fn format_size(size: u64, human_readable: bool) -> String {
    if !human_readable {
        return size.to_string();
    }

    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size >= TB {
        format!("{:.1}T", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{}B", size)
    }
}

/// 打印详细列表行
fn print_entry_long(entry: &FileEntry, config: &LsConfig) {
    let metadata = match &entry.metadata {
        Some(m) => m,
        None => {
            println!("?????????? ? ? ? ? ? {}", entry.name);
            return;
        }
    };

    let colors = Colors::new(config.color);

    // 权限
    let perms = metadata.permissions();
    let mode = perms.mode() & 0o777;
    let perm_str = format_permissions(mode, metadata.file_type());

    // 硬链接数（简化为 1）
    let nlink = 1;

    // 所有者（简化）
    let owner = "user";
    let group = "user";

    // 大小（人类可读）
    let size_str = format_size(metadata.len(), config.human_readable);

    // 修改时间
    let mtime = metadata.modified()
        .map(|t| format_time(t))
        .unwrap_or_else(|_| "?".to_string());

    // 名称（目录加斜杠，带颜色）
    let name = if metadata.is_dir() {
        format!("{}{}/{}", colors.bold, entry.name, colors.reset)
    } else if metadata.is_symlink() {
        format!("{}{}@{}", colors.bold, entry.name, colors.reset)
    } else if mode & 0o111 != 0 {
        // 可执行文件
        format!("{}{}*{}", colors.bold, entry.name, colors.reset)
    } else {
        entry.name.clone()
    };

    println!("{} {} {} {} {:>6} {} {}",
        perm_str, nlink, owner, group, size_str, mtime, name);
}

/// 格式化权限字符串
fn format_permissions(mode: u32, file_type: fs::FileType) -> String {
    let mut result = String::with_capacity(10);

    // 文件类型
    result.push(if file_type.is_dir() {
        'd'
    } else if file_type.is_symlink() {
        'l'
    } else if file_type.is_file() {
        '-'
    } else if file_type.is_fifo() {
        'p'
    } else if file_type.is_socket() {
        's'
    } else {
        '?'
    });

    // 所有者权限
    result.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o100 != 0 { 'x' } else { '-' });

    // 组权限
    result.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o010 != 0 { 'x' } else { '-' });

    // 其他人权限
    result.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o001 != 0 { 'x' } else { '-' });

    result
}

/// 格式化时间
fn format_time(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    let now = Local::now();
    let six_months_ago = now - chrono::Duration::days(180);

    if datetime > six_months_ago {
        // 今年：显示 月 日 时间
        datetime.format("%b %d %H:%M").to_string()
    } else {
        // 超过 6 个月：显示 月 日 年
        datetime.format("%b %d  %Y").to_string()
    }
}

/// 多列打印（带颜色）
fn print_entries_columns(entries: &[FileEntry], colors: &Colors) {
    if entries.is_empty() {
        return;
    }

    // 计算最大名称长度
    let max_len = entries.iter()
        .map(|e| e.name.len())
        .max()
        .unwrap_or(0);

    // 终端宽度（假设 80）
    let term_width = 80;
    let col_width = max_len + 2; // 2 个空格间隔
    let num_cols = (term_width / col_width).max(1);

    // 计算行数
    let num_rows = (entries.len() + num_cols - 1) / num_cols;

    for row in 0..num_rows {
        let mut line = String::new();
        for col in 0..num_cols {
            let idx = row + col * num_rows;
            if idx >= entries.len() {
                break;
            }

            let entry = &entries[idx];
            let is_dir = entry.metadata.as_ref()
                .map(|m| m.is_dir())
                .unwrap_or(false);
            let is_symlink = entry.metadata.as_ref()
                .map(|m| m.is_symlink())
                .unwrap_or(false);

            let display_name = if is_dir {
                format!("{}{}/{}", colors.bold, entry.name, colors.reset)
            } else if is_symlink {
                format!("{}{}@{}", colors.bold, entry.name, colors.reset)
            } else {
                entry.name.clone()
            };

            let cell = format!("{:<width$}", display_name, width = col_width - 1);
            line.push_str(&cell);
        }
        println!("{}", line.trim_end());
    }
}
