use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

/// 配置选项
struct TailConfig {
    /// 显示的行数
    lines: usize,
    /// 跟随模式
    follow: bool,
    /// 跟随模式间隔（秒）
    sleep_interval: f64,
    /// 重试打开文件（-F 选项）
    retry: bool,
}

impl Default for TailConfig {
    fn default() -> Self {
        Self {
            lines: 10,
            follow: false,
            sleep_interval: 1.0,
            retry: false,
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let (config, files) = parse_args(&args[1..])?;

    let targets = if files.is_empty() {
        vec!["-".to_string()]
    } else {
        files
    };

    for (i, file_path) in targets.iter().enumerate() {
        if targets.len() > 1 {
            if i > 0 {
                println!();
            }
            println!("==> {} <==", file_path);
        }

        if file_path == "-" {
            // 标准输入：直接读取并显示最后 N 行
            let stdin = io::stdin();
            tail_stdin(stdin.lock(), &config)?;
        } else {
            let path = Path::new(file_path);
            if !path.exists() {
                eprintln!("tail: cannot open '{}' for reading: No such file or directory", file_path);
                continue;
            }

            if config.retry || config.follow {
                tail_file_with_follow(path, &config)?;
            } else {
                let file = File::open(path)?;
                tail_file(file, &config)?;
            }
        }
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<(TailConfig, Vec<String>)> {
    let mut config = TailConfig::default();
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if arg == "--help" || arg == "-h" {
            print_help();
            std::process::exit(0);
        } else if arg == "-f" || arg == "--follow" {
            config.follow = true;
        } else if arg == "-F" {
            config.follow = true;
            config.retry = true;
        } else if arg == "--retry" {
            config.retry = true;
        } else if arg == "-n" || arg == "--lines" {
            // -n 后面跟数字
            if i + 1 < args.len() {
                i += 1;
                config.lines = args[i].parse().unwrap_or(10);
            }
        } else if let Some(lines_str) = arg.strip_prefix("-n") {
            config.lines = lines_str.parse().unwrap_or(10);
        } else if let Some(lines_str) = arg.strip_prefix("--lines=") {
            config.lines = lines_str.parse().unwrap_or(10);
        } else if arg.starts_with('-') && arg.len() > 1 && arg.chars().nth(1).map_or(false, |c| c.is_ascii_digit()) {
            // -NUM 格式
            if let Ok(num) = arg[1..].parse::<usize>() {
                config.lines = num;
            }
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            eprintln!("tail: invalid option -- '{}'", arg);
            eprintln!("Try 'tail --help' for more information.");
            std::process::exit(1);
        } else if arg.starts_with("--") {
            eprintln!("tail: unrecognized option '{}'", arg);
            eprintln!("Try 'tail --help' for more information.");
            std::process::exit(1);
        } else {
            files.push(arg.clone());
        }
        i += 1;
    }

    Ok((config, files))
}

fn print_help() {
    println!("tail - output the last part of files");
    println!();
    println!("Usage: tail [OPTION]... [FILE]...");
    println!();
    println!("Options:");
    println!("  -n, --lines=NUM          output the last NUM lines, instead of the last 10");
    println!("  -f, --follow             output appended data as the file grows");
    println!("  -F                       same as --follow --retry");
    println!("  --retry                  keep trying to open a file if it is inaccessible");
    println!("  -h, --help               display this help and exit");
    println!();
    println!("With no FILE, or when FILE is -, read standard input.");
}

/// 处理标准输入
fn tail_stdin<R: BufRead>(reader: R, config: &TailConfig) -> io::Result<()> {
    let mut lines: Vec<String> = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        lines.push(line);
        // 保持只有 N 行
        while lines.len() > config.lines {
            lines.remove(0);
        }
    }

    // 输出
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    for line in &lines {
        writeln!(stdout_lock, "{}", line)?;
    }
    stdout_lock.flush()?;

    Ok(())
}

/// 处理文件（带 Seek）
fn tail_file(mut file: File, config: &TailConfig) -> io::Result<()> {
    // 移动到文件末尾
    file.seek(SeekFrom::End(0))?;
    let file_size = file.seek(SeekFrom::Current(0))?;

    if file_size == 0 {
        return Ok(());
    }

    // 反向查找最后 N 行
    let mut lines: Vec<String> = Vec::new();
    let mut pos = file_size;
    let mut buffer = vec![0u8; 1];
    let mut current_line = Vec::new();

    while pos > 0 && lines.len() <= config.lines {
        pos -= 1;
        file.seek(SeekFrom::Start(pos))?;
        file.read_exact(&mut buffer)?;

        let byte = buffer[0];
        if byte == b'\n' && pos < file_size - 1 {
            // 找到一行
            current_line.reverse();
            lines.push(String::from_utf8_lossy(&current_line).to_string());
            current_line.clear();
            if lines.len() >= config.lines + 1 {
                break;
            }
        } else {
            current_line.push(byte);
        }
    }

    // 如果文件第一行前面还有内容，加入
    if pos == 0 && !current_line.is_empty() {
        current_line.reverse();
        lines.push(String::from_utf8_lossy(&current_line).to_string());
    }

    // 反转并输出
    lines.reverse();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line in lines.iter().take(config.lines) {
        writeln!(stdout_lock, "{}", line)?;
    }
    stdout_lock.flush()?;

    Ok(())
}

/// 带跟随模式的文件处理
fn tail_file_with_follow(path: &Path, config: &TailConfig) -> io::Result<()> {
    // 尝试打开文件
    let file = loop {
        match File::open(path) {
            Ok(f) => break Some(f),
            Err(_) => {
                if config.retry {
                    thread::sleep(Duration::from_secs_f64(config.sleep_interval));
                    continue;
                } else {
                    return Err(io::Error::new(io::ErrorKind::NotFound, "File not found"));
                }
            }
        }
    };

    // 初始 tail
    if let Some(mut f) = file {
        // 先显示最后 N 行
        tail_file(f.try_clone()?, config)?;

        // 获取当前文件位置
        let mut pos = f.seek(SeekFrom::End(0))?;

        // 跟随模式
        if config.follow {
            let stdout = io::stdout();
            let mut stdout_lock = stdout.lock();
            let mut f = f;

            loop {
                // 检查文件是否有新内容
                if let Ok(meta) = f.metadata() {
                    let current_size = meta.len();

                    if current_size > pos {
                        // 有新内容
                        f.seek(SeekFrom::Start(pos))?;
                        let mut reader = BufReader::new(&f);
                        let mut buf = Vec::new();
                        reader.read_to_end(&mut buf)?;

                        stdout_lock.write_all(&buf)?;
                        stdout_lock.flush()?;

                        pos = current_size;
                    } else if current_size < pos {
                        // 文件被截断，重新从头开始
                        pos = 0;
                    }
                }

                thread::sleep(Duration::from_secs_f64(config.sleep_interval));
            }
        }
    }

    Ok(())
}
