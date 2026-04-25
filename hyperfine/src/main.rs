use std::env;
use std::process::{Command, exit};
use std::time::{Duration, Instant};

struct Config {
    runs: usize,
    warmup: usize,
    ignore_failures: bool,
    commands: Vec<String>,
}

struct BenchmarkResult {
    times: Vec<Duration>,
    command: String,
}

impl BenchmarkResult {
    fn min(&self) -> Duration {
        *self.times.iter().min().unwrap()
    }

    fn max(&self) -> Duration {
        *self.times.iter().max().unwrap()
    }

    fn mean(&self) -> Duration {
        let sum: Duration = self.times.iter().sum();
        sum / self.times.len() as u32
    }

    fn median(&self) -> Duration {
        let mut sorted = self.times.clone();
        sorted.sort();
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2
        } else {
            sorted[mid]
        }
    }

    fn stddev(&self) -> Duration {
        if self.times.len() < 2 {
            return Duration::ZERO;
        }
        let mean_nanos = self.mean().as_nanos() as f64;
        let variance: f64 = self.times.iter()
            .map(|t| {
                let diff = t.as_nanos() as f64 - mean_nanos;
                diff * diff
            })
            .sum::<f64>() / (self.times.len() - 1) as f64;
        Duration::from_nanos(variance.sqrt() as u64)
    }

    fn format_duration(d: Duration) -> String {
        let millis = d.as_micros() as f64 / 1000.0;
        if millis >= 1000.0 {
            format!("{:.3}s", millis / 1000.0)
        } else {
            format!("{:.3}ms", millis)
        }
    }

    fn print_summary(&self) {
        println!("Benchmark {}:", self.command);
        println!("  Time (mean ± σ):    {} ± {}",
            Self::format_duration(self.mean()),
            Self::format_duration(self.stddev()));
        println!("  Range (min … max):  {} … {}",
            Self::format_duration(self.min()),
            Self::format_duration(self.max()));
        println!("  Median:             {}",
            Self::format_duration(self.median()));
        println!("  Runs:               {}", self.times.len());
    }
}

fn run_command(cmd: &str) -> Result<Duration, String> {
    let start = Instant::now();
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?;

    let elapsed = start.elapsed();

    if output.success() {
        Ok(elapsed)
    } else {
        Err(format!("Command '{}' failed with exit code", cmd))
    }
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut runs = 10;
    let mut warmup = 0;
    let mut ignore_failures = false;
    let mut commands = Vec::new();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-r" | "--runs" => {
                i += 1;
                if i < args.len() {
                    runs = args[i].parse().unwrap_or_else(|_| {
                        eprintln!("Error: Invalid number of runs");
                        exit(1);
                    });
                }
            }
            "-w" | "--warmup" => {
                i += 1;
                if i < args.len() {
                    warmup = args[i].parse().unwrap_or_else(|_| {
                        eprintln!("Error: Invalid number of warmup runs");
                        exit(1);
                    });
                }
            }
            "-i" | "--ignore-failures" => {
                ignore_failures = true;
            }
            "-h" | "--help" => {
                print_help();
                exit(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("Error: Unknown option '{}'", arg);
                exit(1);
            }
            cmd => {
                commands.push(cmd.to_string());
            }
        }
        i += 1;
    }

    if commands.is_empty() {
        eprintln!("Error: At least one command is required");
        print_help();
        exit(1);
    }

    Config {
        runs,
        warmup,
        ignore_failures,
        commands,
    }
}

fn print_help() {
    println!("hyperfine v1.0.0 - Command-line benchmarking tool");
    println!();
    println!("USAGE:");
    println!("    hyperfine [OPTIONS] <command> [<command>...]");
    println!();
    println!("OPTIONS:");
    println!("    -r, --runs <N>            Number of runs (default: 10)");
    println!("    -w, --warmup <N>          Number of warmup runs (default: 0)");
    println!("    -i, --ignore-failures     Ignore non-zero exit codes");
    println!("    -h, --help                Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    hyperfine 'sleep 0.1'");
    println!("    hyperfine -r 20 -w 3 'ls -la' 'ls -la /tmp'");
    println!("    hyperfine -i 'false || true'");
    println!();
    println!("DESCRIPTION:");
    println!("    Benchmark one or more commands by running them multiple times");
    println!("    and displaying statistics about execution time.");
}

fn run_benchmark(cmd: &str, config: &Config) -> BenchmarkResult {
    // Warmup runs
    for i in 0..config.warmup {
        print!("\r  Warmup run {}/{}", i + 1, config.warmup);
        let _ = run_command(cmd);
    }
    println!();

    // Actual benchmark runs
    let mut times = Vec::new();
    for i in 0..config.runs {
        print!("\r  Running {}/{}", i + 1, config.runs);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        match run_command(cmd) {
            Ok(elapsed) => times.push(elapsed),
            Err(_) if config.ignore_failures => {
                // Still record the time even if command failed
                let start = Instant::now();
                let _ = run_command(cmd);
                times.push(start.elapsed());
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                exit(1);
            }
        }
    }
    println!();

    BenchmarkResult {
        times,
        command: cmd.to_string(),
    }
}

fn compare_results(results: &[BenchmarkResult]) {
    if results.len() < 2 {
        return;
    }

    println!("\nComparison:");
    let mut sorted: Vec<_> = results.iter().enumerate().collect();
    sorted.sort_by_key(|(_, r)| r.mean());

    let fastest_mean = sorted[0].1.mean().as_nanos();

    for (idx, (_orig_idx, result)) in sorted.iter().enumerate() {
        let ratio = if fastest_mean > 0 {
            result.mean().as_nanos() as f64 / fastest_mean as f64
        } else {
            1.0
        };

        if idx == 0 {
            println!("  {:.1}  {} (fastest)", ratio, result.command);
        } else {
            println!("  {:.2}  {} ({:.2}x slower)", ratio, result.command, ratio);
        }
    }
}

fn main() {
    let config = parse_args();

    println!("hyperfine v1.0.0");
    println!("Benchmarking {} command(s) with {} runs each (+ {} warmup)\n",
        config.commands.len(), config.runs, config.warmup);

    let mut results = Vec::new();

    for cmd in &config.commands {
        println!("Command: {}", cmd);
        let result = run_benchmark(cmd, &config);
        result.print_summary();
        println!();
        results.push(result);
    }

    compare_results(&results);
}
