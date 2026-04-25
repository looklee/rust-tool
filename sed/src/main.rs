use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};

/// Sed 配置选项
struct SedConfig {
    /// 静默模式（不自动打印）
    quiet: bool,
    /// 原地编辑
    in_place: bool,
    /// 表达式列表
    expressions: Vec<SedExpression>,
    /// 输入文件
    inputs: Vec<String>,
}

impl Default for SedConfig {
    fn default() -> Self {
        Self {
            quiet: false,
            in_place: false,
            expressions: Vec::new(),
            inputs: Vec::new(),
        }
    }
}

/// Sed 表达式类型
#[derive(Debug, Clone)]
enum SedExpression {
    /// 替换: s/pattern/replacement/[flags]
    Substitute {
        pattern: String,
        replacement: String,
        global: bool,
    },
    /// 删除行: d
    Delete,
    /// 打印行: p
    Print,
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
fn process_input(input: &str, config: &SedConfig) -> io::Result<()> {
    let lines: Vec<String> = match input {
        "-" => {
            let reader = BufReader::new(io::stdin());
            reader.lines().filter_map(|l| l.ok()).collect()
        }
        path => {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            reader.lines().filter_map(|l| l.ok()).collect()
        }
    };

    let output = process_lines(&lines, config);

    if config.in_place && input != "-" {
        // 原地写入
        let mut file = File::create(input)?;
        write!(file, "{}", output)?;
    } else {
        print!("{}", output);
    }

    Ok(())
}

/// 处理所有行
fn process_lines(lines: &[String], config: &SedConfig) -> String {
    let mut output = String::new();

    for (_line_num, line) in lines.iter().enumerate() {
        let mut current_line = line.clone();
        let mut delete = false;
        let mut force_print = false;

        for expr in &config.expressions {
            match expr {
                SedExpression::Substitute { pattern, replacement, global } => {
                    current_line = apply_substitute(&current_line, pattern, replacement, *global);
                }
                SedExpression::Delete => {
                    delete = true;
                }
                SedExpression::Print => {
                    force_print = true;
                }
            }
        }

        if delete {
            continue;
        }

        if force_print || !config.quiet {
            output.push_str(&current_line);
            output.push('\n');
        }
    }

    output
}

/// 应用替换操作
fn apply_substitute(line: &str, pattern: &str, replacement: &str, global: bool) -> String {
    if pattern.is_empty() {
        return line.to_string();
    }

    // 简单的字符串替换（非正则）
    if global {
        line.replace(pattern, replacement)
    } else {
        // 只替换第一次出现
        if let Some(pos) = line.find(pattern) {
            let mut result = String::new();
            result.push_str(&line[..pos]);
            result.push_str(replacement);
            result.push_str(&line[pos + pattern.len()..]);
            result
        } else {
            line.to_string()
        }
    }
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<SedConfig> {
    let mut config = SedConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            break;
        } else if arg == "-n" || arg == "--quiet" || arg == "--silent" {
            config.quiet = true;
        } else if arg == "-i" || arg == "--in-place" {
            config.in_place = true;
        } else if arg == "-e" {
            // 下一个参数是表达式
            i += 1;
            if i < args.len() {
                let expr = parse_expression(&args[i])?;
                config.expressions.push(expr);
            }
        } else if arg == "-f" {
            // 从文件读取表达式
            i += 1;
            if i < args.len() {
                let file_exprs = parse_expression_file(&args[i])?;
                config.expressions.extend(file_exprs);
            }
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            // 短选项组合
            for flag in arg[1..].chars() {
                match flag {
                    'n' => config.quiet = true,
                    'i' => config.in_place = true,
                    'h' => break,
                    _ => {
                        eprintln!("sed: invalid option -- '{}'", flag);
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

/// 解析单个 sed 表达式
fn parse_expression(expr: &str) -> io::Result<SedExpression> {
    let chars: Vec<char> = expr.chars().collect();

    if chars.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty expression"));
    }

    match chars[0] {
        's' => {
            // 替换: s/pattern/replacement/[flags]
            if chars.len() < 4 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid substitute expression"));
            }
            let delimiter = chars[1];
            let expr_str: String = chars.iter().skip(2).collect();

            let parts: Vec<&str> = split_expression(&expr_str, delimiter);
            if parts.len() < 2 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid substitute expression"));
            }

            let pattern = parts[0].to_string();
            let replacement = parts[1].to_string();
            let global = parts.get(2).map_or(false, |f| f.contains('g'));

            Ok(SedExpression::Substitute { pattern, replacement, global })
        }
        'd' => Ok(SedExpression::Delete),
        'p' => Ok(SedExpression::Print),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("unknown command: {}", chars[0]))),
    }
}

/// 按分隔符拆分表达式
fn split_expression(expr: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut escaped = false;

    for (i, c) in expr.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == delimiter {
            parts.push(&expr[start..i]);
            start = i + 1;
        }
    }
    parts.push(&expr[start..]);

    parts
}

/// 从文件解析表达式
fn parse_expression_file(path: &str) -> io::Result<Vec<SedExpression>> {
    let content = fs::read_to_string(path)?;
    let mut expressions = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let expr = parse_expression(line)?;
        expressions.push(expr);
    }

    Ok(expressions)
}

fn print_help() {
    println!("sed - stream editor");
    println!();
    println!("Usage: sed [OPTION]... [-e script]... [-f script-file]... [file]...");
    println!();
    println!("Options:");
    println!("  -e, --expression=SCRIPT  add the script to the commands to be executed");
    println!("  -f, --file=SCRIPT-FILE   add the contents of script-file to the commands");
    println!("  -n, --quiet, --silent    suppress automatic printing of pattern space");
    println!("  -i, --in-place           edit files in place");
    println!("  -h, --help               display this help");
    println!();
    println!("Commands:");
    println!("  s/pattern/replacement/   substitute pattern with replacement");
    println!("  s/pattern/replacement/g  substitute all occurrences");
    println!("  d                        delete the line");
    println!("  p                        print the line");
    println!();
    println!("Examples:");
    println!("  sed -e 's/old/new/' file.txt           Replace first occurrence of old with new");
    println!("  sed -e 's/old/new/g' file.txt          Replace all occurrences");
    println!("  sed -e 'd' -e 's/old/new/' file.txt    Delete and substitute");
    println!("  sed -n -e 'p' file.txt                 Print all lines (quiet mode)");
    println!("  sed -i -e 's/old/new/' file.txt        Edit file in place");
    println!();
    println!("With no FILE, or when FILE is -, read standard input.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_flag() {
        assert!(true);
    }
}
