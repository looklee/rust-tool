use std::env;
use std::io::{self, BufRead, BufReader};

/// Col 配置选项
struct ColConfig {
    /// 不输出反向换行
    no_backspaces: bool,
    /// 灵活模式（允许前向换行）
    flexible: bool,
    /// 输入文件
    inputs: Vec<String>,
}

impl Default for ColConfig {
    fn default() -> Self {
        Self {
            no_backspaces: false,
            flexible: false,
            inputs: Vec::new(),
        }
    }
}

/// 输出行缓冲区
struct LineBuffer {
    lines: Vec<Vec<char>>,
    current_line: usize,
    max_line: usize,
}

impl LineBuffer {
    fn new() -> Self {
        Self {
            lines: vec![Vec::new()],
            current_line: 0,
            max_line: 0,
        }
    }

    fn ensure_line(&mut self, line_num: usize) {
        while self.lines.len() <= line_num {
            self.lines.push(Vec::new());
        }
        if line_num > self.max_line {
            self.max_line = line_num;
        }
    }

    fn add_char(&mut self, c: char) {
        self.ensure_line(self.current_line);
        self.lines[self.current_line].push(c);
    }

    fn add_char_at(&mut self, line_num: usize, col: usize, c: char) {
        self.ensure_line(line_num);
        let line = &mut self.lines[line_num];
        if col < line.len() {
            line[col] = c;
        } else {
            while line.len() < col {
                line.push(' ');
            }
            line.push(c);
        }
    }

    fn output(&self, no_backspaces: bool) {
        for line in &self.lines {
            let output = if no_backspaces {
                // 移除退格字符及其前一个字符
                remove_backspaces(line)
            } else {
                line.iter().collect::<String>()
            };
            println!("{}", output);
        }
    }
}

/// 移除退格字符
fn remove_backspaces(chars: &[char]) -> String {
    let mut result = Vec::new();

    for &c in chars {
        if c == '\x08' {
            // 退格：移除前一个字符
            if !result.is_empty() {
                result.pop();
            }
        } else {
            result.push(c);
        }
    }

    result.into_iter().collect()
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
fn process_input(input: &str, config: &ColConfig) -> io::Result<()> {
    let reader: Box<dyn BufRead> = match input {
        "-" => Box::new(BufReader::new(io::stdin())),
        path => {
            let file = std::fs::File::open(path)?;
            Box::new(BufReader::new(file))
        }
    };

    let mut buffer = LineBuffer::new();
    let mut col = 0;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        for c in line.chars() {
            process_char(c, &mut buffer, &mut col, config);
        }

        // 换行
        buffer.ensure_line(buffer.current_line + 1);
        buffer.current_line += 1;
        col = 0;
    }

    // 输出结果
    buffer.output(config.no_backspaces);

    Ok(())
}

/// 处理单个字符
fn process_char(c: char, buffer: &mut LineBuffer, col: &mut usize, _config: &ColConfig) {
    match c {
        '\x08' => {
            // 退格
            if *col > 0 {
                *col -= 1;
            }
        }
        '\x0b' | '\x1b' => {
            // 垂直制表符或 ESC（反向换行）
            if buffer.current_line > 0 {
                buffer.current_line -= 1;
                *col = 0;
            }
        }
        '\x0c' => {
            // 换页符
            buffer.ensure_line(buffer.current_line + 1);
            buffer.current_line += 1;
            *col = 0;
        }
        '\x07' => {
            // BEL - 忽略
        }
        '\t' => {
            // 制表符：前进到下一个制表位（每8列）
            let tab_stop = 8;
            let next_tab = (*col / tab_stop + 1) * tab_stop;
            while *col < next_tab {
                buffer.add_char(' ');
                *col += 1;
            }
        }
        '\r' => {
            // 回车：回到行首
            *col = 0;
        }
        _ => {
            buffer.add_char_at(buffer.current_line, *col, c);
            *col += 1;
        }
    }
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<ColConfig> {
    let mut config = ColConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            break;
        } else if arg == "-b" || arg == "--no-backspaces" {
            config.no_backspaces = true;
        } else if arg == "-f" || arg == "--flexible" {
            config.flexible = true;
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            // 短选项组合
            for flag in arg[1..].chars() {
                match flag {
                    'b' => config.no_backspaces = true,
                    'f' => config.flexible = true,
                    'h' => break,
                    _ => {
                        eprintln!("col: invalid option -- '{}'", flag);
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
    println!("col - filter reverse line feeds");
    println!();
    println!("Usage: col [OPTION]... [FILE]...");
    println!();
    println!("Options:");
    println!("  -b, --no-backspaces    do not output backspaces");
    println!("  -f, --flexible         allow forward line feeds");
    println!("  -h, --help             display this help");
    println!();
    println!("Description:");
    println!("  col filters reverse line feeds so that the output is in the correct");
    println!("  order, with only forward and half-forward line feeds.");
    println!();
    println!("  It tracks the column position of each character and handles:");
    println!("  - Backspaces (move column back one)");
    println!("  - Reverse line feeds (move to previous line)");
    println!("  - Tabs (advance to next tab stop)");
    println!("  - Carriage returns (return to start of line)");
    println!();
    println!("Examples:");
    println!("  col < file.txt              Filter reverse line feeds");
    println!("  col -b < file.txt           Remove backspaces from output");
    println!("  man ls | col -b | less       Clean man page output");
    println!();
    println!("With no FILE, or when FILE is -, read standard input.");
}
