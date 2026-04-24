use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::fs::File;
use std::env;
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU32, Ordering}};

use crate::builtins;
use crate::script;

/// 中断标志（Ctrl+C）
pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// 当前前台进程 PID
pub static FOREGROUND_PID: AtomicU32 = AtomicU32::new(0);

/// 设置信号处理（占位函数）
pub fn setup_signal_handler() {}

/// 向前台进程发送中断信号
pub fn interrupt_foreground() {
    let pid = FOREGROUND_PID.load(Ordering::SeqCst);
    if pid > 0 {
        // 使用外部 kill 命令发送信号
        let _ = Command::new("kill")
            .arg("-INT")
            .arg(pid.to_string())
            .output();
    }
    INTERRUPTED.store(true, Ordering::SeqCst);
}

/// 作业状态
#[derive(Clone, Copy, PartialEq)]
pub enum JobState {
    Running,
    Stopped,
    Completed,
}

/// 后台作业
pub struct Job {
    pub id: usize,
    pub command: String,
    pub pid: u32,
    pub state: JobState,
}

/// 全局 Shell 状态
pub struct ShellState {
    /// 后台作业列表
    pub jobs: HashMap<usize, Job>,
    /// 别名表
    pub aliases: HashMap<String, String>,
    /// 上一个命令的退出码
    pub last_exit_code: i32,
    /// 下一个作业 ID
    pub next_job_id: usize,
    /// 当前前台作业 ID
    pub foreground_job_id: Option<usize>,
    /// 当前前台进程 PID
    pub foreground_pid: Option<u32>,
}

impl ShellState {
    pub fn new() -> Self {
        ShellState {
            jobs: HashMap::new(),
            aliases: HashMap::new(),
            last_exit_code: 0,
            next_job_id: 1,
            foreground_job_id: None,
            foreground_pid: None,
        }
    }
}

// 使用 thread_local 存储每个线程的 Shell 状态
thread_local! {
    pub static STATE: Arc<Mutex<ShellState>> = Arc::new(Mutex::new(ShellState::new()));
}

/// 执行一行命令
/// 返回 true 表示退出 shell
pub fn execute(input: &str) -> io::Result<bool> {
    // 检查是否是控制流语句
    let trimmed = input.trim();
    if trimmed.starts_with("if ") {
        return script::execute_if(input);
    }
    if trimmed.starts_with("for ") {
        return script::execute_for(input);
    }
    if trimmed.starts_with("while ") {
        return script::execute_while(input);
    }
    
    // 别名展开
    let input = expand_aliases(input);
    
    // ~ 家目录展开
    let input = expand_tilde(&input);
    
    // 变量替换
    let input = expand_variables(&input);
    
    // 命令替换
    let input = expand_commands(&input)?;
    
    // 分割参数
    let args: Vec<String> = shell_split(&input);

    if args.is_empty() {
        return Ok(false);
    }

    // 先尝试内置命令
    match builtins::execute(&args) {
        Ok(should_exit) => return Ok(should_exit),
        Err(_) => {} // 不是内置命令，继续执行外部命令
    }

    // 通配符展开
    let args = expand_globs(&args);

    // 解析管道和重定向
    let pipeline = parse_pipeline(&args);

    // 执行管道
    execute_pipeline(&pipeline)
}

/// 展开别名
fn expand_aliases(input: &str) -> String {
    STATE.with(|state| {
        let state = state.lock().unwrap();
        let result = input.to_string();
        
        // 检查输入的第一个词是否是别名
        if let Some(first_space) = result.find(' ') {
            let first_word = &result[..first_space];
            if let Some(replacement) = state.aliases.get(first_word) {
                return format!("{} {}", replacement, &result[first_space..].trim());
            }
        } else {
            // 没有参数
            if let Some(replacement) = state.aliases.get(&result) {
                return replacement.clone();
            }
        }
        
        result
    })
}

/// 展开 ~ 为家目录
fn expand_tilde(input: &str) -> String {
    let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
    
    // 替换所有的 ~ 为家目录（简化处理）
    // 注意：这不会正确处理 ~user 格式
    let mut result = input.replace("~/", &format!("{}/", home));
    
    // 处理单独的 ~（后面跟空格或末尾）
    if result == "~" {
        result = home;
    } else {
        // 处理参数中的 ~ 如 "ls ~"
        result = result.replace(" ~", &format!(" {}", home));
    }
    
    result
}

/// 展开变量替换
/// 支持 $VAR, ${VAR}, $?, $$
fn expand_variables(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            match chars.peek() {
                Some(&'{') => {
                    // ${VAR} 格式
                    chars.next(); // 跳过 '{'
                    let mut var_name = String::new();
                    for ch in &mut chars {
                        if ch == '}' {
                            break;
                        }
                        var_name.push(ch);
                    }
                    if let Ok(val) = env::var(&var_name) {
                        result.push_str(&val);
                    }
                    // 如果变量不存在，替换为空字符串
                }
                Some(&'?') => {
                    // $? - 上一个命令的退出码
                    chars.next();
                    let exit_code = STATE.with(|state| {
                        state.lock().unwrap().last_exit_code
                    });
                    result.push_str(&exit_code.to_string());
                }
                Some(&'$') => {
                    // $$ - 当前进程 ID
                    chars.next();
                    result.push_str(&std::process::id().to_string());
                }
                Some(ch) if ch.is_alphabetic() || *ch == '_' => {
                    // $VAR 格式
                    let mut var_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if let Ok(val) = env::var(&var_name) {
                        result.push_str(&val);
                    }
                    // 如果变量不存在，替换为空字符串
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// 展开命令替换
/// 支持 $(command) 和 `command`
fn expand_commands(input: &str) -> io::Result<String> {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'(') {
            // $(command) 格式
            chars.next(); // 跳过 '('
            let mut cmd = String::new();
            let mut depth = 1;
            
            for ch in &mut chars {
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                cmd.push(ch);
            }
            
            // 执行命令并获取输出
            let output = execute_command_for_output(&cmd)?;
            // 去除末尾换行
            result.push_str(output.trim_end());
        } else if c == '`' {
            // `command` 格式
            let mut cmd = String::new();
            for ch in &mut chars {
                if ch == '`' {
                    break;
                }
                cmd.push(ch);
            }
            
            // 执行命令并获取输出
            let output = execute_command_for_output(&cmd)?;
            // 去除末尾换行
            result.push_str(output.trim_end());
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

/// 执行命令并捕获输出
fn execute_command_for_output(cmd: &str) -> io::Result<String> {
    let args: Vec<String> = shell_split(cmd);
    if args.is_empty() {
        return Ok(String::new());
    }
    
    let output = Command::new(&args[0])
        .args(&args[1..])
        .output()?;
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 展开 glob 通配符
/// 支持 *, ?, [...] 等模式
fn expand_globs(args: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    
    for arg in args {
        // 检查是否包含通配符
        if arg.contains('*') || arg.contains('?') || arg.contains('[') {
            let matches = glob_pattern(arg);
            if matches.is_empty() {
                // 如果没有匹配到任何文件，保留原模式
                result.push(arg.clone());
            } else {
                result.extend(matches);
            }
        } else {
            result.push(arg.clone());
        }
    }
    
    result
}

/// 简单的 glob 模式匹配实现
fn glob_pattern(pattern: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    // 分离目录和文件名模式
    let (dir_pattern, file_pattern) = if pattern.contains('/') {
        let pos = pattern.rfind('/').unwrap();
        (&pattern[..pos], &pattern[pos + 1..])
    } else {
        (".", pattern)
    };
    
    // 如果目录不存在，返回空
    let dir_path = Path::new(dir_pattern);
    if !dir_path.exists() || !dir_path.is_dir() {
        return results;
    }
    
    // 读取目录
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            if glob_match(&name_str, file_pattern) {
                let path = if dir_pattern == "." {
                    name_str.to_string()
                } else {
                    format!("{}/{}", dir_pattern, name_str)
                };
                results.push(path);
            }
        }
    }
    
    // 排序结果
    results.sort();
    results
}

/// 检查字符串是否匹配 glob 模式
fn glob_match(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();
    
    // 处理特殊字符
    let mut i = 0; // text index
    let mut j = 0; // pattern index
    let mut star_idx: Option<usize> = None;
    let mut match_idx = 0;
    
    while i < text_chars.len() {
        // 匹配当前字符或使用 [...] 
        if j < pattern_chars.len() && 
           (pattern_chars[j] == '?' || 
            pattern_chars[j] == text_chars[i] ||
            (pattern_chars[j] == '[' && {
                let end = find_bracket_end(&pattern_chars[j..]);
                if end > 0 {
                    let bracket_content = &pattern_chars[j + 1..j + end - 1];
                    let matches = bracket_matches(text_chars[i], bracket_content);
                    if matches {
                        j += end;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }))
        {
            i += 1;
            j += 1;
        } else if j < pattern_chars.len() && pattern_chars[j] == '*' {
            // 记录 * 的位置
            star_idx = Some(j);
            match_idx = i;
            j += 1;
        } else if let Some(idx) = star_idx {
            // 回溯到最后一个 *
            j = idx + 1;
            match_idx += 1;
            i = match_idx;
        } else {
            return false;
        }
    }
    
    // 处理末尾的 *
    while j < pattern_chars.len() && pattern_chars[j] == '*' {
        j += 1;
    }
    
    j == pattern_chars.len()
}

/// 查找 ] 的位置
fn find_bracket_end(chars: &[char]) -> usize {
    for (i, &c) in chars.iter().enumerate() {
        if c == ']' {
            return i + 1;
        }
    }
    0
}

/// 检查字符是否匹配 [...] 内容
fn bracket_matches(c: char, content: &[char]) -> bool {
    if content.is_empty() {
        return false;
    }
    
    let mut i = 0;
    let mut negate = false;
    
    // 检查是否是否定
    if content[0] == '!' || content[0] == '^' {
        negate = true;
        i = 1;
    }
    
    let mut matched = false;
    
    while i < content.len() {
        // 检查范围 a-z
        if i + 2 < content.len() && content[i + 1] == '-' {
            let start = content[i];
            let end = content[i + 2];
            if c >= start && c <= end {
                matched = true;
                break;
            }
            i += 3;
        } else {
            if content[i] == c {
                matched = true;
                break;
            }
            i += 1;
        }
    }
    
    if negate { !matched } else { matched }
}

/// 管道中的单个命令
struct PipelineCommand {
    args: Vec<String>,
    input_file: Option<String>,
    output_file: Option<String>,
    append: bool,
    background: bool,
}

/// 解析管道和重定向
fn parse_pipeline(args: &[String]) -> Vec<PipelineCommand> {
    let mut commands = Vec::new();
    let mut current_args = Vec::new();
    let mut input_file = None;
    let mut output_file = None;
    let mut append = false;
    let mut background = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "|" => {
                // 保存当前命令
                commands.push(PipelineCommand {
                    args: current_args.clone(),
                    input_file: input_file.take(),
                    output_file: output_file.take(),
                    append,
                    background: false, // 管道中的命令不能后台执行
                });
                current_args.clear();
                append = false;
            }
            ">" => {
                if i + 1 < args.len() {
                    output_file = Some(args[i + 1].clone());
                    append = false;
                    i += 1;
                }
            }
            ">>" => {
                if i + 1 < args.len() {
                    output_file = Some(args[i + 1].clone());
                    append = true;
                    i += 1;
                }
            }
            "<" => {
                if i + 1 < args.len() {
                    input_file = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "&" => {
                // 最后一个参数是 &，表示后台执行
                if i == args.len() - 1 {
                    background = true;
                } else {
                    current_args.push("&".to_string());
                }
            }
            _ => {
                current_args.push(args[i].clone());
            }
        }
        i += 1;
    }

    // 添加最后一个命令
    if !current_args.is_empty() {
        commands.push(PipelineCommand {
            args: current_args,
            input_file,
            output_file,
            append,
            background,
        });
    }

    commands
}

/// 执行管道
fn execute_pipeline(commands: &[PipelineCommand]) -> io::Result<bool> {
    if commands.is_empty() {
        return Ok(false);
    }

    if commands.len() == 1 {
        // 单个命令
        return execute_single(&commands[0]);
    }

    // 多个命令，使用管道连接
    // 简化实现：依次执行每个命令，用内存缓冲传递数据
    let mut input_data: Option<Vec<u8>> = None;

    for (i, cmd) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;

        let mut command = Command::new(&cmd.args[0]);
        command.args(&cmd.args[1..]);

        // 设置输入
        if let Some(ref input) = cmd.input_file {
            let file = File::open(input)?;
            command.stdin(Stdio::from(file));
        } else if let Some(ref data) = input_data {
            // 使用管道的 stdin
            command.stdin(Stdio::piped());
            command.stdout(if is_last { Stdio::inherit() } else { Stdio::piped() });

            let mut child = command.spawn()?;
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(data)?;
            }
            let output = child.wait_with_output()?;

            if !is_last {
                input_data = Some(output.stdout);
            }
            continue;
        }

        // 设置输出
        if is_last {
            if let Some(ref output) = cmd.output_file {
                let file = if cmd.append {
                    File::options().create(true).append(true).open(output)?
                } else {
                    File::create(output)?
                };
                command.stdout(Stdio::from(file));
            }
        } else {
            command.stdout(Stdio::piped());
        }

        // 执行命令
        let output = command.output()?;

        if !is_last {
            input_data = Some(output.stdout);
        }
    }

    Ok(false)
}

/// 执行单个命令（无管道）
fn execute_single(cmd: &PipelineCommand) -> io::Result<bool> {
    let mut command = Command::new(&cmd.args[0]);
    command.args(&cmd.args[1..]);

    // 输入重定向
    if let Some(ref input) = cmd.input_file {
        let file = File::open(input)?;
        command.stdin(Stdio::from(file));
    }

    // 输出重定向
    if let Some(ref output) = cmd.output_file {
        let file = if cmd.append {
            File::options().create(true).append(true).open(output)?
        } else {
            File::create(output)?
        };
        command.stdout(Stdio::from(file));
    }

    if cmd.background {
        // 后台执行
        match command.spawn() {
            Ok(child) => {
                let pid = child.id();
                // 将作业添加到列表
                let job_id = STATE.with(|state| {
                    let mut state = state.lock().unwrap();
                    let job_id = state.next_job_id;
                    state.next_job_id += 1;
                    state.jobs.insert(job_id, Job {
                        id: job_id,
                        command: cmd.args.join(" "),
                        pid,
                        state: JobState::Running,
                    });
                    job_id
                });
                println!("[{}] {} (PID: {})", job_id, cmd.args.join(" "), pid);
                // 注意：这里没有 wait，让进程在后台运行
                std::mem::forget(child);
            }
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("{}: {}", cmd.args[0], e),
                ));
            }
        }
        Ok(false)
    } else {
        // 前台执行，等待完成（可被 Ctrl+C 中断）
        let pid;
        let mut child = match command.spawn() {
            Ok(c) => {
                pid = c.id();
                // 设置前台进程 PID
                FOREGROUND_PID.store(pid, Ordering::SeqCst);
                STATE.with(|state| {
                    state.lock().unwrap().foreground_pid = Some(pid);
                });
                c
            }
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("{}: {}", cmd.args[0], e),
                ));
            }
        };
        
        // 等待进程完成，定期检查中断
        loop {
            // 检查 Ctrl+C
            if INTERRUPTED.load(Ordering::SeqCst) {
                INTERRUPTED.store(false, Ordering::SeqCst);
                // 向前台进程发送 SIGINT（使用外部 kill 命令）
                let _ = Command::new("kill")
                    .arg("-INT")
                    .arg(pid.to_string())
                    .output();
                eprintln!("^C");
                FOREGROUND_PID.store(0, Ordering::SeqCst);
                STATE.with(|state| {
                    let mut state = state.lock().unwrap();
                    state.foreground_pid = None;
                    state.last_exit_code = 130; // 128 + SIGINT
                });
                // 等待进程终止
                let _ = child.wait();
                return Err(io::Error::new(io::ErrorKind::Interrupted, "Interrupted"));
            }
            
            // 非阻塞检查进程状态
            match child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code().unwrap_or(0);
                    FOREGROUND_PID.store(0, Ordering::SeqCst);
                    STATE.with(|state| {
                        let mut state = state.lock().unwrap();
                        state.foreground_pid = None;
                        state.last_exit_code = code;
                    });
                    return Ok(false);
                }
                Ok(None) => {
                    // 进程仍在运行，短暂休眠后继续检查
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    FOREGROUND_PID.store(0, Ordering::SeqCst);
                    STATE.with(|state| {
                        let mut state = state.lock().unwrap();
                        state.foreground_pid = None;
                        state.last_exit_code = 1;
                    });
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("{}: {}", cmd.args[0], e),
                    ));
                }
            }
        }
    }
}

/// 简单的 shell 参数分割
/// 支持引号包裹的字符串
fn shell_split(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';

    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' => {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = c;
                } else if c == quote_char {
                    in_quotes = false;
                } else {
                    current.push(c);
                }
            }
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            '|' | '>' | '<' | '&' if !in_quotes => {
                // 管道、重定向和后台符号作为独立参数
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
                // 检查是否是 >>
                if c == '>' {
                    if let Some(&next) = chars.peek() {
                        if next == '>' {
                            chars.next();
                            args.push(">>".to_string());
                            continue;
                        }
                    }
                }
                args.push(c.to_string());
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}
