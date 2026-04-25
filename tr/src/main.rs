use std::env;
use std::io::{self, BufRead, BufReader, Write};

/// Tr 配置选项
struct TrConfig {
    /// 删除字符
    delete: bool,
    /// 挤压重复字符
    squeeze: bool,
    /// 字符集1（源）
    set1: Vec<char>,
    /// 字符集2（目标）
    set2: Vec<char>,
    /// 输入文件
    inputs: Vec<String>,
}

impl Default for TrConfig {
    fn default() -> Self {
        Self {
            delete: false,
            squeeze: false,
            set1: Vec::new(),
            set2: Vec::new(),
            inputs: Vec::new(),
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 || (args.len() == 2 && (args[1] == "-h" || args[1] == "--help")) {
        print_help();
        return Ok(());
    }

    let config = parse_args(&args[1..])?;

    // 处理输入
    let inputs = if config.inputs.is_empty() {
        vec!["-".to_string()]
    } else {
        config.inputs.clone()
    };

    for input in &inputs {
        process_input(input, &config)?;
    }

    Ok(())
}

/// 处理单个输入
fn process_input(input: &str, config: &TrConfig) -> io::Result<()> {
    let reader: Box<dyn BufRead> = match input {
        "-" => Box::new(BufReader::new(io::stdin())),
        path => {
            let file = std::fs::File::open(path)?;
            Box::new(BufReader::new(file))
        }
    };

    let mut output = Vec::new();
    let mut prev_char: Option<char> = None;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        for c in line.chars() {
            let result = process_char(c, config, &prev_char);

            match result {
                CharResult::Skip => continue,
                CharResult::Output(ch) => {
                    // 检查挤压
                    if config.squeeze {
                        if let Some(prev) = prev_char {
                            if prev == ch && config.set1.contains(&prev) {
                                continue;
                            }
                        }
                    }
                    output.push(ch);
                    prev_char = Some(ch);
                }
            }
        }
        // 保留换行符
        output.push('\n');
    }

    // 输出结果
    let output_str: String = output.into_iter().collect();
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    write!(handle, "{}", output_str)?;

    Ok(())
}

/// 字符处理结果
enum CharResult {
    Skip,
    Output(char),
}

/// 处理单个字符
fn process_char(c: char, config: &TrConfig, prev_char: &Option<char>) -> CharResult {
    if config.delete {
        // 删除模式：如果字符在 set1 中，则删除
        if config.set1.contains(&c) {
            return CharResult::Skip;
        }
        CharResult::Output(c)
    } else if !config.set1.is_empty() && !config.set2.is_empty() {
        // 翻译模式
        if let Some(pos) = config.set1.iter().position(|&x| x == c) {
            let replacement = config.set2[pos % config.set2.len()];
            if config.squeeze {
                if let Some(prev) = prev_char {
                    if *prev == replacement && config.set1.contains(prev) {
                        return CharResult::Skip;
                    }
                }
            }
            CharResult::Output(replacement)
        } else {
            CharResult::Output(c)
        }
    } else {
        CharResult::Output(c)
    }
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<TrConfig> {
    let mut config = TrConfig::default();
    let mut i = 0;
    let mut set_index = 0; // 0=set1, 1=set2

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            break;
        } else if arg == "-d" || arg == "--delete" {
            config.delete = true;
        } else if arg == "-s" || arg == "--squeeze-repeats" {
            config.squeeze = true;
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            // 短选项组合
            for flag in arg[1..].chars() {
                match flag {
                    'd' => config.delete = true,
                    's' => config.squeeze = true,
                    'h' => break,
                    _ => {
                        eprintln!("tr: invalid option -- '{}'", flag);
                        std::process::exit(1);
                    }
                }
            }
        } else {
            // 字符集参数
            if set_index == 0 {
                config.set1 = parse_charset(arg);
                set_index = 1;
            } else {
                config.set2 = parse_charset(arg);
                set_index = 2;
            }
        }
        i += 1;
    }

    Ok(config)
}

/// 解析字符集
fn parse_charset(set: &str) -> Vec<char> {
    let mut chars = Vec::new();
    let chars_vec: Vec<char> = set.chars().collect();
    let mut i = 0;

    while i < chars_vec.len() {
        // 检查范围表示法 a-z
        if i + 2 < chars_vec.len() && chars_vec[i + 1] == '-' {
            let start = chars_vec[i];
            let end = chars_vec[i + 2];

            // 处理预定义字符类
            let (actual_start, actual_end) = match (start, end) {
                ('a', 'z') => ('a', 'z'),
                ('A', 'Z') => ('A', 'Z'),
                ('0', '9') => ('0', '9'),
                _ => (start, end),
            };

            let mut c = actual_start;
            while c <= actual_end {
                chars.push(c);
                if let Some(next) = char::from_u32((c as u32) + 1) {
                    c = next;
                } else {
                    break;
                }
            }
            i += 3;
        } else {
            // 处理转义序列
            let c = chars_vec[i];
            if c == '\\' && i + 1 < chars_vec.len() {
                let next = chars_vec[i + 1];
                let escaped = match next {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '0'..='7' => {
                        // 八进制转义
                        let mut octal = String::new();
                        octal.push(next);
                        let mut j = i + 2;
                        while j < chars_vec.len() && octal.len() < 3 && chars_vec[j].is_ascii_digit() {
                            octal.push(chars_vec[j]);
                            j += 1;
                        }
                        if let Ok(val) = u8::from_str_radix(&octal, 8) {
                            chars.push(val as char);
                        }
                        i = j;
                        continue;
                    }
                    _ => next,
                };
                chars.push(escaped);
                i += 2;
            } else {
                chars.push(c);
                i += 1;
            }
        }
    }

    chars
}

fn print_help() {
    println!("tr - translate or delete characters");
    println!();
    println!("Usage: tr [OPTION]... SET1 [SET2]");
    println!();
    println!("Options:");
    println!("  -d, --delete              delete characters in SET1");
    println!("  -s, --squeeze-repeats     replace repeated characters with a single instance");
    println!("  -h, --help                display this help");
    println!();
    println!("Character sets:");
    println!("  a-z    all lowercase letters");
    println!("  A-Z    all uppercase letters");
    println!("  0-9    all digits");
    println!("  \\n     newline");
    println!("  \\t     tab");
    println!("  \\\\     backslash");
    println!();
    println!("Examples:");
    println!("  tr a-z A-Z < file.txt           Convert to uppercase");
    println!("  tr -d '0-9' < file.txt           Delete all digits");
    println!("  tr -s ' ' < file.txt             Squeeze repeated spaces");
    println!("  tr ' ' '\\n' < file.txt           Replace spaces with newlines");
    println!("  tr -d '\\r' < file.txt            Delete carriage returns");
    println!();
    println!("With no input file, or when input is -, read standard input.");
}
