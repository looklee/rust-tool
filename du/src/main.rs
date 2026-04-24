use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 配置选项
struct DuConfig {
    /// 人类可读的文件大小
    human_readable: bool,
    /// 显示总计
    total: bool,
    /// 最大显示深度
    max_depth: Option<usize>,
    /// 只计算指定文件类型
    files_only: bool,
    /// 排序输出
    sort: SortOrder,
}

#[derive(Default, Clone, Copy)]
enum SortOrder {
    #[default]
    None,
    Size,
    Time,
}

impl Default for DuConfig {
    fn default() -> Self {
        Self {
            human_readable: false,
            total: false,
            max_depth: None,
            files_only: false,
            sort: SortOrder::None,
        }
    }
}

/// 目录条目信息
struct DirEntry {
    path: PathBuf,
    size: u64,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let (config, paths) = parse_args(&args[1..])?;

    let targets = if paths.is_empty() {
        vec![".".to_string()]
    } else {
        paths
    };

    let mut grand_total = 0u64;

    for (i, path) in targets.iter().enumerate() {
        if targets.len() > 1 && config.total {
            if i > 0 {
                println!();
            }
        }

        let total = du_path(path, &config)?;
        grand_total += total;
    }

    if config.total && targets.len() > 1 {
        println!("{} total", format_size(grand_total, config.human_readable));
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<(DuConfig, Vec<String>)> {
    let mut config = DuConfig::default();
    let mut paths = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if arg == "--help" || arg == "-h" {
            print_help();
            std::process::exit(0);
        } else if arg == "-h" || arg == "--human-readable" || arg == "--human-numeric" {
            config.human_readable = true;
        } else if arg == "-c" || arg == "--total" {
            config.total = true;
        } else if arg == "-d" || arg == "--max-depth" {
            if i + 1 < args.len() {
                i += 1;
                config.max_depth = args[i].parse().ok();
            }
        } else if let Some(depth_str) = arg.strip_prefix("-d") {
            config.max_depth = depth_str.parse().ok();
        } else if arg == "--max-depth=" {
            if i + 1 < args.len() {
                i += 1;
                config.max_depth = args[i].parse().ok();
            }
        } else if arg == "-s" || arg == "--summarize" {
            config.max_depth = Some(0);
        } else if arg == "-a" || arg == "--all" {
            config.files_only = true;
        } else if arg == "--sort=size" {
            config.sort = SortOrder::Size;
        } else if arg == "--sort=time" {
            config.sort = SortOrder::Time;
        } else if arg.starts_with('-') && arg.chars().nth(1).map_or(false, |c| c.is_ascii_digit()) {
            // -NUM 格式，设置深度
            if let Ok(num) = arg[1..].parse::<usize>() {
                config.max_depth = Some(num);
            }
        } else if arg.starts_with('-') {
            eprintln!("du: invalid option '{}'", arg);
            eprintln!("Try 'du --help' for more information.");
            std::process::exit(1);
        } else {
            paths.push(arg.clone());
        }
        i += 1;
    }

    Ok((config, paths))
}

fn print_help() {
    println!("du - estimate file space usage");
    println!();
    println!("Usage: du [OPTION]... [FILE]...");
    println!();
    println!("Options:");
    println!("  -h, --human-readable         print sizes in human readable format (e.g., 1K, 2M)");
    println!("  -c, --total                  produce a grand total");
    println!("  -d, --max-depth=N            print the total for a directory only if it is N or fewer");
    println!("                               levels below the command line argument");
    println!("  -s, --summarize              display only a total for each argument");
    println!("  -a, --all                    write counts for all files, not just directories");
    println!("  --sort=size                  sort by size (largest first)");
    println!("  --sort=time                  sort by modification time (newest first)");
    println!("  --help                       display this help and exit");
}

/// 处理路径
fn du_path(path_str: &str, config: &DuConfig) -> io::Result<u64> {
    let path = Path::new(path_str);

    if !path.exists() {
        eprintln!("du: cannot access '{}': No such file or directory", path_str);
        return Ok(0);
    }

    let mut entries: Vec<DirEntry> = Vec::new();
    let total = calculate_size(path, 0, config.max_depth, &mut entries)?;

    // 如果不显示所有文件，只打印目录
    if !config.files_only {
        // 排序
        match config.sort {
            SortOrder::Size => {
                entries.sort_by(|a, b| b.size.cmp(&a.size));
            }
            SortOrder::Time => {
                entries.sort_by(|a, b| {
                    let a_time = a.path.metadata().and_then(|m| m.modified()).ok();
                    let b_time = b.path.metadata().and_then(|m| m.modified()).ok();
                    b_time.cmp(&a_time)
                });
            }
            SortOrder::None => {}
        }

        for entry in &entries {
            println!("{}\t{}", format_size(entry.size, config.human_readable), entry.path.display());
        }
    } else {
        // 显示所有文件
        for entry in &entries {
            println!("{}\t{}", format_size(entry.size, config.human_readable), entry.path.display());
        }
    }

    // 打印当前路径的总计
    if config.max_depth.is_none() || config.max_depth == Some(0) {
        println!("{}\t{}", format_size(total, config.human_readable), path.display());
    }

    Ok(total)
}

/// 递归计算大小
fn calculate_size(
    path: &Path,
    depth: usize,
    max_depth: Option<usize>,
    entries: &mut Vec<DirEntry>,
) -> io::Result<u64> {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return Ok(0),
    };

    if metadata.is_file() {
        entries.push(DirEntry {
            path: path.to_path_buf(),
            size: metadata.len(),
        });
        return Ok(metadata.len());
    }

    if metadata.is_dir() {
        // 检查是否超过最大深度
        if let Some(max) = max_depth {
            if depth > max {
                return Ok(0);
            }
        }

        let mut total_size = 0u64;

        // 读取目录内容
        if let Ok(read_dir) = fs::read_dir(path) {
            for entry_result in read_dir.flatten() {
                let entry_path = entry_result.path();
                let size = calculate_size(&entry_path, depth + 1, max_depth, entries)?;
                total_size += size;
            }
        }

        // 添加目录本身的条目
        entries.push(DirEntry {
            path: path.to_path_buf(),
            size: total_size,
        });

        return Ok(total_size);
    }

    Ok(0)
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
