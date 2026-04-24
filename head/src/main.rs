use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write, Read};

/// 配置选项
struct HeadConfig {
    /// 显示的行数
    lines: usize,
    /// 显示的字节数
    bytes: Option<usize>,
    /// 输入文件
    inputs: Vec<String>,
    /// 详细模式（多文件时显示文件名）
    verbose: bool,
}

impl Default for HeadConfig {
    fn default() -> Self {
        Self {
            lines: 10,
            bytes: None,
            inputs: Vec::new(),
            verbose: false,
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut config = parse_args(&args[1..])?;

    // 检查帮助或无参数
    if args.len() == 1 || (args.len() == 2 && (args[1] == "-h" || args[1] == "--help")) {
        print_help();
        return Ok(());
    }

    let inputs = if config.inputs.is_empty() {
        vec!["-".to_string()]
    } else {
        config.inputs.clone()
    };

    config.verbose = inputs.len() > 1;

    for (i, input) in inputs.iter().enumerate() {
        if inputs.len() > 1 {
            if i > 0 {
                println!();
            }
            println!("==> {} <==", input);
        }

        head_input(input, &config)?;
    }

    Ok(())
}

/// 处理单个输入
fn head_input(input: &str, config: &HeadConfig) -> io::Result<()> {
    let reader: Box<dyn BufRead> = match input {
        "-" => Box::new(BufReader::new(io::stdin())),
        path => {
            let file = File::open(path)?;
            Box::new(BufReader::new(file))
        }
    };

    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    if let Some(bytes_count) = config.bytes {
        // 按字节读取
        let mut buffer = vec![0u8; bytes_count];
        let mut handle = reader.take(bytes_count as u64);
        let n = handle.read(&mut buffer)?;
        stdout_lock.write_all(&buffer[..n])?;
    } else {
        // 按行读取
        for (i, line_result) in reader.lines().enumerate() {
            if i >= config.lines {
                break;
            }
            let line = line_result?;
            writeln!(stdout_lock, "{}", line)?;
        }
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<HeadConfig> {
    let mut config = HeadConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            break;
        } else if arg == "-n" || arg == "--lines" {
            if i + 1 < args.len() {
                i += 1;
                config.lines = args[i].parse().unwrap_or(10);
            }
        } else if let Some(n) = arg.strip_prefix("-n") {
            config.lines = n.parse().unwrap_or(10);
        } else if arg == "-c" || arg == "--bytes" {
            if i + 1 < args.len() {
                i += 1;
                config.bytes = args[i].parse().ok();
            }
        } else if let Some(n) = arg.strip_prefix("-c") {
            config.bytes = n.parse().ok();
        } else if arg == "-q" || arg == "--quiet" || arg == "--silent" {
            config.verbose = false;
        } else if arg == "-v" || arg == "--verbose" {
            config.verbose = true;
        } else if arg.starts_with('-') && arg.len() > 1 && arg.chars().nth(1).map_or(false, |c| c.is_ascii_digit()) {
            // -NUM 格式
            if let Ok(num) = arg[1..].parse::<usize>() {
                config.lines = num;
            }
        } else if arg.starts_with('-') {
            eprintln!("head: invalid option '{}'", arg);
            std::process::exit(1);
        } else {
            config.inputs.push(arg.clone());
        }
        i += 1;
    }

    Ok(config)
}

fn print_help() {
    println!("head - output the first part of files");
    println!();
    println!("Usage: head [OPTION]... [FILE]...");
    println!();
    println!("Options:");
    println!("  -n, --lines=[-]NUM   output the first NUM lines instead of the first 10");
    println!("  -c, --bytes=[-]NUM   output the first NUM bytes of each file");
    println!("  -q, --quiet, --silent  never print headers giving file names");
    println!("  -v, --verbose        always print headers giving file names");
    println!("  -h, --help           display this help and exit");
    println!();
    println!("With no FILE, or when FILE is -, read standard input.");
}
