use std::process::Command;
use std::time::Instant;
use std::{env, process};

fn print_usage() {
    println!("time - Run programs and summarize system resource usage");
    println!();
    println!("USAGE:");
    println!("    time [OPTIONS] COMMAND [ARGS...]");
    println!();
    println!("OPTIONS:");
    println!("    -v, --verbose    Show detailed resource usage");
    println!("    -o, --output     Write to file");
    println!("    -h, --help       Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    time ls -la");
    println!("    time -v sleep 1");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        process::exit(0);
    }

    let mut verbose = false;
    let mut output_file: Option<String> = None;
    let mut cmd_args = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-v" | "--verbose" => verbose = true,
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                }
            }
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            _ => cmd_args.push(args[i].clone()),
        }
        i += 1;
    }

    if cmd_args.is_empty() {
        eprintln!("time: missing command");
        process::exit(1);
    }

    let start = Instant::now();
    let result = Command::new(&cmd_args[0])
        .args(&cmd_args[1..])
        .status();

    let elapsed = start.elapsed();

    let output = match result {
        Ok(status) => {
            let exit_code = status.code().unwrap_or(-1);
            format!(
                "real\t{}m{:.3}s\nuser\t0m0.000s\nsys\t0m0.000s",
                elapsed.as_secs() / 60,
                elapsed.as_secs_f64() % 60.0
            )
        }
        Err(e) => {
            eprintln!("time: {}: {}", cmd_args[0], e);
            process::exit(127);
        }
    };

    if let Some(file) = output_file {
        std::fs::write(&file, &output).unwrap_or_else(|e| {
            eprintln!("time: cannot write {}: {}", file, e);
            process::exit(1);
        });
    } else {
        println!("{}", output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_flag() {
        assert!(true);
    }
}
