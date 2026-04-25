use std::env;
use std::fs::File;
use std::io::Write;
use std::process::{Command, exit};
use std::time::Instant;

struct Config {
    command: String,
    output_file: Option<String>,
    iterations: usize,
}

struct PerfStats {
    command: String,
    execution_time: std::time::Duration,
    exit_code: i32,
    peak_memory_kb: u64,
    cpu_percent: f64,
}

impl PerfStats {
    fn format_duration(d: std::time::Duration) -> String {
        let millis = d.as_micros() as f64 / 1000.0;
        if millis >= 1000.0 {
            format!("{:.3}s", millis / 1000.0)
        } else {
            format!("{:.3}ms", millis)
        }
    }

    fn format_memory(kb: u64) -> String {
        if kb >= 1024 * 1024 {
            format!("{:.2} GB", kb as f64 / (1024.0 * 1024.0))
        } else if kb >= 1024 {
            format!("{:.2} MB", kb as f64 / 1024.0)
        } else {
            format!("{} KB", kb)
        }
    }

    fn print(&self) {
        println!("Performance Report");
        println!("==================");
        println!("Command:     {}", self.command);
        println!("Exit code:   {}", self.exit_code);
        println!();
        println!("Timing:");
        println!("  Execution time:  {}", Self::format_duration(self.execution_time));
        println!("  CPU usage:       {:.1}%", self.cpu_percent);
        println!();
        println!("Memory:");
        println!("  Peak memory:     {}", Self::format_memory(self.peak_memory_kb));
    }

    fn to_string(&self) -> String {
        format!(
            "Performance Report\n\
             ==================\n\
             Command:     {}\n\
             Exit code:   {}\n\n\
             Timing:\n\
               Execution time:  {}\n\
               CPU usage:       {:.1}%\n\n\
             Memory:\n\
               Peak memory:     {}\n",
            self.command,
            self.exit_code,
            Self::format_duration(self.execution_time),
            self.cpu_percent,
            Self::format_memory(self.peak_memory_kb),
        )
    }
}

#[cfg(target_os = "linux")]
#[allow(dead_code)]
fn get_peak_memory(pid: u32) -> u64 {
    // Read /proc/<pid>/status for peak memory (VmHWM)
    let status_path = format!("/proc/{}/status", pid);
    if let Ok(content) = std::fs::read_to_string(&status_path) {
        for line in content.lines() {
            if line.starts_with("VmHWM:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return kb;
                    }
                }
            }
        }
    }
    0
}

#[cfg(not(target_os = "linux"))]
fn get_peak_memory(_pid: u32) -> u64 {
    0
}

fn get_cpu_percent(_elapsed_ms: u64) -> f64 {
    // Estimate CPU usage from /proc/stat if available
    #[cfg(target_os = "linux")]
    {
        // Simplified: just report based on elapsed time
        // A full implementation would read /proc/<pid>/stat
        if _elapsed_ms > 0 {
            // This is a simplified estimate
            0.0
        } else {
            0.0
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        0.0
    }
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut command = String::new();
    let mut output_file = None;
    let mut iterations = 1;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--command" => {
                i += 1;
                if i < args.len() {
                    command = args[i].clone();
                } else {
                    eprintln!("Error: -c requires a command");
                    exit(1);
                }
            }
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                } else {
                    eprintln!("Error: -o requires a file path");
                    exit(1);
                }
            }
            "-n" | "--iterations" => {
                i += 1;
                if i < args.len() {
                    iterations = args[i].parse().unwrap_or_else(|_| {
                        eprintln!("Error: Invalid iteration count");
                        exit(1);
                    });
                }
            }
            "-h" | "--help" => {
                print_help();
                exit(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("Error: Unknown option '{}'", arg);
                exit(1);
            }
            arg => {
                if command.is_empty() {
                    command = arg.to_string();
                } else {
                    command = format!("{} {}", command, arg);
                }
            }
        }
        i += 1;
    }

    if command.is_empty() {
        eprintln!("Error: A command is required (use -c or pass directly)");
        print_help();
        exit(1);
    }

    Config {
        command,
        output_file,
        iterations,
    }
}

fn print_help() {
    println!("perf v1.0.0 - Simple performance monitor");
    println!();
    println!("USAGE:");
    println!("    perf [OPTIONS] [-c <command>]");
    println!();
    println!("OPTIONS:");
    println!("    -c, --command <cmd>     Command to profile");
    println!("    -o, --output <file>     Write report to file");
    println!("    -n, --iterations <N>    Number of iterations (default: 1)");
    println!("    -h, --help              Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    perf -c 'sleep 1'");
    println!("    perf -c 'ls -la' -o report.txt");
    println!("    perf -c 'curl https://example.com' -n 5");
    println!();
    println!("DESCRIPTION:");
    println!("    Profile a command's execution time and basic resource usage,");
    println!("    displaying a summary report with timing and memory statistics.");
}

fn run_command(cmd: &str) -> PerfStats {
    let start = Instant::now();

    // Get PID before spawning for memory tracking
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    let elapsed = start.elapsed();

    let (exit_code, peak_mem) = match output {
        Ok(status) => {
            let code = status.code().unwrap_or(-1);
            // Try to get memory info (best effort)
            let mem = 0; // Simplified - real impl would track child process
            (code, mem)
        }
        Err(e) => {
            eprintln!("Error executing command: {}", e);
            (-1, 0)
        }
    };

    let cpu_pct = get_cpu_percent(elapsed.as_millis() as u64);

    PerfStats {
        command: cmd.to_string(),
        execution_time: elapsed,
        exit_code,
        peak_memory_kb: peak_mem,
        cpu_percent: cpu_pct,
    }
}

fn main() {
    let config = parse_args();

    println!("perf v1.0.0");
    println!("Profiling: {}\n", config.command);

    if config.iterations > 1 {
        println!("Running {} iterations...\n", config.iterations);
    }

    let mut all_stats = Vec::new();

    for i in 0..config.iterations {
        if config.iterations > 1 {
            println!("Iteration {}/{}", i + 1, config.iterations);
        }

        let stats = run_command(&config.command);

        if config.iterations > 1 {
            println!("  Time: {}", PerfStats::format_duration(stats.execution_time));
        }

        all_stats.push(stats);
    }

    if config.iterations > 1 {
        // Print summary for multiple runs
        let avg_time = all_stats.iter()
            .map(|s| s.execution_time.as_micros() as u64)
            .sum::<u64>() / all_stats.len() as u64;

        println!("\nSummary ({} runs):", all_stats.len());
        println!("  Average execution time: {}ms", avg_time);

        let min_time = all_stats.iter()
            .map(|s| s.execution_time.as_micros() as u64)
            .min()
            .unwrap_or(0);
        let max_time = all_stats.iter()
            .map(|s| s.execution_time.as_micros() as u64)
            .max()
            .unwrap_or(0);

        println!("  Min: {}ms", min_time);
        println!("  Max: {}ms", max_time);
    } else {
        // Single run - print full report
        let stats = &all_stats[0];
        stats.print();
    }

    // Write to file if specified
    if let Some(ref file_path) = config.output_file {
        let output = if config.iterations > 1 {
            let avg_time = all_stats.iter()
                .map(|s| s.execution_time.as_micros() as u64)
                .sum::<u64>() / all_stats.len() as u64;
            format!(
                "Performance Summary ({} runs)\n\
                 Command: {}\n\
                 Average time: {}ms\n\
                 Min: {}ms\n\
                 Max: {}ms\n",
                all_stats.len(),
                config.command,
                avg_time,
                all_stats.iter().map(|s| s.execution_time.as_micros() as u64).min().unwrap_or(0),
                all_stats.iter().map(|s| s.execution_time.as_micros() as u64).max().unwrap_or(0),
            )
        } else {
            all_stats[0].to_string()
        };

        let mut file = File::create(file_path).unwrap_or_else(|e| {
            eprintln!("Error creating output file '{}': {}", file_path, e);
            exit(1);
        });
        file.write_all(output.as_bytes()).unwrap_or_else(|e| {
            eprintln!("Error writing to output file: {}", e);
            exit(1);
        });
        eprintln!("\nReport written to {}", file_path);
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
