use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

/// Fmt 配置选项
struct FmtConfig {
    /// 输出宽度
    width: usize,
    /// 皇冠边距模式
    crown_margin: bool,
    /// 输入文件
    inputs: Vec<String>,
}

impl Default for FmtConfig {
    fn default() -> Self {
        Self {
            width: 72,
            crown_margin: false,
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
fn process_input(input: &str, config: &FmtConfig) -> io::Result<()> {
    let reader: Box<dyn BufRead> = match input {
        "-" => Box::new(BufReader::new(io::stdin())),
        path => {
            let file = File::open(path)?;
            Box::new(BufReader::new(file))
        }
    };

    let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

    if config.crown_margin {
        process_crown_margin(&lines, config);
    } else {
        process_simple(&lines, config);
    }

    Ok(())
}

/// 简单文本重排
fn process_simple(lines: &[String], config: &FmtConfig) {
    let mut paragraph: Vec<String> = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            // 空行：输出当前段落并开始新段落
            if !paragraph.is_empty() {
                format_paragraph(&paragraph, config);
                println!();
                paragraph.clear();
            } else {
                println!();
            }
        } else {
            paragraph.push(trimmed.to_string());
        }
    }

    // 输出最后一个段落
    if !paragraph.is_empty() {
        format_paragraph(&paragraph, config);
    }
}

/// 皇冠边距模式
fn process_crown_margin(lines: &[String], config: &FmtConfig) {
    if lines.is_empty() {
        return;
    }

    // 检测皇冠边距（前两行的缩进）
    let mut first_indent = 0;
    let mut second_indent = 0;

    if let Some(first) = lines.first() {
        first_indent = first.chars().take_while(|&c| c == ' ').count();
    }
    if lines.len() > 1 {
        second_indent = lines[1].chars().take_while(|&c| c == ' ').count();
    }

    let mut paragraph: Vec<String> = Vec::new();
    let mut line_num = 0;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                format_crown_paragraph(&paragraph, first_indent, second_indent, config);
                println!();
                paragraph.clear();
                line_num = 0;
            } else {
                println!();
            }
        } else {
            paragraph.push(trimmed.to_string());
            line_num += 1;
        }
    }

    if !paragraph.is_empty() {
        format_crown_paragraph(&paragraph, first_indent, second_indent, config);
    }
}

/// 格式化段落
fn format_paragraph(paragraph: &[String], config: &FmtConfig) {
    // 合并所有行
    let text: String = paragraph.join(" ");
    let words: Vec<&str> = text.split_whitespace().collect();

    let mut current_line = String::new();

    for word in words {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if test_line.len() <= config.width {
            current_line = test_line;
        } else {
            // 输出当前行
            println!("{}", current_line);

            // 如果单个单词超过宽度，强制换行
            if word.len() > config.width {
                println!("{}", word);
                current_line = String::new();
            } else {
                current_line = word.to_string();
            }
        }
    }

    // 输出最后一行
    if !current_line.is_empty() {
        println!("{}", current_line);
    }
}

/// 格式化皇冠边距段落
fn format_crown_paragraph(
    paragraph: &[String],
    first_indent: usize,
    second_indent: usize,
    config: &FmtConfig,
) {
    if paragraph.is_empty() {
        return;
    }

    // 合并所有行
    let text: String = paragraph.join(" ");
    let words: Vec<&str> = text.split_whitespace().collect();

    let mut current_line = String::new();
    let mut line_num = 0;

    for word in words {
        let indent = if line_num == 0 {
            first_indent
        } else if line_num == 1 {
            second_indent
        } else {
            0
        };

        let available_width = config.width.saturating_sub(indent);
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if test_line.len() <= available_width {
            current_line = test_line;
        } else {
            // 输出当前行（带缩进）
            let spaces: String = " ".repeat(indent);
            println!("{}{}", spaces, current_line);

            if word.len() > available_width {
                println!("{}", word);
                current_line = String::new();
                line_num += 1;
            } else {
                current_line = word.to_string();
                line_num += 1;
            }
        }
    }

    // 输出最后一行
    if !current_line.is_empty() {
        let indent = if line_num == 0 {
            first_indent
        } else if line_num == 1 {
            second_indent
        } else {
            0
        };
        let spaces: String = " ".repeat(indent);
        println!("{}{}", spaces, current_line);
    }
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<FmtConfig> {
    let mut config = FmtConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            break;
        } else if arg == "-w" || arg == "--width" {
            i += 1;
            if i < args.len() {
                config.width = args[i].parse::<usize>().unwrap_or(72);
            }
        } else if arg == "-c" || arg == "--crown-margin" {
            config.crown_margin = true;
        } else if arg.starts_with("-w") && arg.len() > 2 {
            // -w72 紧凑形式
            config.width = arg[2..].parse::<usize>().unwrap_or(72);
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            // 短选项组合
            for flag in arg[1..].chars() {
                match flag {
                    'c' => config.crown_margin = true,
                    'h' => break,
                    _ => {
                        eprintln!("fmt: invalid option -- '{}'", flag);
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
    println!("fmt - simple text formatter");
    println!();
    println!("Usage: fmt [OPTION]... [FILE]...");
    println!();
    println!("Options:");
    println!("  -w, --width=WIDTH   maximum line width (default: 72)");
    println!("  -c, --crown-margin  preserve first two lines' indentation");
    println!("  -h, --help          display this help");
    println!();
    println!("Description:");
    println!("  Reformats each paragraph in the input file so that each line");
    println!("  is at most WIDTH characters long. Empty lines separate paragraphs.");
    println!();
    println!("Examples:");
    println!("  fmt file.txt               Format with default width (72)");
    println!("  fmt -w 80 file.txt         Format with width 80");
    println!("  fmt -c file.txt            Crown margin mode");
    println!("  fmt -w 40 file.txt > out   Format and save to file");
    println!();
    println!("With no FILE, or when FILE is -, read standard input.");
}
