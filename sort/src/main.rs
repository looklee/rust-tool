use std::cmp::Ordering;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

/// 配置选项
struct SortConfig {
    /// 反向排序
    reverse: bool,
    /// 忽略大小写
    ignore_case: bool,
    /// 忽略前导空白
    ignore_blank: bool,
    /// 唯一行
    unique: bool,
    /// 排序键（列）
    key: Option<usize>,
    /// 分隔符
    delimiter: char,
    /// 数值排序
    numeric: bool,
    /// 随机排序
    random: bool,
    /// 输入文件
    input: Option<String>,
    /// 输出文件
    output: Option<String>,
}

impl Default for SortConfig {
    fn default() -> Self {
        Self {
            reverse: false,
            ignore_case: false,
            ignore_blank: false,
            unique: false,
            key: None,
            delimiter: ' ',
            numeric: false,
            random: false,
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

    // 排序
    let mut sorted = sort_lines(&lines, &config);

    // 唯一性过滤
    if config.unique {
        sorted.dedup();
    }

    // 输出
    let output: Box<dyn Write> = match &config.output {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(io::stdout()),
    };

    let mut output_lock = output;
    for line in sorted {
        writeln!(output_lock, "{}", line)?;
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<SortConfig> {
    let mut config = SortConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            break;
        } else if arg == "-r" || arg == "--reverse" {
            config.reverse = true;
        } else if arg == "-f" || arg == "--ignore-case" {
            config.ignore_case = true;
        } else if arg == "-b" || arg == "--ignore-leading-blanks" {
            config.ignore_blank = true;
        } else if arg == "-u" || arg == "--unique" {
            config.unique = true;
        } else if arg == "-n" || arg == "--numeric-sort" {
            config.numeric = true;
        } else if arg == "-R" || arg == "--random-sort" {
            config.random = true;
        } else if arg == "-t" || arg == "--field-separator" {
            if i + 1 < args.len() {
                i += 1;
                config.delimiter = args[i].chars().next().unwrap_or(' ');
            }
        } else if arg == "-k" || arg == "--key" {
            if i + 1 < args.len() {
                i += 1;
                config.key = args[i].parse().ok();
            }
        } else if arg == "-o" || arg == "--output" {
            if i + 1 < args.len() {
                i += 1;
                config.output = Some(args[i].clone());
            }
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            // 短选项组合
            let mut j = 1;
            while j < arg.len() {
                let flag = arg.chars().nth(j).unwrap();
                match flag {
                    'r' => config.reverse = true,
                    'f' => config.ignore_case = true,
                    'b' => config.ignore_blank = true,
                    'u' => config.unique = true,
                    'n' => config.numeric = true,
                    'R' => config.random = true,
                    'h' => break,
                    _ => {
                        eprintln!("sort: invalid option -- '{}'", flag);
                        std::process::exit(1);
                    }
                }
                j += 1;
            }
        } else if !arg.starts_with('-') {
            if config.input.is_none() {
                config.input = Some(arg.clone());
            } else {
                eprintln!("sort: multiple input files not supported");
            }
        }
        i += 1;
    }

    Ok(config)
}

fn print_help() {
    println!("sort - sort lines of text files");
    println!();
    println!("Usage: sort [OPTION]... [FILE]...");
    println!();
    println!("Options:");
    println!("  -r, --reverse              reverse the output");
    println!("  -f, --ignore-case          fold lower case to upper case");
    println!("  -b, --ignore-leading-blanks  ignore leading blanks");
    println!("  -u, --unique               output only unique lines");
    println!("  -n, --numeric-sort         compare according to string numerical value");
    println!("  -R, --random-sort          shuffle lines randomly");
    println!("  -t, --field-separator=C    use C as field separator");
    println!("  -k, --key=N                sort by Nth column");
    println!("  -o, --output=FILE          write to FILE instead of stdout");
    println!("  -h, --help                 display this help");
}

/// 排序行
fn sort_lines(lines: &[String], config: &SortConfig) -> Vec<String> {
    let mut result: Vec<String> = lines.to_vec();

    if config.random {
        // 随机排序
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        result.sort_by(|a, b| {
            let hash_a = simple_hash(a, seed);
            let hash_b = simple_hash(b, seed);
            hash_a.cmp(&hash_b)
        });
    } else {
        result.sort_by(|a, b| compare_lines(a, b, config));
    }

    result
}

/// 简单哈希函数
fn simple_hash(s: &str, seed: u64) -> u64 {
    let mut hash = seed;
    for byte in s.bytes() {
        hash = hash.wrapping_add(u64::from(byte));
        hash = hash.wrapping_mul(31);
    }
    hash
}

/// 比较两行
fn compare_lines(a: &str, b: &str, config: &SortConfig) -> Ordering {
    let mut a_str = a;
    let mut b_str = b;

    // 忽略前导空白
    if config.ignore_blank {
        a_str = a_str.trim_start();
        b_str = b_str.trim_start();
    }

    // 忽略大小写
    let a_cmp = if config.ignore_case {
        a_str.to_lowercase()
    } else {
        a_str.to_string()
    };
    let b_cmp = if config.ignore_case {
        b_str.to_lowercase()
    } else {
        b_str.to_string()
    };

    // 按列排序
    let (a_key, b_key) = if let Some(key) = config.key {
        let a_parts: Vec<&str> = a_cmp.split(config.delimiter).collect();
        let b_parts: Vec<&str> = b_cmp.split(config.delimiter).collect();
        (
            a_parts.get(key.saturating_sub(1)).copied().unwrap_or(""),
            b_parts.get(key.saturating_sub(1)).copied().unwrap_or(""),
        )
    } else {
        (a_cmp.as_str(), b_cmp.as_str())
    };

    // 数值排序
    let ordering = if config.numeric {
        let a_num: f64 = a_key.parse().unwrap_or(0.0);
        let b_num: f64 = b_key.parse().unwrap_or(0.0);
        a_num.partial_cmp(&b_num).unwrap_or(Ordering::Equal)
    } else {
        a_key.cmp(b_key)
    };

    if config.reverse {
        ordering.reverse()
    } else {
        ordering
    }
}
