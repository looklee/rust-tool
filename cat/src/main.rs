use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;

/// 配置选项
struct CatConfig {
    /// 显示行号
    show_line_numbers: bool,
    /// 压缩空行（不编号连续空行）
    squeeze_blank: bool,
    /// 显示行尾符
    show_ends: bool,
    /// 显示制表符
    show_tabs: bool,
    /// 显示非打印字符（^ 前缀）
    show_nonprinting: bool,
}

impl Default for CatConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            squeeze_blank: false,
            show_ends: false,
            show_tabs: false,
            show_nonprinting: false,
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

    for file_path in &targets {
        if file_path == "-" {
            // 从标准输入读取
            let stdin = io::stdin();
            cat_reader(stdin.lock(), &config)?;
        } else {
            let path = Path::new(file_path);
            if !path.exists() {
                eprintln!("cat: {}: No such file or directory", file_path);
                continue;
            }
            if !path.is_file() {
                eprintln!("cat: {}: Is a directory", file_path);
                continue;
            }
            let file = File::open(path)?;
            cat_reader(file, &config)?;
        }
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<(CatConfig, Vec<String>)> {
    let mut config = CatConfig::default();
    let mut files = Vec::new();

    for arg in args {
        if arg == "--help" || arg == "-h" {
            print_help();
            std::process::exit(0);
        } else if arg == "-n" || arg == "--number" {
            config.show_line_numbers = true;
        } else if arg == "-b" || arg == "--number-nonblank" {
            config.show_line_numbers = true;
            config.squeeze_blank = true; // -b 隐含 -s
        } else if arg == "-s" || arg == "--squeeze-blank" {
            config.squeeze_blank = true;
        } else if arg == "-E" || arg == "--show-ends" {
            config.show_ends = true;
        } else if arg == "-T" || arg == "--show-tabs" {
            config.show_tabs = true;
        } else if arg == "-A" || arg == "--show-all" {
            config.show_nonprinting = true;
            config.show_ends = true;
            config.show_tabs = true;
        } else if arg == "-v" || arg == "--show-nonprinting" {
            config.show_nonprinting = true;
        } else if arg == "-t" {
            config.show_tabs = true;
        } else if arg == "-e" {
            config.show_ends = true;
            config.show_nonprinting = true;
        } else if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") {
            // 组合短选项
            for flag in arg[1..].chars() {
                match flag {
                    'n' => config.show_line_numbers = true,
                    'b' => {
                        config.show_line_numbers = true;
                        config.squeeze_blank = true;
                    }
                    's' => config.squeeze_blank = true,
                    'E' => config.show_ends = true,
                    'T' => config.show_tabs = true,
                    'A' => {
                        config.show_nonprinting = true;
                        config.show_ends = true;
                        config.show_tabs = true;
                    }
                    'v' => config.show_nonprinting = true,
                    't' => config.show_tabs = true,
                    'e' => {
                        config.show_ends = true;
                        config.show_nonprinting = true;
                    }
                    'h' => {
                        print_help();
                        std::process::exit(0);
                    }
                    _ => {
                        eprintln!("cat: invalid option -- '{}'", flag);
                        eprintln!("Try 'cat --help' for more information.");
                        std::process::exit(1);
                    }
                }
            }
        } else if arg.starts_with("--") {
            eprintln!("cat: unrecognized option '{}'", arg);
            eprintln!("Try 'cat --help' for more information.");
            std::process::exit(1);
        } else {
            files.push(arg.clone());
        }
    }

    Ok((config, files))
}

fn print_help() {
    println!("cat - concatenate and print files");
    println!();
    println!("Usage: cat [OPTION]... [FILE]...");
    println!();
    println!("Options:");
    println!("  -n, --number              number all output lines");
    println!("  -b, --number-nonblank     number nonempty output lines, override -n");
    println!("  -s, --squeeze-blank       suppress repeated empty output lines");
    println!("  -E, --show-ends           display $ at end of each line");
    println!("  -T, --show-tabs           display TAB characters at ^I");
    println!("  -v, --show-nonprinting    display nonprinting characters (except LFD and TAB)");
    println!("  -A, --show-all            equivalent to -vET");
    println!("  -e                        equivalent to -vE");
    println!("  -t                        equivalent to -vT");
    println!("  -h, --help                display this help and exit");
    println!();
    println!("With no FILE, or when FILE is -, read standard input.");
}

/// 处理读取器
fn cat_reader<R: Read>(reader: R, config: &CatConfig) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    let reader = BufReader::new(reader);

    let mut line_num = 0;
    let mut prev_blank = false;

    for line_result in reader.lines() {
        let line = line_result?;
        line_num += 1;

        // 检查是否是空行
        let is_blank = line.is_empty();

        // 压缩空行
        if config.squeeze_blank && is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;

        // 显示行号
        if config.show_line_numbers {
            if config.squeeze_blank && is_blank {
                // -b 模式：空行不编号
                write!(stdout_lock, "      ")?;
            } else {
                write!(stdout_lock, "{:6}\t", line_num)?;
            }
        }

        // 处理行内容：显示非打印字符和制表符
        let mut output = String::new();
        for c in line.chars() {
            match c {
                '\t' if config.show_tabs => output.push_str("^I"),
                c if c.is_control() && c != '\n' && config.show_nonprinting => {
                    // 显示控制字符为 ^X 格式
                    output.push('^');
                    output.push((c as u8 + 64) as char);
                }
                c if c.is_control() && c != '\n' && !config.show_nonprinting => {
                    // 不显示非打印字符时，跳过或原样输出
                    output.push(c);
                }
                c => output.push(c),
            }
        }

        // 显示行尾符
        if config.show_ends {
            writeln!(stdout_lock, "{}$", output)?;
        } else {
            writeln!(stdout_lock, "{}", output)?;
        }
    }

    stdout_lock.flush()?;
    Ok(())
}
