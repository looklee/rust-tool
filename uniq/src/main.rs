use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

/// 配置选项
struct UniqConfig {
    /// 忽略大小写
    ignore_case: bool,
    /// 显示重复行
    show_duplicates: bool,
    /// 只显示重复行
    only_duplicates: bool,
    /// 显示计数
    count: bool,
    /// 忽略前 N 个字符
    skip_chars: Option<usize>,
    /// 忽略前 N 个字段
    skip_fields: Option<usize>,
    /// 输入文件
    input: Option<String>,
    /// 输出文件
    output: Option<String>,
}

impl Default for UniqConfig {
    fn default() -> Self {
        Self {
            ignore_case: false,
            show_duplicates: false,
            only_duplicates: false,
            count: false,
            skip_chars: None,
            skip_fields: None,
            input: None,
            output: None,
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let config = parse_args(&args[1..])?;

    // 检查帮助
    if args.len() == 1 || (args.len() == 2 && (args[1] == "-h" || args[1] == "--help")) {
        print_help();
        return Ok(());
    }

    // 读取输入
    let lines: Vec<String> = match &config.input {
        Some(path) if path != "-" => {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            reader.lines().filter_map(|r| r.ok()).collect()
        }
        _ => {
            let stdin = io::stdin();
            let reader = BufReader::new(stdin.lock());
            reader.lines().filter_map(|r| r.ok()).collect()
        }
    };

    // 输出
    let mut output_lock = io::stdout().lock();

    if lines.is_empty() {
        return Ok(());
    }

    let mut prev_line = &lines[0];
    let mut count = 1u64;

    for line in lines.iter().skip(1) {
        let prev_key = normalize_line(prev_line, &config);
        let curr_key = normalize_line(line, &config);

        if prev_key == curr_key {
            count += 1;
        } else {
            // 输出前一行
            if should_output(count, config.ignore_case && prev_key == curr_key, &config) {
                if config.count {
                    writeln!(output_lock, "{:6} {}", count, prev_line)?;
                } else {
                    writeln!(output_lock, "{}", prev_line)?;
                }
            }
            prev_line = line;
            count = 1;
        }
    }

    // 输出最后一行
    if should_output(count, false, &config) {
        if config.count {
            writeln!(output_lock, "{:6} {}", count, prev_line)?;
        } else {
            writeln!(output_lock, "{}", prev_line)?;
        }
    }

    Ok(())
}

/// 规范化行（用于比较）
fn normalize_line(line: &str, config: &UniqConfig) -> String {
    let mut result = line.to_string();

    // 忽略字符
    if let Some(n) = config.skip_chars {
        if n < result.len() {
            result = result[n..].to_string();
        } else {
            result = String::new();
        }
    }

    // 忽略字段
    if let Some(n) = config.skip_fields {
        let parts: Vec<&str> = result.split_whitespace().collect();
        if n < parts.len() {
            result = parts[n..].join(" ");
        } else {
            result = String::new();
        }
    }

    // 忽略大小写
    if config.ignore_case {
        result = result.to_lowercase();
    }

    result
}

/// 是否应该输出
fn should_output(count: u64, _is_same: bool, config: &UniqConfig) -> bool {
    if config.show_duplicates {
        return true;
    }
    if config.only_duplicates {
        return count > 1;
    }
    true
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<UniqConfig> {
    let mut config = UniqConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            break;
        } else if arg == "-i" || arg == "--ignore-case" {
            config.ignore_case = true;
        } else if arg == "-D" || arg == "--all-duplicates" {
            config.show_duplicates = true;
        } else if arg == "-d" || arg == "--repeated" {
            config.only_duplicates = true;
        } else if arg == "-c" || arg == "--count" {
            config.count = true;
        } else if arg == "-s" || arg == "--skip-chars" {
            if i + 1 < args.len() {
                i += 1;
                config.skip_chars = args[i].parse().ok();
            }
        } else if arg == "-f" || arg == "--skip-fields" {
            if i + 1 < args.len() {
                i += 1;
                config.skip_fields = args[i].parse().ok();
            }
        } else if arg == "-u" || arg == "--unique" {
            // 默认行为，只显示唯一行
        } else if !arg.starts_with('-') {
            if config.input.is_none() {
                config.input = Some(arg.clone());
            } else if config.output.is_none() {
                config.output = Some(arg.clone());
            }
        }
        i += 1;
    }

    Ok(config)
}

fn print_help() {
    println!("uniq - report or omit repeated lines");
    println!();
    println!("Usage: uniq [OPTION]... [INPUT [OUTPUT]]");
    println!();
    println!("Options:");
    println!("  -i, --ignore-case       ignore differences in case");
    println!("  -c, --count             prefix lines by the number of occurrences");
    println!("  -d, --repeated          only print duplicate lines");
    println!("  -D, --all-duplicate     print all duplicate lines");
    println!("  -u, --unique            only print unique lines (default)");
    println!("  -s, --skip-chars=N      avoid comparing the first N characters");
    println!("  -f, --skip-fields=N     avoid comparing the first N fields");
    println!("  -h, --help              display this help");
}
