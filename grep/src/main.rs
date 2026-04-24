use regex::Regex;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

/// ANSI 颜色代码
struct Colors {
    reset: &'static str,
    red: &'static str,
    green: &'static str,
    yellow: &'static str,
    cyan: &'static str,
}

impl Colors {
    fn new(enabled: bool) -> Self {
        if enabled {
            Self {
                reset: "\x1b[0m",
                red: "\x1b[31m",
                green: "\x1b[32m",
                yellow: "\x1b[33m",
                cyan: "\x1b[36m",
            }
        } else {
            Self {
                reset: "",
                red: "",
                green: "",
                yellow: "",
                cyan: "",
            }
        }
    }
}

/// 配置选项
struct GrepConfig {
    /// 忽略大小写
    ignore_case: bool,
    /// 反转匹配（显示不匹配的行）
    invert_match: bool,
    /// 递归搜索
    recursive: bool,
    /// 显示行号
    line_number: bool,
    /// 显示文件名
    with_filename: bool,
    /// 只显示匹配的文件
    files_with_matches: bool,
    /// 显示匹配计数
    count: bool,
    /// 彩色输出
    color: bool,
    /// 只显示匹配的部分
    only_matching: bool,
    /// 上下文行数
    context: usize,
    /// 之前行数
    before_context: usize,
    /// 之后行数
    after_context: usize,
    /// 正则表达式
    pattern: String,
    /// 文件模式（glob）
    glob_pattern: Option<String>,
}

impl Default for GrepConfig {
    fn default() -> Self {
        Self {
            ignore_case: false,
            invert_match: false,
            recursive: false,
            line_number: false,
            with_filename: false,
            files_with_matches: false,
            count: false,
            color: false,
            only_matching: false,
            context: 0,
            before_context: 0,
            after_context: 0,
            pattern: String::new(),
            glob_pattern: None,
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let (config, files) = parse_args(&args[1..])?;

    if config.pattern.is_empty() {
        eprintln!("grep: no pattern provided");
        eprintln!("Usage: grep [OPTIONS] PATTERN [FILE]...");
        std::process::exit(1);
    }

    // 构建正则表达式
    let re_result = if config.ignore_case {
        Regex::new(&format!("(?i){}", config.pattern))
    } else {
        Regex::new(&config.pattern)
    };

    let re = match re_result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("grep: invalid regex pattern: {}", e);
            std::process::exit(1);
        }
    };

    // 获取文件列表
    let search_files = if files.is_empty() {
        Vec::new()
    } else if config.recursive {
        let mut all_files = Vec::new();
        for file_path in &files {
            let path = Path::new(file_path);
            if path.is_dir() {
                collect_files_recursive(path, &mut all_files, &config)?;
            } else {
                all_files.push(file_path.clone());
            }
        }
        all_files
    } else {
        files.clone()
    };

    if search_files.is_empty() {
        // 没有文件，从 stdin 读取
        let stdin = io::stdin();
        grep_reader(stdin.lock(), &re, "<stdin>", &config)?;
    } else {
        let mut total_matches = 0u64;

        for file_path in &search_files {
            match grep_file(file_path, &re, &config) {
                Ok(count) => total_matches += count,
                Err(e) => {
                    eprintln!("grep: {}: {}", file_path, e);
                }
            }
        }

        if config.count {
            if search_files.len() > 1 {
                println!("total:{}", total_matches);
            }
        }
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<(GrepConfig, Vec<String>)> {
    let mut config = GrepConfig::default();
    let mut pattern: Option<String> = None;
    let mut files: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if arg == "--help" || arg == "-h" {
            print_help();
            std::process::exit(0);
        } else if arg == "-i" || arg == "--ignore-case" {
            config.ignore_case = true;
        } else if arg == "-v" || arg == "--invert-match" {
            config.invert_match = true;
        } else if arg == "-r" || arg == "-R" || arg == "--recursive" {
            config.recursive = true;
        } else if arg == "-n" || arg == "--line-number" {
            config.line_number = true;
        } else if arg == "-H" || arg == "--with-filename" {
            config.with_filename = true;
        } else if arg == "-l" || arg == "--files-with-matches" {
            config.files_with_matches = true;
        } else if arg == "-c" || arg == "--count" {
            config.count = true;
        } else if arg == "--color" || arg == "--colour" {
            config.color = true;
        } else if arg == "--no-color" || arg == "--no-colour" {
            config.color = false;
        } else if arg == "-o" || arg == "--only-matching" {
            config.only_matching = true;
        } else if arg == "-C" || arg == "--context" {
            if i + 1 < args.len() {
                i += 1;
                config.context = args[i].parse().unwrap_or(0);
                config.before_context = config.context;
                config.after_context = config.context;
            }
        } else if arg == "-B" || arg == "--before-context" {
            if i + 1 < args.len() {
                i += 1;
                config.before_context = args[i].parse().unwrap_or(0);
            }
        } else if arg == "-A" || arg == "--after-context" {
            if i + 1 < args.len() {
                i += 1;
                config.after_context = args[i].parse().unwrap_or(0);
            }
        } else if arg == "-e" || arg == "--regexp" {
            if i + 1 < args.len() {
                i += 1;
                pattern = Some(args[i].clone());
            }
        } else if arg == "-g" || arg == "--glob" {
            if i + 1 < args.len() {
                i += 1;
                config.glob_pattern = Some(args[i].clone());
            }
        } else if arg == "-E" || arg == "--extended-regex" {
            // 默认支持扩展正则，此选项保留兼容性
        } else if arg.starts_with("--color=") || arg.starts_with("--colour=") {
            let value = arg.split('=').nth(1).unwrap_or("always");
            config.color = match value {
                "always" => true,
                "never" => false,
                "auto" => atty::is(atty::Stream::Stdout),
                _ => true,
            };
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            // 短选项组合
            let mut j = 1;
            while j < arg.len() {
                let flag = arg.chars().nth(j).unwrap();
                match flag {
                    'i' => config.ignore_case = true,
                    'v' => config.invert_match = true,
                    'r' | 'R' => config.recursive = true,
                    'n' => config.line_number = true,
                    'H' => config.with_filename = true,
                    'l' => config.files_with_matches = true,
                    'c' => config.count = true,
                    'o' => config.only_matching = true,
                    'e' => {
                        // 下一个字符是模式
                        if j + 1 < arg.len() {
                            pattern = Some(arg[j + 1..].to_string());
                            break;
                        } else if i + 1 < args.len() {
                            i += 1;
                            pattern = Some(args[i].clone());
                            break;
                        }
                    }
                    'h' => {
                        print_help();
                        std::process::exit(0);
                    }
                    _ => {
                        eprintln!("grep: invalid option -- '{}'", flag);
                        eprintln!("Try 'grep --help' for more information.");
                        std::process::exit(1);
                    }
                }
                j += 1;
            }
        } else if arg.starts_with("--") {
            eprintln!("grep: unrecognized option '{}'", arg);
            eprintln!("Try 'grep --help' for more information.");
            std::process::exit(1);
        } else {
            if pattern.is_none() {
                pattern = Some(arg.clone());
            } else {
                files.push(arg.clone());
            }
        }
        i += 1;
    }

    config.pattern = pattern.unwrap_or_default();

    // 如果未指定 --color，检查是否输出到终端
    if !config.color {
        config.color = atty::is(atty::Stream::Stdout);
    }

    // 多个文件时自动显示文件名
    if files.len() > 1 {
        config.with_filename = true;
    }

    Ok((config, files))
}

fn print_help() {
    println!("grep - search for patterns in files");
    println!();
    println!("Usage: grep [OPTIONS] PATTERN [FILE]...");
    println!();
    println!("Options:");
    println!("  -i, --ignore-case       ignore case distinctions");
    println!("  -v, --invert-match      select non-matching lines");
    println!("  -r, -R, --recursive     read all files under each directory");
    println!("  -n, --line-number       print line number with output lines");
    println!("  -H, --with-filename     print the filename for each match");
    println!("  -l, --files-with-matches    print only names of FILEs with matches");
    println!("  -c, --count             print only a count of matching lines");
    println!("  -o, --only-matching     print only the matched part of lines");
    println!("  -C, --context=NUM       print NUM lines of output context");
    println!("  -B, --before-context=NUM    print NUM lines before match");
    println!("  -A, --after-context=NUM     print NUM lines after match");
    println!("  -e, --regexp=PATTERN    use PATTERN for matching");
    println!("  -g, --glob=PATTERN      search only files matching PATTERN");
    println!("  -E, --extended-regex    use extended regular expressions");
    println!("  --color=WHEN            use color output: always, never, auto");
    println!("  -h, --help              display this help and exit");
    println!();
    println!("With no FILE, or when FILE is -, read standard input.");
}

/// 递归收集文件
fn collect_files_recursive(
    dir: &Path,
    files: &mut Vec<String>,
    config: &GrepConfig,
) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry_result in entries {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // 跳过隐藏目录和常见忽略目录
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.')
                || name_str == "target"
                || name_str == "node_modules"
                || name_str == ".git"
                || name_str == "vendor"
            {
                continue;
            }
        }

        // 检查 glob 模式
        if let Some(ref glob) = config.glob_pattern {
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if !glob_match(&name_str, glob) {
                    continue;
                }
            }
        }

        if path.is_dir() {
            collect_files_recursive(&path, files, config)?;
        } else if path.is_file() {
            files.push(path.to_string_lossy().to_string());
        }
    }

    Ok(())
}

/// 简单的 glob 匹配
fn glob_match(text: &str, pattern: &str) -> bool {
    // 简化实现：只支持 * 和 ?
    let regex_pattern = pattern
        .replace('.', "\\.")
        .replace('*', ".*")
        .replace('?', ".");

    let re = match Regex::new(&format!("^{}$", regex_pattern)) {
        Ok(r) => r,
        Err(_) => return false,
    };

    re.is_match(text)
}

/// 处理单个文件
fn grep_file(file_path: &str, re: &Regex, config: &GrepConfig) -> io::Result<u64> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut match_count = 0u64;
    let mut lines: Vec<String> = Vec::new();
    let mut line_numbers: Vec<usize> = Vec::new();
    let mut all_lines: Vec<String> = Vec::new();

    // 读取所有行（用于上下文）
    for (_idx, line_result) in reader.lines().enumerate() {
        if let Ok(line) = line_result {
            all_lines.push(line);
        } else {
            all_lines.push(String::new());
        }
    }

    // 查找匹配
    for (idx, line) in all_lines.iter().enumerate() {
        let is_match = re.is_match(line);
        let should_print = if config.invert_match {
            !is_match
        } else {
            is_match
        };

        if should_print && !config.invert_match {
            match_count += 1;
        } else if config.invert_match && should_print {
            match_count += 1;
        }

        if should_print || config.before_context > 0 || config.after_context > 0 {
            lines.push(line.clone());
            line_numbers.push(idx + 1);
        }
    }

    if config.files_with_matches && match_count > 0 {
        println!("{}", file_path);
        return Ok(1);
    }

    if config.count {
        let display_name = if config.with_filename {
            file_path
        } else {
            ""
        };
        if config.with_filename {
            println!("{}:{}", display_name, match_count);
        } else {
            println!("{}", match_count);
        }
        return Ok(match_count);
    }

    // 输出匹配行（带上下文）
    let colors = Colors::new(config.color);
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let line_num = line_numbers[i];
        let is_match_line = re.is_match(line);

        // 打印之前上下文
        if config.before_context > 0 && is_match_line {
            for j in 1..=config.before_context.min(i) {
                let ctx_line = &lines[i - j];
                let ctx_num = line_numbers[i - j];
                let prefix = if config.with_filename {
                    format!("{}-{}-", colors.cyan, colors.reset)
                } else {
                    format!("{}-", colors.cyan)
                };
                let line_prefix = if config.line_number {
                    format!("{}{:6}-{}", colors.cyan, ctx_num, colors.reset)
                } else {
                    prefix
                };
                let _ = writeln!(stdout_lock, "{}{}", line_prefix, ctx_line);
            }
        }

        // 打印当前行
        if re.is_match(line) != config.invert_match {
            let prefix = if config.with_filename {
                if config.line_number {
                    format!("{}{}:{}{}:{}{}:", colors.green, file_path, colors.yellow, line_num, colors.reset, colors.green)
                } else {
                    format!("{}{}:{}", colors.green, file_path, colors.reset)
                }
            } else if config.line_number {
                format!("{}{:6}:{}", colors.yellow, line_num, colors.reset)
            } else {
                String::new()
            };

            if config.only_matching {
                for mat in re.find_iter(line) {
                    if config.with_filename || config.line_number {
                        let _ = write!(stdout_lock, "{}:", prefix);
                    }
                    let _ = writeln!(stdout_lock, "{}{}{}", colors.red, mat.as_str(), colors.reset);
                }
            } else {
                let highlighted = re.replace_all(line, |caps: &regex::Captures| {
                    format!("{}{}{}", colors.red, &caps[0], colors.reset)
                });
                let _ = writeln!(stdout_lock, "{}{}", prefix, highlighted);
            }
        }

        // 打印之后上下文
        if config.after_context > 0 && is_match_line {
            for j in 1..=config.after_context {
                if i + j < lines.len() {
                    let ctx_line = &lines[i + j];
                    let ctx_num = line_numbers[i + j];
                    let line_prefix = if config.line_number {
                        format!("{}{:6}-{}", colors.cyan, ctx_num, colors.reset)
                    } else {
                        format!("{}-{}", colors.cyan, colors.reset)
                    };
                    let _ = writeln!(stdout_lock, "{}{}", line_prefix, ctx_line);
                }
            }
        }

        i += 1;
    }

    stdout_lock.flush()?;
    Ok(match_count)
}

/// 处理读取器（stdin）
fn grep_reader<R: BufRead>(
    reader: R,
    re: &Regex,
    _filename: &str,
    config: &GrepConfig,
) -> io::Result<u64> {
    let colors = Colors::new(config.color);
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    let mut match_count = 0u64;

    for (idx, line_result) in reader.lines().enumerate() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        let is_match = re.is_match(&line);
        let should_print = if config.invert_match {
            !is_match
        } else {
            is_match
        };

        if should_print {
            if config.invert_match || !config.files_with_matches {
                match_count += 1;
            }

            if config.count {
                continue;
            }

            let prefix = if config.line_number {
                format!("{}{:6}:{}", colors.yellow, idx + 1, colors.reset)
            } else {
                String::new()
            };

            if config.only_matching {
                for mat in re.find_iter(&line) {
                    let _ = writeln!(stdout_lock, "{}{}{}", colors.red, mat.as_str(), colors.reset);
                }
            } else {
                let highlighted = re.replace_all(&line, |caps: &regex::Captures| {
                    format!("{}{}{}", colors.red, &caps[0], colors.reset)
                });
                let _ = writeln!(stdout_lock, "{}{}", prefix, highlighted);
            }
        }
    }

    if config.count {
        println!("{}", match_count);
    }

    stdout_lock.flush()?;
    Ok(match_count)
}
