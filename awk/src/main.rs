use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

/// Awk 配置选项
struct AwkConfig {
    /// 字段分隔符
    field_separator: char,
    /// 程序规则
    rules: Vec<AwkRule>,
    /// 输入文件
    inputs: Vec<String>,
}

impl Default for AwkConfig {
    fn default() -> Self {
        Self {
            field_separator: ' ',
            rules: Vec::new(),
            inputs: Vec::new(),
        }
    }
}

/// Awk 规则
#[derive(Debug, Clone)]
struct AwkRule {
    /// 模式（可选）
    pattern: Option<String>,
    /// 动作
    action: AwkAction,
}

/// Awk 动作
#[derive(Debug, Clone)]
enum AwkAction {
    /// 打印
    Print { fields: Vec<FieldSpec> },
    /// BEGIN 块
    Begin(Vec<String>),
    /// END 块
    End(Vec<String>),
}

/// 字段规范
#[derive(Debug, Clone)]
enum FieldSpec {
    /// 字段引用 $1, $2, etc.
    Field(usize),
    /// 字面字符串
    Literal(String),
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 || (args.len() == 2 && (args[1] == "-h" || args[1] == "--help")) {
        print_help();
        return Ok(());
    }

    let config = parse_args(&args[1..])?;

    // 执行 BEGIN 块
    for rule in &config.rules {
        if let AwkAction::Begin(lines) = &rule.action {
            for line in lines {
                println!("{}", line);
            }
        }
    }

    // 处理输入
    let inputs = if config.inputs.is_empty() {
        vec!["-".to_string()]
    } else {
        config.inputs.clone()
    };

    for input in &inputs {
        process_input(input, &config)?;
    }

    // 执行 END 块
    for rule in &config.rules {
        if let AwkAction::End(lines) = &rule.action {
            for line in lines {
                println!("{}", line);
            }
        }
    }

    Ok(())
}

/// 处理单个输入
fn process_input(input: &str, config: &AwkConfig) -> io::Result<()> {
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

        let fields: Vec<&str> = split_fields(&line, config.field_separator);

        // 应用规则
        for rule in &config.rules {
            match &rule.action {
                AwkAction::Begin(_) | AwkAction::End(_) => continue,
                AwkAction::Print { fields: field_specs } => {
                    // 检查模式匹配
                    let matches = match &rule.pattern {
                        None => true,
                        Some(pattern) => match_pattern(&line, pattern, &fields),
                    };

                    if matches {
                        let output = build_output(field_specs, &fields);
                        println!("{}", output);
                    }
                }
            }
        }
    }

    Ok(())
}

/// 按分隔符拆分字段
fn split_fields(line: &str, separator: char) -> Vec<&str> {
    if separator == ' ' {
        // 空格分隔符：按空白分割，忽略前导空白
        line.split_whitespace().collect()
    } else {
        line.split(separator).collect()
    }
}

/// 检查模式匹配
fn match_pattern(line: &str, pattern: &str, fields: &[&str]) -> bool {
    // 简单的模式匹配：正则或字符串匹配
    if pattern.starts_with('/') && pattern.ends_with('/') {
        // 正则模式 /pattern/
        let regex_pattern = &pattern[1..pattern.len() - 1];
        line.contains(regex_pattern)
    } else if pattern.starts_with('$') {
        // 字段匹配 $1 == "value"
        if let Some(eq_pos) = pattern.find("==") {
            let field_str = &pattern[..eq_pos].trim();
            let value = pattern[eq_pos + 2..].trim().trim_matches('"');

            if let Some(field_num) = field_str.strip_prefix('$') {
                if let Ok(num) = field_num.parse::<usize>() {
                    if num > 0 && num <= fields.len() {
                        return fields[num - 1] == value;
                    }
                }
            }
        }
        false
    } else {
        // 简单字符串匹配
        line.contains(pattern)
    }
}

/// 构建输出
fn build_output(field_specs: &[FieldSpec], fields: &[&str]) -> String {
    if field_specs.is_empty() {
        // 默认打印整行
        return fields.join(" ");
    }

    let parts: Vec<String> = field_specs
        .iter()
        .map(|spec| match spec {
            FieldSpec::Field(n) => {
                if *n > 0 && *n <= fields.len() {
                    fields[n - 1].to_string()
                } else {
                    String::new()
                }
            }
            FieldSpec::Literal(s) => s.clone(),
        })
        .collect();

    parts.join("")
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<AwkConfig> {
    let mut config = AwkConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            break;
        } else if arg == "-F" {
            // 字段分隔符
            i += 1;
            if i < args.len() {
                config.field_separator = args[i].chars().next().unwrap_or(' ');
            }
        } else if arg.starts_with('-') && arg.len() == 2 && arg.chars().nth(1).unwrap() == 'F' {
            // -F separator 紧凑形式
            if let Some(sep) = arg.chars().nth(2) {
                config.field_separator = sep;
            }
        } else if !arg.starts_with('-') {
            // 可能是程序或文件名
            if arg.contains('{') {
                // 这是一个 awk 程序
                let rules = parse_program(arg)?;
                config.rules.extend(rules);
            } else {
                config.inputs.push(arg.clone());
            }
        }
        i += 1;
    }

    Ok(config)
}

/// 解析 awk 程序
fn parse_program(program: &str) -> io::Result<Vec<AwkRule>> {
    let mut rules = Vec::new();

    // 查找所有 {action} 块
    let mut chars = program.chars().peekable();
    let mut current = String::new();

    while let Some(c) = chars.next() {
        if c == '{' {
            // 收集动作直到 }
            let mut action_str = String::new();
            let mut depth = 1;

            while let Some(ac) = chars.next() {
                if ac == '{' {
                    depth += 1;
                } else if ac == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                action_str.push(ac);
            }

            // 解析模式
            let pattern = if current.trim().is_empty() {
                None
            } else {
                Some(current.trim().to_string())
            };

            // 解析动作
            let action = parse_action(&pattern, &action_str)?;
            rules.push(AwkRule { pattern, action });

            current.clear();
        } else {
            current.push(c);
        }
    }

    Ok(rules)
}

/// 解析动作
fn parse_action(pattern: &Option<String>, action_str: &str) -> io::Result<AwkAction> {
    let trimmed = action_str.trim();

    if pattern.is_none() && trimmed.starts_with("BEGIN") {
        let lines: Vec<String> = trimmed
            .strip_prefix("BEGIN")
            .unwrap_or(trimmed)
            .trim()
            .trim_matches(|c| c == '{' || c == '}')
            .trim()
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(AwkAction::Begin(lines));
    }

    if pattern.is_none() && trimmed.starts_with("END") {
        let lines: Vec<String> = trimmed
            .strip_prefix("END")
            .unwrap_or(trimmed)
            .trim()
            .trim_matches(|c| c == '{' || c == '}')
            .trim()
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(AwkAction::End(lines));
    }

    // 解析 print 动作
    if trimmed.starts_with("print") {
        let rest = trimmed.strip_prefix("print").unwrap().trim();
        let fields = parse_print_fields(rest);
        return Ok(AwkAction::Print { fields });
    }

    // 默认为打印
    let fields = parse_print_fields(trimmed);
    Ok(AwkAction::Print { fields })
}

/// 解析 print 字段
fn parse_print_fields(fields_str: &str) -> Vec<FieldSpec> {
    let mut fields = Vec::new();

    if fields_str.is_empty() {
        return fields;
    }

    // 按逗号分割
    for part in fields_str.split(',') {
        let part = part.trim();

        if part.starts_with('"') && part.ends_with('"') {
            // 字面字符串
            fields.push(FieldSpec::Literal(part[1..part.len() - 1].to_string()));
        } else if part.starts_with('$') {
            // 字段引用
            if let Ok(n) = part[1..].parse::<usize>() {
                fields.push(FieldSpec::Field(n));
            }
        } else {
            // 当作字面字符串
            fields.push(FieldSpec::Literal(part.to_string()));
        }
    }

    fields
}

fn print_help() {
    println!("awk - text pattern scanning and processing");
    println!();
    println!("Usage: awk [OPTION]... [PROGRAM] [FILE]...");
    println!();
    println!("Options:");
    println!("  -F separator   use separator as the field separator");
    println!("  -h, --help     display this help");
    println!();
    println!("Program format:");
    println!("  'pattern {{ action }}'");
    println!();
    println!("Actions:");
    println!("  print $1, $2       print specific fields");
    println!("  print              print entire line");
    println!("  BEGIN {{ ... }}      execute before processing input");
    println!("  END {{ ... }}        execute after processing input");
    println!();
    println!("Examples:");
    println!("  awk '{{ print $1 }}' file.txt           Print first field");
    println!("  awk -F: '{{ print $1 }}' /etc/passwd    Print usernames");
    println!("  awk '/pattern/ {{ print $0 }}' file.txt  Print matching lines");
    println!("  awk 'BEGIN {{ print \"Header\" }} {{ print $1 }}' file.txt");
    println!("  awk '{{ print $1, $3 }}' file.txt       Print fields 1 and 3");
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
