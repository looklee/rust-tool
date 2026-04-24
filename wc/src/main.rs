use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

/// 配置选项
struct WcConfig {
    /// 显示行数
    lines: bool,
    /// 显示单词数
    words: bool,
    /// 显示字节数
    bytes: bool,
    /// 显示字符数
    chars: bool,
    /// 显示最大行长度
    max_line_length: bool,
    /// 输入文件
    inputs: Vec<String>,
}

impl Default for WcConfig {
    fn default() -> Self {
        Self {
            lines: false,
            words: false,
            bytes: false,
            chars: false,
            max_line_length: false,
            inputs: Vec::new(),
        }
    }
}

struct FileStats {
    lines: u64,
    words: u64,
    bytes: u64,
    chars: u64,
    max_line_len: u64,
}

impl Default for FileStats {
    fn default() -> Self {
        Self {
            lines: 0,
            words: 0,
            bytes: 0,
            chars: 0,
            max_line_len: 0,
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let config = parse_args(&args[1..])?;

    // 检查帮助或无参数
    if args.len() == 1 || (args.len() == 2 && (args[1] == "-h" || args[1] == "--help")) {
        print_help();
        return Ok(());
    }

    // 如果未指定任何计数选项，显示所有
    let show_all = !config.lines && !config.words && !config.bytes && !config.chars && !config.max_line_length;

    // 处理文件
    let mut total = FileStats::default();
    let mut results: Vec<(String, FileStats)> = Vec::new();

    let inputs = if config.inputs.is_empty() {
        vec!["-".to_string()]
    } else {
        config.inputs.clone()
    };

    for input in &inputs {
        let stats = wc_input(input, &config)?;
        
        total.lines += stats.lines;
        total.words += stats.words;
        total.bytes += stats.bytes;
        total.chars += stats.chars;
        if stats.max_line_len > total.max_line_len {
            total.max_line_len = stats.max_line_len;
        }

        results.push((input.clone(), stats));
    }

    // 输出
    for (filename, stats) in &results {
        print_stats(stats, filename, &config, show_all, inputs.len() > 1);
    }

    if inputs.len() > 1 {
        print_stats(&total, "total", &config, show_all, true);
    }

    Ok(())
}

/// 处理单个输入
fn wc_input(input: &str, _config: &WcConfig) -> io::Result<FileStats> {
    let mut stats = FileStats::default();

    let reader: Box<dyn BufRead> = match input {
        "-" => Box::new(BufReader::new(io::stdin())),
        path => {
            let file = File::open(path)?;
            Box::new(BufReader::new(file))
        }
    };

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        stats.lines += 1;
        stats.bytes += line.len() as u64 + 1; // +1 for newline
        stats.chars += line.chars().count() as u64 + 1;
        stats.words += line.split_whitespace().count() as u64;

        if line.len() as u64 > stats.max_line_len {
            stats.max_line_len = line.len() as u64;
        }
    }

    Ok(stats)
}

/// 打印统计
fn print_stats(stats: &FileStats, filename: &str, config: &WcConfig, show_all: bool, show_filename: bool) {
    let mut parts: Vec<String> = Vec::new();

    if show_all {
        parts.push(format!("{:>8}", stats.lines));
        parts.push(format!("{:>8}", stats.words));
        parts.push(format!("{:>8}", stats.bytes));
    } else {
        if config.lines {
            parts.push(format!("{:>8}", stats.lines));
        }
        if config.words {
            parts.push(format!("{:>8}", stats.words));
        }
        if config.bytes {
            parts.push(format!("{:>8}", stats.bytes));
        }
        if config.chars {
            parts.push(format!("{:>8}", stats.chars));
        }
        if config.max_line_length {
            parts.push(format!("{:>8}", stats.max_line_len));
        }
    }

    if show_filename {
        println!("{} {}", parts.join(" "), filename);
    } else {
        println!("{}", parts.join(" "));
    }
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<WcConfig> {
    let mut config = WcConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            break;
        } else if arg == "-l" || arg == "--lines" {
            config.lines = true;
        } else if arg == "-w" || arg == "--words" {
            config.words = true;
        } else if arg == "-c" || arg == "--bytes" {
            config.bytes = true;
        } else if arg == "-m" || arg == "--chars" {
            config.chars = true;
        } else if arg == "-L" || arg == "--max-line-length" {
            config.max_line_length = true;
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            // 短选项组合
            for flag in arg[1..].chars() {
                match flag {
                    'l' => config.lines = true,
                    'w' => config.words = true,
                    'c' => config.bytes = true,
                    'm' => config.chars = true,
                    'L' => config.max_line_length = true,
                    'h' => break,
                    _ => {
                        eprintln!("wc: invalid option -- '{}'", flag);
                        std::process::exit(1);
                    }
                }
            }
        } else if !arg.starts_with('-') {
            config.inputs.push(arg.clone());
        }
        i += 1;
    }

    Ok(config)
}

fn print_help() {
    println!("wc - print newline, word, and byte counts");
    println!();
    println!("Usage: wc [OPTION]... [FILE]...");
    println!();
    println!("Options:");
    println!("  -l, --lines       print the newline counts");
    println!("  -w, --words       print the word counts");
    println!("  -c, --bytes       print the byte counts");
    println!("  -m, --chars       print the character counts");
    println!("  -L, --max-line-length  print the length of the longest line");
    println!("  -h, --help        display this help");
    println!();
    println!("With no FILE, or when FILE is -, read standard input.");
}
