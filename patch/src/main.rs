use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

/// 配置选项
struct PatchConfig {
    /// 输入补丁文件
    patch_file: Option<String>,
    /// 目标文件
    target_file: Option<String>,
    /// 反向应用补丁
    reverse: bool,
    /// 交互式确认
    interactive: bool,
    /// 备份原文件
    backup: bool,
}

impl Default for PatchConfig {
    fn default() -> Self {
        Self {
            patch_file: None,
            target_file: None,
            reverse: false,
            interactive: false,
            backup: true,
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 || args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }

    let config = parse_args(&args[1..])?;

    if let Some(patch_file) = &config.patch_file {
        apply_patch(patch_file, &config)?;
    } else {
        // 从 stdin 读取补丁
        let stdin = io::stdin();
        apply_patch_from_reader(stdin.lock(), &config)?;
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> io::Result<PatchConfig> {
    let mut config = PatchConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            print_help();
            std::process::exit(0);
        } else if arg == "-R" || arg == "--reverse" {
            config.reverse = true;
        } else if arg == "-i" || arg == "--interactive" {
            config.interactive = true;
        } else if arg == "--no-backup" {
            config.backup = false;
        } else if arg == "-p" {
            // 忽略 -p 参数（兼容 git diff 格式）
        } else if !arg.starts_with('-') {
            if config.patch_file.is_none() {
                config.patch_file = Some(arg.clone());
            } else if config.target_file.is_none() {
                config.target_file = Some(arg.clone());
            }
        }
        i += 1;
    }

    Ok(config)
}

fn print_help() {
    println!("patch - apply diff to files");
    println!();
    println!("Usage: patch [OPTIONS] [PATCHFILE]");
    println!();
    println!("Options:");
    println!("  -R, --reverse         apply patch in reverse");
    println!("  -i, --interactive     ask for confirmation before applying");
    println!("  --no-backup           don't create backup files");
    println!("  -h, --help            show this help");
    println!();
    println!("With no PATCHFILE, read from standard input.");
}

/// 从文件应用补丁
fn apply_patch(patch_file: &str, config: &PatchConfig) -> io::Result<()> {
    let file = File::open(patch_file)?;
    let reader = BufReader::new(file);
    apply_patch_from_reader(reader, config)
}

/// 从读取器应用补丁
fn apply_patch_from_reader<R: BufRead>(reader: R, config: &PatchConfig) -> io::Result<()> {
    let mut current_file: Option<String> = None;
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut in_hunk = false;

    for line_result in reader.lines() {
        let line = line_result?;

        // 检测文件头 (diff --git a/file b/file 或 --- a/file)
        if line.starts_with("diff --git ") {
            // 处理之前的 hunk
            if let Some(file) = current_file.take() {
                if !hunks.is_empty() {
                    apply_hunks_to_file(&file, &hunks, config)?;
                }
                hunks.clear();
            }

            // 解析新文件名
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let file_b = parts[3].trim_start_matches("b/");
                current_file = Some(file_b.to_string());
            }
            in_hunk = false;
        } else if line.starts_with("--- ") {
            // 解析旧文件名 (可能包含 a/ 前缀)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let file_a = parts[1].trim_start_matches("a/");
                if current_file.is_none() {
                    // 如果没有从 diff --git 获取文件名，尝试从 --- 获取
                    current_file = Some(file_a.to_string());
                }
            }
            in_hunk = false;
        } else if line.starts_with("+++ ") {
            // 解析新文件名
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let file_b = parts[1].trim_start_matches("b/");
                current_file = Some(file_b.to_string());
            }
            in_hunk = false;
        } else if line.starts_with("@@ ") {
            // 解析 hunk 头
            if let Some(hunk) = parse_hunk_header(&line) {
                hunks.push(hunk);
                in_hunk = true;
            }
        } else if in_hunk {
            // hunk 内容
            if let Some(hunk) = hunks.last_mut() {
                hunk.lines.push(line);
            }
        }
    }

    // 处理最后一个 hunk
    if let Some(file) = current_file.take() {
        if !hunks.is_empty() {
            apply_hunks_to_file(&file, &hunks, config)?;
        }
    }

    Ok(())
}

/// 解析 hunk 头 (@@ -old_start,old_count +new_start,new_count @@)
fn parse_hunk_header(line: &str) -> Option<Hunk> {
    let parts: Vec<&str> = line.split("@@").collect();
    if parts.len() < 2 {
        return None;
    }

    let header = parts[1].trim();
    let nums: Vec<&str> = header.split_whitespace().next()?.split(',').collect();

    let old_start: usize = nums.first()?.trim_start_matches('-').parse().ok()?;
    let old_count = nums.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);

    let nums: Vec<&str> = header.split_whitespace().nth(1)?.split(',').collect();
    let new_start: usize = nums.first()?.trim_start_matches('-').parse().ok()?;
    let new_count = nums.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);

    Some(Hunk {
        old_start,
        old_count,
        new_start,
        new_count,
        lines: Vec::new(),
    })
}

#[derive(Debug)]
struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    lines: Vec<String>,
}

/// 应用 hunks 到文件
fn apply_hunks_to_file(file_path: &str, hunks: &[Hunk], config: &PatchConfig) -> io::Result<()> {
    println!("📄 Processing: {}", file_path);

    // 读取原文件
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("⚠️  Cannot read {}: {}", file_path, e);
            return Ok(());
        }
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines: Vec<String> = Vec::new();
    let mut old_line_num = 1;
    let mut hunk_idx = 0;

    while old_line_num <= lines.len() || hunk_idx < hunks.len() {
        // 检查是否有 hunk 覆盖当前行
        if hunk_idx < hunks.len() {
            let hunk = &hunks[hunk_idx];

            if old_line_num >= hunk.old_start {
                // 应用这个 hunk
                for line in &hunk.lines {
                    if line.starts_with('-') || line.starts_with(' ') {
                        if config.reverse && line.starts_with('-') {
                            // 反向：- 变成 +
                            result_lines.push(line[1..].to_string());
                        } else if !config.reverse && line.starts_with(' ') {
                            result_lines.push(line[1..].to_string());
                        }
                    }
                    if line.starts_with('+') || line.starts_with(' ') {
                        if config.reverse && line.starts_with('+') {
                            // 反向：+ 变成 -
                            // 不添加到结果
                        } else if !config.reverse && line.starts_with('+') {
                            result_lines.push(line[1..].to_string());
                        }
                    }
                }
                old_line_num = hunk.old_start + hunk.old_count;
                hunk_idx += 1;
                continue;
            }
        }

        // 复制未修改的行
        if old_line_num <= lines.len() {
            result_lines.push(lines[old_line_num - 1].to_string());
            old_line_num += 1;
        } else {
            break;
        }
    }

    // 显示差异
    println!("  Changes:");
    let old_count = lines.len();
    let new_count = result_lines.len();
    println!("    {} -> {} lines", old_count, new_count);

    // 确认
    if config.interactive {
        print!("  Apply changes? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            println!("  Skipped");
            return Ok(());
        }
    }

    // 备份
    if config.backup {
        let backup_path = format!("{}.bak", file_path);
        fs::copy(file_path, &backup_path)?;
        println!("  Backup: {}", backup_path);
    }

    // 写入新内容
    let mut output = String::new();
    for (i, line) in result_lines.iter().enumerate() {
        output.push_str(line);
        if i < result_lines.len() - 1 || content.ends_with('\n') {
            output.push('\n');
        }
    }

    fs::write(file_path, output.as_bytes())?;
    println!("  ✅ Applied");

    Ok(())
}
