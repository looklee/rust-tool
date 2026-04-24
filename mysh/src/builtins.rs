use std::env;
use std::io::{self, Write};
use std::path::Path;

use crate::command;

/// 简单的 glob 匹配（用于 find -name）
fn simple_glob_match(text: &str, pattern: &str) -> bool {
    let pattern = pattern.trim_matches('"').trim_matches('\'');
    
    // 处理 *.ext 模式
    if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        return text.ends_with(suffix);
    }
    
    // 处理 exact 匹配
    text == pattern
}

/// 提取函数名
fn extract_function_name(line: &str) -> Option<String> {
    // fn name(...) 或 pub fn name(...)
    let line = line.trim_start_matches("pub ").trim_start_matches("async ");
    if let Some(rest) = line.strip_prefix("fn ") {
        rest.split('(').next().map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// 提取结构体名
fn extract_struct_name(line: &str) -> Option<String> {
    // struct Name 或 pub struct Name
    let line = line.trim_start_matches("pub ").trim_start_matches("async ");
    if let Some(rest) = line.strip_prefix("struct ") {
        rest.split('{').next().map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// 内置命令执行结果
/// true = 退出 shell, false = 继续
pub fn execute(args: &[String]) -> io::Result<bool> {
    if args.is_empty() {
        return Ok(false);
    }

    match args[0].as_str() {
        "exit" => Ok(true),

        "cd" => {
            let dir = if args.len() > 1 {
                args[1].clone()
            } else {
                // 无参数时回到 home
                env::var("HOME").unwrap_or_else(|_| ".".to_string())
            };

            match env::set_current_dir(Path::new(&dir)) {
                Ok(_) => Ok(false),
                Err(e) => {
                    eprintln!("cd: {}: {}", dir, e);
                    Ok(false)
                }
            }
        }

        "help" => {
            println!("内置命令:");
            println!("  exit          - 退出 shell");
            println!("  cd [DIR]      - 切换目录");
            println!("  pwd           - 显示当前目录");
            println!("  echo [ARGS]   - 输出文本");
            println!("  cat <FILE>    - 显示文件内容");
            println!("  head [-n N]   - 显示文件前 N 行");
            println!("  tail [-n N]   - 显示文件后 N 行");
            println!("  sed -i 's/a/b/' <FILE> - 替换文件内容");
            println!("  git [...]     - Git 版本控制");
            println!("  cinfo         - 显示 Cargo 项目信息");
            println!("  crecommend    - 推荐 Cargo 命令");
            println!("  rscheck       - Rust 代码检查与诊断");
            println!("  rshelp        - Rust 错误诊断帮助");
            println!("  gstatus       - Git status 快捷命令");
            println!("  gdiff         - Git diff 快捷命令");
            println!("  glog          - Git log 快捷命令");
            println!("  find -name    - 查找文件");
            println!("  grep          - 搜索文本");
            println!("  wc            - 统计行数");
            println!("  codeanalyze   - 分析代码结构");
            println!("  ask           - AI 自然语言转命令");
            println!("  explain       - 解释命令作用");
            println!("  fix           - 诊断错误并修复");
            println!("  ollama        - Ollama 本地模型管理");
            println!("  suggestions   - 显示命令建议");
            println!("  help          - 显示帮助");
            println!("  export VAR=value - 设置环境变量");
            println!("  unset VAR      - 删除环境变量");
            println!("  jobs          - 显示后台作业");
            println!("  fg [JOB_ID]   - 将作业移到前台");
            println!("  bg [JOB_ID]   - 继续后台作业");
            println!("  kill [-SIG] PID - 发送信号到进程");
            println!("  type CMD      - 显示命令类型");
            println!("  test/[ ... ]  - 条件测试");
            println!("  ai [CMD]      - AI 编程助手");
            println!("  alias [NAME=CMD] - 设置或显示别名");
            println!();
            println!("脚本控制:");
            println!("  if <cmd>; then <cmd>; [else <cmd>]; fi");
            println!("  for VAR in WORDS; do <cmd>; done");
            println!("  while <cmd>; do <cmd>; done");
            println!();
            println!("变量替换:");
            println!("  $VAR, ${{VAR}}  - 环境变量");
            println!("  $$            - 当前进程 ID");
            println!("  $?            - 上一个命令退出码");
            println!();
            println!("命令替换:");
            println!("  $(command)    - 执行命令并替换结果");
            println!("  `command`     - 执行命令并替换结果");
            println!();
            println!("通配符:");
            println!("  *             - 匹配任意字符");
            println!("  ?             - 匹配单个字符");
            println!("  [...]         - 匹配字符范围");
            println!();
            println!("特殊操作:");
            println!("  command &     - 后台执行命令");
            println!();
            println!("提示符定制:");
            println!("  PS1 环境变量支持以下转义序列:");
            println!("    \\u - 用户名  \\h - 主机名");
            println!("    \\w - 完整路径 \\W - 目录名");
            println!("    \\s - Shell 名 \\$ - #/ $符号");
            println!("  例：export PS1='[\\u@\\h \\W]\\$ '");
            println!();
            println!("外部命令直接执行，例如:");
            println!("  ls -l");
            println!("  echo $HOME | grep home");
            println!("  echo Current dir: $(pwd)");
            println!("  cat *.txt     - 查看所有 txt 文件");
            println!("  sleep 10 &    - 后台执行 sleep");
            println!("  cat < file.txt");
            println!("  echo test > file.txt");
            Ok(false)
        }

        "export" => {
            if args.len() < 2 {
                // 无参数时显示所有环境变量
                for (key, value) in env::vars() {
                    println!("{}={}", key, value);
                }
            } else {
                // 解析 VAR=value 格式
                let arg = &args[1];
                if let Some(pos) = arg.find('=') {
                    let key = &arg[..pos];
                    let value = &arg[pos + 1..];
                    env::set_var(key, value);
                } else {
                    // 只有变量名，显示该变量
                    match env::var(arg) {
                        Ok(val) => println!("{}={}", arg, val),
                        Err(_) => eprintln!("export: variable not found: {}", arg),
                    }
                }
            }
            Ok(false)
        }

        "unset" => {
            if args.len() < 2 {
                eprintln!("unset: usage: unset VAR");
            } else {
                env::remove_var(&args[1]);
            }
            Ok(false)
        }

        "alias" => {
            if args.len() < 2 {
                // 显示所有别名
                command::STATE.with(|state| {
                    let state = state.lock().unwrap();
                    for (name, cmd) in &state.aliases {
                        println!("{}={}", name, cmd);
                    }
                });
            } else {
                // 解析 NAME=CMD 格式
                let arg = &args[1];
                if let Some(pos) = arg.find('=') {
                    let name = &arg[..pos];
                    let cmd = &arg[pos + 1..];
                    command::STATE.with(|state| {
                        state.lock().unwrap()
                            .aliases.insert(name.to_string(), cmd.to_string());
                    });
                } else {
                    // 显示单个别名
                    command::STATE.with(|state| {
                        let state = state.lock().unwrap();
                        if let Some(cmd) = state.aliases.get(arg) {
                            println!("{}={}", arg, cmd);
                        } else {
                            eprintln!("alias: not found: {}", arg);
                        }
                    });
                }
            }
            Ok(false)
        }

        "jobs" => {
            command::STATE.with(|state| {
                let state = state.lock().unwrap();
                if state.jobs.is_empty() {
                    println!("No background jobs");
                } else {
                    for (_, job) in &state.jobs {
                        let status = match job.state {
                            command::JobState::Running => "Running",
                            command::JobState::Stopped => "Stopped",
                            command::JobState::Completed => "Completed",
                        };
                        println!("[{}]\t{}\t(PID: {})\t{}", job.id, job.command, job.pid, status);
                    }
                }
            });
            Ok(false)
        }

        "fg" => {
            if args.len() < 2 {
                eprintln!("fg: usage: fg <JOB_ID>");
                eprintln!("提示：使用 'jobs' 查看作业列表");
            } else {
                let job_id: usize = match args[1].parse() {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("fg: invalid job ID: {}", args[1]);
                        return Ok(false);
                    }
                };
                
                command::STATE.with(|state| {
                    let state = state.lock().unwrap();
                    if let Some(job) = state.jobs.get(&job_id) {
                        println!("Bringing job [{}] to foreground: {}", job_id, job.command);
                        // 在实际实现中，这里需要使用 waitpid 等待进程
                        // 由于我们使用了 mem::forget，无法直接 wait
                        // 这是一个简化实现
                        println!("fg: 作业已在后台运行，使用 'kill {}' 终止", job.pid);
                    } else {
                        eprintln!("fg: job {} not found", job_id);
                    }
                });
            }
            Ok(false)
        }

        "bg" => {
            if args.len() < 2 {
                eprintln!("bg: usage: bg <JOB_ID>");
                eprintln!("提示：使用 'jobs' 查看作业列表");
            } else {
                let job_id: usize = match args[1].parse() {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("bg: invalid job ID: {}", args[1]);
                        return Ok(false);
                    }
                };
                
                command::STATE.with(|state| {
                    let mut state = state.lock().unwrap();
                    if let Some(job) = state.jobs.get_mut(&job_id) {
                        if job.state == command::JobState::Stopped {
                            job.state = command::JobState::Running;
                            println!("Continuing job [{}] in background", job_id);
                            // 在实际实现中，这里需要发送 SIGCONT 信号
                        } else {
                            println!("Job [{}] is already running", job_id);
                        }
                    } else {
                        eprintln!("bg: job {} not found", job_id);
                    }
                });
            }
            Ok(false)
        }

        "pwd" => {
            match env::current_dir() {
                Ok(path) => {
                    println!("{}", path.display());
                }
                Err(e) => {
                    eprintln!("pwd: {}", e);
                }
            }
            Ok(false)
        }

        "echo" => {
            // 简单的 echo 实现
            let output: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
            println!("{}", output.join(" "));
            Ok(false)
        }

        "kill" => {
            if args.len() < 2 {
                eprintln!("kill: usage: kill [-SIGNAL] PID");
                Ok(false)
            } else {
                let mut signal = 15; // SIGTERM
                let mut pid_idx = 1;
                
                // 检查是否有信号参数
                if args[1].starts_with('-') {
                    signal = match args[1][1..].parse() {
                        Ok(s) => s,
                        Err(_) => {
                            eprintln!("kill: invalid signal: {}", args[1]);
                            return Ok(false);
                        }
                    };
                    pid_idx = 2;
                }
                
                if pid_idx >= args.len() {
                    eprintln!("kill: usage: kill [-SIGNAL] PID");
                    return Ok(false);
                }
                
                let pid: i32 = match args[pid_idx].parse() {
                    Ok(p) => p,
                    Err(_) => {
                        eprintln!("kill: invalid PID: {}", args[pid_idx]);
                        return Ok(false);
                    }
                };
                
                // 使用 nix 或 libc 发送信号，这里简化为调用外部 kill 命令
                match std::process::Command::new("kill")
                    .arg("-s")
                    .arg(signal.to_string())
                    .arg(pid.to_string())
                    .output()
                {
                    Ok(_) => Ok(false),
                    Err(e) => {
                        eprintln!("kill: {}", e);
                        Ok(false)
                    }
                }
            }
        }

        "type" => {
            if args.len() < 2 {
                eprintln!("type: usage: type COMMAND");
                Ok(false)
            } else {
                let cmd = &args[1];
                // 检查是否是内置命令
                match cmd.as_str() {
                    "exit" | "cd" | "help" | "export" | "unset" | "alias" | "jobs" | "fg" | "bg" | "pwd" | "echo" | "kill" | "type" | "test" | "[" => {
                        println!("{} is a shell builtin", cmd);
                    }
                    _ => {
                        // 检查是否是外部命令
                        match std::process::Command::new(cmd).output() {
                            Ok(_) => {
                                println!("{} is an external command", cmd);
                            }
                            Err(_) => {
                                eprintln!("type: {}: not found", cmd);
                            }
                        }
                    }
                }
                Ok(false)
            }
        }

        "test" | "[" => {
            // test 命令或 [ 命令
            let test_args = if args[0] == "[" {
                // 移除末尾的 ]
                if args.last().map(|s| s.as_str()) == Some("]") {
                    &args[1..args.len()-1]
                } else {
                    eprintln!("[: missing ]");
                    // 返回 Ok 但设置退出码为 2（错误用法）
                    crate::command::STATE.with(|state| {
                        state.lock().unwrap().last_exit_code = 2;
                    });
                    return Ok(false);
                }
            } else {
                &args[1..]
            };
            
            match crate::script::execute_test(test_args) {
                Ok(_) => Ok(false), // 测试通过，退出码 0
                Err(_) => {
                    // 测试失败，设置退出码为 1
                    crate::command::STATE.with(|state| {
                        state.lock().unwrap().last_exit_code = 1;
                    });
                    Ok(false) // 返回 Ok 表示命令执行完成（只是退出码非 0）
                }
            }
        }

        "ai" => {
            // AI 助手命令 - 调用外部脚本
            if args.len() < 2 {
                println!("AI Code Assistant");
                println!("使用方法:");
                println!("  ai analyze <代码>  - 分析代码");
                println!("  ai generate <描述> - 生成代码");
                println!("  ai explain <代码>  - 解释代码");
                println!();
                println!("注意：需要设置 OPENAI_API_KEY 环境变量");
            } else {
                let output = std::process::Command::new("/root/ai-assist.sh")
                    .args(&args[1..])
                    .output();
                
                match output {
                    Ok(out) => {
                        if !out.stdout.is_empty() {
                            println!("{}", String::from_utf8_lossy(&out.stdout));
                        }
                        if !out.stderr.is_empty() {
                            eprintln!("{}", String::from_utf8_lossy(&out.stderr));
                        }
                    }
                    Err(e) => {
                        eprintln!("ai: {}", e);
                    }
                }
            }
            Ok(false)
        }

        "sed" => {
            // 简化的 sed 实现 - 用于文件内容替换
            if args.len() < 4 {
                eprintln!("sed: usage: sed -i 's/old/new/g' <file>");
            } else {
                let mut file_path = None;
                let mut pattern = None;
                
                let mut i = 1;
                while i < args.len() {
                    if args[i] == "-i" {
                        // 原地编辑
                        if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                            file_path = Some(args[i + 1].clone());
                            i += 1;
                        }
                    } else if args[i].starts_with("s/") {
                        pattern = Some(args[i].clone());
                    } else if !args[i].starts_with("-") && file_path.is_none() {
                        file_path = Some(args[i].clone());
                    }
                    i += 1;
                }
                
                if let (Some(path), Some(pat)) = (file_path, pattern) {
                    // 解析 sed 模式 s/old/new/flags
                    let parts: Vec<&str> = pat.split('/').collect();
                    if parts.len() >= 4 && parts[0] == "s" {
                        let old = parts[1];
                        let new = parts[2];
                        
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                let new_content = content.replace(old, new);
                                match std::fs::write(&path, new_content) {
                                    Ok(_) => println!("Modified: {}", path),
                                    Err(e) => eprintln!("sed: write error: {}", e),
                                }
                            }
                            Err(e) => eprintln!("sed: read error: {}", e),
                        }
                    } else {
                        eprintln!("sed: invalid pattern: {}", pat);
                    }
                } else {
                    eprintln!("sed: usage: sed -i 's/old/new/g' <file>");
                }
            }
            Ok(false)
        }

        "cat" => {
            // 简化的 cat 实现
            if args.len() < 2 {
                eprintln!("cat: usage: cat <file>");
            } else {
                for file in &args[1..] {
                    match std::fs::read_to_string(file) {
                        Ok(content) => print!("{}", content),
                        Err(e) => eprintln!("cat: {}: {}", file, e),
                    }
                }
            }
            Ok(false)
        }

        "head" => {
            // 简化的 head 实现
            let mut lines_to_show = 10;
            let mut file_path = None;
            
            for arg in &args[1..] {
                if let Some(n) = arg.strip_prefix("-n") {
                    lines_to_show = n.parse().unwrap_or(10);
                } else if !arg.starts_with('-') {
                    file_path = Some(arg.as_str());
                }
            }
            
            if let Some(path) = file_path {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        for line in content.lines().take(lines_to_show) {
                            println!("{}", line);
                        }
                    }
                    Err(e) => eprintln!("head: {}: {}", path, e),
                }
            }
            Ok(false)
        }

        "tail" => {
            // 简化的 tail 实现
            let mut lines_to_show = 10;
            let mut file_path = None;
            
            for arg in &args[1..] {
                if let Some(n) = arg.strip_prefix("-n") {
                    lines_to_show = n.parse().unwrap_or(10);
                } else if !arg.starts_with('-') {
                    file_path = Some(arg.as_str());
                }
            }
            
            if let Some(path) = file_path {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let lines: Vec<&str> = content.lines().collect();
                        let start = if lines.len() > lines_to_show {
                            lines.len() - lines_to_show
                        } else {
                            0
                        };
                        for line in lines.iter().skip(start) {
                            println!("{}", line);
                        }
                    }
                    Err(e) => eprintln!("tail: {}: {}", path, e),
                }
            }
            Ok(false)
        }

        "git" => {
            // Git 命令包装器
            let output = std::process::Command::new("git")
                .args(&args[1..])
                .output();
            
            match output {
                Ok(out) => {
                    if !out.stdout.is_empty() {
                        print!("{}", String::from_utf8_lossy(&out.stdout));
                    }
                    if !out.stderr.is_empty() {
                        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
                    }
                    // 返回 git 的退出码
                    crate::command::STATE.with(|state| {
                        state.lock().unwrap().last_exit_code = out.status.code().unwrap_or(1);
                    });
                }
                Err(e) => {
                    eprintln!("git: command not found: {}", e);
                }
            }
            Ok(false)
        }

        "gstatus" => {
            // git status 快捷命令
            let output = std::process::Command::new("git")
                .arg("status")
                .output();
            
            match output {
                Ok(out) => {
                    print!("{}", String::from_utf8_lossy(&out.stdout));
                    if !out.stderr.is_empty() {
                        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
                    }
                }
                Err(e) => {
                    eprintln!("gstatus: git not found: {}", e);
                }
            }
            Ok(false)
        }

        "gdiff" => {
            // git diff 快捷命令
            let output = std::process::Command::new("git")
                .arg("diff")
                .output();
            
            match output {
                Ok(out) => {
                    print!("{}", String::from_utf8_lossy(&out.stdout));
                    if !out.stderr.is_empty() {
                        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
                    }
                }
                Err(e) => {
                    eprintln!("gdiff: git not found: {}", e);
                }
            }
            Ok(false)
        }

        "glog" => {
            // git log 快捷命令
            let output = std::process::Command::new("git")
                .arg("log")
                .arg("--oneline")
                .arg("-10")
                .output();
            
            match output {
                Ok(out) => {
                    print!("{}", String::from_utf8_lossy(&out.stdout));
                    if !out.stderr.is_empty() {
                        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
                    }
                }
                Err(e) => {
                    eprintln!("glog: git not found: {}", e);
                }
            }
            Ok(false)
        }

        "find" => {
            // 简化的 find 实现
            let mut path = ".";
            let mut name_pattern = None;
            let mut _max_depth: Option<usize> = None;
            
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "-name" => {
                        if i + 1 < args.len() {
                            name_pattern = Some(args[i + 1].as_str());
                            i += 1;
                        }
                    }
                    "-maxdepth" => {
                        if i + 1 < args.len() {
                            _max_depth = args[i + 1].parse().ok();
                            i += 1;
                        }
                    }
                    _ => {
                        if !args[i].starts_with('-') {
                            path = &args[i];
                        }
                    }
                }
                i += 1;
            }
            
            match std::fs::read_dir(path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        
                        // 检查名称匹配
                        if let Some(pattern) = name_pattern {
                            if !simple_glob_match(&name, pattern) {
                                continue;
                            }
                        }
                        
                        println!("{}", path.display());
                    }
                }
                Err(e) => {
                    eprintln!("find: {}: {}", path, e);
                }
            }
            Ok(false)
        }

        "grep" => {
            // 简化的 grep 实现
            if args.len() < 3 {
                eprintln!("grep: usage: grep <pattern> <file>");
            } else {
                let pattern = &args[1];
                let file = &args[2];
                
                match std::fs::read_to_string(file) {
                    Ok(content) => {
                        for (line_num, line) in content.lines().enumerate() {
                            if line.contains(pattern.as_str()) {
                                println!("{}:{}", line_num + 1, line);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("grep: {}: {}", file, e);
                    }
                }
            }
            Ok(false)
        }

        "wc" => {
            // 简化的 wc 实现
            if args.len() < 2 {
                eprintln!("wc: usage: wc <file>");
            } else {
                for file in &args[1..] {
                    match std::fs::read_to_string(file) {
                        Ok(content) => {
                            let lines = content.lines().count();
                            let words = content.split_whitespace().count();
                            let bytes = content.len();
                            println!(" {} {} {} {}", lines, words, bytes, file);
                        }
                        Err(e) => {
                            eprintln!("wc: {}: {}", file, e);
                        }
                    }
                }
            }
            Ok(false)
        }

        "codeanalyze" => {
            // 代码分析命令
            if args.len() < 2 {
                eprintln!("codeanalyze: usage: codeanalyze <file>");
                eprintln!("分析代码文件，显示:");
                eprintln!("  - 行数、单词数");
                eprintln!("  - 函数/方法定义");
                eprintln!("  - 导入/使用语句");
            } else {
                let file = &args[1];
                match std::fs::read_to_string(file) {
                    Ok(content) => {
                        let lines = content.lines().count();
                        let words = content.split_whitespace().count();
                        
                        println!("=== 代码分析：{} ===", file);
                        println!("行数：{}", lines);
                        println!("单词数：{}", words);
                        println!();
                        
                        // 分析代码结构
                        let mut functions = Vec::new();
                        let mut imports = Vec::new();
                        let mut structs = Vec::new();
                        
                        for (line_num, line) in content.lines().enumerate() {
                            let trimmed = line.trim();
                            
                            // 检测函数定义
                            if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
                                if let Some(name) = extract_function_name(trimmed) {
                                    functions.push((line_num + 1, name));
                                }
                            }
                            
                            // 检测 use 语句
                            if trimmed.starts_with("use ") {
                                imports.push((line_num + 1, trimmed.to_string()));
                            }
                            
                            // 检测 struct 定义
                            if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
                                if let Some(name) = extract_struct_name(trimmed) {
                                    structs.push((line_num + 1, name));
                                }
                            }
                        }
                        
                        if !imports.is_empty() {
                            println!("导入 ({}):", imports.len());
                            for (line, imp) in &imports {
                                println!("  {}: {}", line, imp);
                            }
                            println!();
                        }
                        
                        if !structs.is_empty() {
                            println!("结构体 ({}):", structs.len());
                            for (line, name) in &structs {
                                println!("  {}: {}", line, name);
                            }
                            println!();
                        }
                        
                        if !functions.is_empty() {
                            println!("函数 ({}):", functions.len());
                            for (line, name) in &functions {
                                println!("  {}: {}", line, name);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("codeanalyze: {}: {}", file, e);
                    }
                }
            }
            Ok(false)
        }

        "ask" => {
            // AI 智能命令 - 自然语言转命令
            if args.len() < 2 {
                println!("AI 智能助手");
                println!("使用方法:");
                println!("  ask [描述]     - 将自然语言转为命令");
                println!("  explain [命令]  - 解释命令的作用");
                println!("  fix [错误]     - 诊断错误并给出建议");
                println!();
            } else {
                let mut ai = crate::ai_assistant::AIAssistant::new();
                let query = args[1..].join(" ");
                
                print!("思考中... ");
                let _ = io::stdout().flush();
                
                match ai.nl_to_command(&query) {
                    Ok(cmd) => {
                        println!("\n建议命令：{}", cmd);
                        println!();
                    }
                    Err(e) => {
                        println!();
                        eprintln!("错误：{}", e);
                    }
                }
            }
            Ok(false)
        }

        "explain" => {
            // 解释命令
            if args.len() < 2 {
                eprintln!("explain: usage: explain <命令>");
            } else {
                let ai = crate::ai_assistant::AIAssistant::new();
                let cmd = args[1..].join(" ");
                
                print!("分析中... ");
                let _ = io::stdout().flush();
                
                match ai.explain_command(&cmd) {
                    Ok(explanation) => {
                        println!();
                        println!("{}", explanation);
                        println!();
                    }
                    Err(e) => {
                        println!();
                        eprintln!("错误：{}", e);
                    }
                }
            }
            Ok(false)
        }

        "fix" => {
            // 错误诊断
            if args.len() < 3 {
                eprintln!("fix: usage: fix <命令> <错误信息>");
                eprintln!("示例：fix git push fatal: remote not found");
            } else {
                let ai = crate::ai_assistant::AIAssistant::new();
                let cmd = &args[1];
                let error = args[2..].join(" ");
                
                print!("诊断中... ");
                let _ = io::stdout().flush();
                
                match ai.diagnose_error(cmd, &error) {
                    Ok(suggestion) => {
                        println!();
                        println!("{}", suggestion);
                        println!();
                    }
                    Err(e) => {
                        println!();
                        eprintln!("错误：{}", e);
                    }
                }
            }
            Ok(false)
        }

        "ollama" => {
            // Ollama 模型管理
            if args.len() < 2 {
                println!("Ollama 本地模型管理");
                println!("使用方法:");
                println!("  ollama list         - 列出已安装的模型");
                println!("  ollama pull <model> - 下载模型");
                println!("  ollama run <model>  - 运行模型");
                println!();
                println!("常用模型：llama3.2, qwen2.5, mistral");
            } else {
                let output = std::process::Command::new("ollama")
                    .args(&args[1..])
                    .output();
                
                match output {
                    Ok(out) => {
                        if !out.stdout.is_empty() {
                            print!("{}", String::from_utf8_lossy(&out.stdout));
                        }
                        if !out.stderr.is_empty() {
                            eprintln!("{}", String::from_utf8_lossy(&out.stderr));
                        }
                    }
                    Err(e) => {
                        eprintln!("ollama: {}", e);
                        eprintln!("提示：请先安装 Ollama: https://ollama.ai");
                    }
                }
            }
            Ok(false)
        }

        "cinfo" => {
            // Cargo 项目信息
            println!("{}", crate::cargo_cmd::get_cargo_info());
            Ok(false)
        }

        "crecommend" => {
            // Cargo 命令推荐
            println!("推荐的 Cargo 命令:");
            for rec in crate::cargo_cmd::get_cargo_recommendations() {
                println!("  {}", rec);
            }
            Ok(false)
        }

        "rscheck" => {
            // Rust 代码检查
            if !crate::cargo_cmd::is_cargo_project() {
                eprintln!("rscheck: 当前目录不是 Cargo 项目");
            } else {
                println!("运行 Rust 代码检查...\n");
                let output = std::process::Command::new("cargo")
                    .arg("check")
                    .output();
                
                match output {
                    Ok(out) => {
                        if out.status.success() {
                            println!("✓ 代码检查通过，没有错误");
                        } else {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            println!("{}", crate::rust_diagnostic::RustDiagnostic::analyze_error(&stderr));
                        }
                    }
                    Err(e) => {
                        eprintln!("rscheck: {}", e);
                    }
                }
            }
            Ok(false)
        }

        "rshelp" => {
            // Rust 错误诊断帮助
            println!("{}", crate::rust_diagnostic::RustDiagnostic::get_rust_help());
            Ok(false)
        }

        "suggestions" => {
            // 显示智能建议
            let completer = crate::completer::Completer::new();
            let partial = if args.len() > 1 { args[1].clone() } else { "".to_string() };
            
            println!("命令建议:");
            for suggestion in completer.complete(&partial) {
                println!("  {}", suggestion);
            }
            
            if !partial.is_empty() {
                println!("\n智能建议:");
                for suggestion in completer.get_smart_suggestions(&partial) {
                    println!("  {}", suggestion);
                }
            }
            Ok(false)
        }

        _ => {
            // 不是内置命令
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("command not found: {}", args[0]),
            ))
        }
    }
}
