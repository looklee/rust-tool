use std::io;
use crate::command;

/// 解析并执行 if/else/fi 结构
/// 支持语法：if <cmd>; then <cmd>; [else <cmd>]; fi
pub fn execute_if(input: &str) -> io::Result<bool> {
    // 检查是否是 if 语句
    if !input.trim().starts_with("if ") {
        return Ok(false);
    }

    // 解析 if/then/else/fi
    let parts = parse_if_statement(input);
    
    // 执行条件命令并检查退出码
    let condition_result = command::execute(&parts.condition);
    
    // 获取退出码（0 表示成功）
    let exit_code = crate::command::STATE.with(|state| {
        state.lock().unwrap().last_exit_code
    });
    
    let condition_passed = condition_result.is_ok() && exit_code == 0;
    
    if condition_passed {
        // 条件成功，执行 then 分支
        if !parts.then_cmd.is_empty() {
            command::execute(&parts.then_cmd)?;
        }
    } else if let Some(else_cmd) = parts.else_cmd {
        // 条件失败，执行 else 分支
        if !else_cmd.is_empty() {
            command::execute(&else_cmd)?;
        }
    }
    
    Ok(false)
}

/// 执行 for 循环
/// 支持语法：for VAR in WORDS; do CMD; done
pub fn execute_for(input: &str) -> io::Result<bool> {
    let input = input.trim();
    if !input.starts_with("for ") {
        return Ok(false);
    }
    
    // 解析 for 循环
    if let Some(loop_parts) = parse_for_loop(input) {
        let ParseForLoop { var, values, body } = loop_parts;
        
        for value in values {
            // 设置变量
            command::STATE.with(|state| {
                state.lock().unwrap().aliases.insert(var.clone(), value.clone());
            });
            
            // 执行循环体
            command::execute(&body)?;
        }
        
        // 清理变量
        command::STATE.with(|state| {
            state.lock().unwrap().aliases.remove(&var);
        });
    }
    
    Ok(false)
}

/// 执行 while 循环
/// 支持语法：while <cmd>; do <cmd>; done
pub fn execute_while(input: &str) -> io::Result<bool> {
    let input = input.trim();
    if !input.starts_with("while ") {
        return Ok(false);
    }
    
    // 解析 while 循环
    if let Some((condition, body)) = parse_while_loop(input) {
        loop {
            // 执行条件命令
            let result = command::execute(&condition);
            let exit_code = crate::command::STATE.with(|state| {
                state.lock().unwrap().last_exit_code
            });
            
            // 如果条件失败（退出码非 0），退出循环
            if result.is_err() || exit_code != 0 {
                break;
            }
            
            // 执行循环体
            command::execute(&body)?;
        }
    }
    
    Ok(false)
}

struct IfParts {
    condition: String,
    then_cmd: String,
    else_cmd: Option<String>,
}

struct ParseForLoop {
    var: String,
    values: Vec<String>,
    body: String,
}

/// 解析 for 循环
fn parse_for_loop(input: &str) -> Option<ParseForLoop> {
    // for VAR in WORDS; do CMD; done
    let input = input.trim();
    
    // 移除 "for "
    let rest = input.strip_prefix("for ")?;
    
    // 查找 " in "
    let in_pos = rest.find(" in ")?;
    let var = rest[..in_pos].trim().to_string();
    
    // 获取 in 之后的部分
    let after_in = &rest[in_pos + 4..];
    
    // 查找 "; do"
    let do_pos = after_in.find("; do")?;
    let values_str = after_in[..do_pos].trim();
    
    // 分割值
    let values: Vec<String> = values_str.split_whitespace()
        .map(|s| s.to_string())
        .collect();
    
    // 获取 do 之后的部分
    let after_do = &after_in[do_pos + 5..];
    
    // 查找 "; done" 或 " done"
    let done_pos = after_do.find("; done")
        .or_else(|| after_do.find(" done"))
        .unwrap_or(after_do.len());
    
    let body = after_do[..done_pos].trim().to_string();
    
    Some(ParseForLoop { var, values, body })
}

/// 解析 while 循环
fn parse_while_loop(input: &str) -> Option<(String, String)> {
    // while <cmd>; do <cmd>; done
    let input = input.trim();
    
    // 移除 "while "
    let rest = input.strip_prefix("while ")?;
    
    // 查找 "; do"
    let do_pos = rest.find("; do")?;
    let condition = rest[..do_pos].trim().to_string();
    
    // 获取 do 之后的部分
    let after_do = &rest[do_pos + 5..];
    
    // 查找 "; done"
    let done_pos = after_do.find("; done")
        .or_else(|| after_do.find(" done"))
        .unwrap_or(after_do.len());
    
    let body = after_do[..done_pos].trim().to_string();
    
    Some((condition, body))
}

/// 解析 if 语句
fn parse_if_statement(input: &str) -> IfParts {
    let input = input.trim();
    
    // 移除开头的 "if "
    let rest = &input[3..];
    
    // 查找 "; then"
    let then_pos = rest.find("; then")
        .or_else(|| rest.find(" ; then"))
        .or_else(|| rest.find(";then"))
        .unwrap_or(rest.len());
    
    let condition = rest[..then_pos].trim().to_string();
    
    // 获取 then 之后的部分
    let then_start = then_pos + 6; // "; then".len()
    let then_part = &rest[then_start..];
    
    // 查找 "; else" 或 "; fi"
    let else_pos = then_part.find("; else")
        .or_else(|| then_part.find(" ; else"))
        .or_else(|| then_part.find(";else"));
    
    let fi_pos = then_part.find("; fi")
        .or_else(|| then_part.find(" ; fi"))
        .or_else(|| then_part.find(";fi"))
        .unwrap_or(then_part.len());
    
    let (then_cmd, else_cmd) = if let Some(ep) = else_pos {
        // 有 else 分支
        let then_cmd = then_part[..ep].trim().to_string();
        let else_start = ep + 7; // "; else".len()
        let else_part = &then_part[else_start..];
        // 移除末尾的 fi
        let else_cmd = else_part.strip_suffix("; fi")
            .or_else(|| else_part.strip_suffix(" ; fi"))
            .or_else(|| else_part.strip_suffix(";fi"))
            .unwrap_or(else_part)
            .trim()
            .to_string();
        (then_cmd, Some(else_cmd))
    } else {
        // 没有 else 分支
        let then_cmd = then_part[..fi_pos].trim().to_string();
        (then_cmd, None)
    };
    
    IfParts {
        condition,
        then_cmd,
        else_cmd,
    }
}

/// 执行 test/[ 命令
pub fn execute_test(args: &[String]) -> io::Result<bool> {
    if args.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "test: missing argument",
        ));
    }
    
    // 简化实现：只支持基本的字符串和文件测试
    let result = if args.len() >= 2 && 
        (args[0].starts_with('-') || args[0] == "=" || args[0] == "==" || args[0] == "!=") {
        // 一元或二元操作符
        match args[0].as_str() {
            // 字符串测试
            "-z" => {
                // 空字符串测试
                args[1].is_empty()
            }
            "-n" => {
                // 非空字符串测试
                !args[1].is_empty()
            }
            // 文件测试
            "-f" => {
                // 普通文件
                std::path::Path::new(&args[1]).is_file()
            }
            "-d" => {
                // 目录
                std::path::Path::new(&args[1]).is_dir()
            }
            "-e" => {
                // 文件/目录存在
                std::path::Path::new(&args[1]).exists()
            }
            // 字符串比较
            "=" | "==" => {
                if args.len() < 3 {
                    false
                } else {
                    args[1] == args[2]
                }
            }
            "!=" => {
                if args.len() < 3 {
                    false
                } else {
                    args[1] != args[2]
                }
            }
            // 数值比较
            "-eq" => compare_numbers(&args[1], &args[2], |a, b| a == b),
            "-ne" => compare_numbers(&args[1], &args[2], |a, b| a != b),
            "-lt" => compare_numbers(&args[1], &args[2], |a, b| a < b),
            "-le" => compare_numbers(&args[1], &args[2], |a, b| a <= b),
            "-gt" => compare_numbers(&args[1], &args[2], |a, b| a > b),
            "-ge" => compare_numbers(&args[1], &args[2], |a, b| a >= b),
            _ => !args[0].is_empty(),
        }
    } else if args.len() >= 3 {
        // 数值比较（中缀语法：num1 op num2）
        match args[1].as_str() {
            "-eq" => compare_numbers(&args[0], &args[2], |a, b| a == b),
            "-ne" => compare_numbers(&args[0], &args[2], |a, b| a != b),
            "-lt" => compare_numbers(&args[0], &args[2], |a, b| a < b),
            "-le" => compare_numbers(&args[0], &args[2], |a, b| a <= b),
            "-gt" => compare_numbers(&args[0], &args[2], |a, b| a > b),
            "-ge" => compare_numbers(&args[0], &args[2], |a, b| a >= b),
            // 字符串比较
            "=" | "==" => args[0] == args[2],
            "!=" => args[0] != args[2],
            _ => !args[0].is_empty(),
        }
    } else {
        // 默认：如果参数非空则返回 true
        !args[0].is_empty()
    };
    
    if result {
        Ok(false) // 成功，退出码 0
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "test failed",
        ))
    }
}

fn compare_numbers<F>(a: &str, b: &str, op: F) -> bool
where
    F: Fn(i64, i64) -> bool,
{
    let a: i64 = match a.parse() {
        Ok(n) => n,
        Err(_) => return false,
    };
    
    let b: i64 = match b.parse() {
        Ok(n) => n,
        Err(_) => return false,
    };
    
    op(a, b)
}
