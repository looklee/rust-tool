use std::fs;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

struct ProcessInfo {
    pid: u32,
    comm: String,
    cpu_percent: f64,
    mem_percent: f64,
    vss: u64,
    rss: u64,
    state: char,
    utime: u64,
    stime: u64,
}

struct SystemStats {
    total_mem: u64,
    free_mem: u64,
    used_mem: u64,
    uptime: f64,
    tasks_total: u32,
    tasks_running: u32,
    load_avg_1: f64,
    load_avg_5: f64,
    load_avg_15: f64,
}

fn read_proc_meminfo() -> Vec<(String, u64)> {
    let content = match fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut stats = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(val) = parts[1].parse::<u64>() {
                stats.push((parts[0].trim_end_matches(':').to_string(), val));
            }
        }
    }
    stats
}

fn read_proc_stat() -> io::Result<Vec<u64>> {
    let content = fs::read_to_string("/proc/stat")?;
    let line = content.lines().next().unwrap_or("");
    let parts: Vec<&str> = line.split_whitespace().skip(1).collect();
    Ok(parts.iter().filter_map(|s| s.parse::<u64>().ok()).collect::<Vec<_>>())
}

fn read_proc_loadavg() -> io::Result<(f64, f64, f64)> {
    let content = fs::read_to_string("/proc/loadavg")?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    let l1 = parts[0].parse::<f64>().unwrap_or(0.0);
    let l5 = parts[1].parse::<f64>().unwrap_or(0.0);
    let l15 = parts[2].parse::<f64>().unwrap_or(0.0);
    Ok((l1, l5, l15))
}

fn read_proc_uptime() -> f64 {
    fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|content| {
            let parts: Vec<&str> = content.split_whitespace().collect();
            parts[0].parse::<f64>().ok()
        })
        .unwrap_or(0.0)
}

fn get_system_stats() -> SystemStats {
    let meminfo = read_proc_meminfo();
    let mut total_mem = 0u64;
    let mut free_mem = 0u64;
    let mut available_mem = 0u64;

    for (key, val) in &meminfo {
        match key.as_str() {
            "MemTotal" => total_mem = *val,
            "MemFree" => free_mem = *val,
            "MemAvailable" => available_mem = *val,
            _ => {}
        }
    }

    let used_mem = if available_mem > 0 {
        total_mem.saturating_sub(available_mem)
    } else {
        total_mem.saturating_sub(free_mem)
    };

    let (l1, l5, l15) = read_proc_loadavg().unwrap_or((0.0, 0.0, 0.0));
    let uptime = read_proc_uptime();

    let mut tasks_total = 0u32;
    let mut tasks_running = 0u32;
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.filter_map(|e| e.ok()) {
            if let Ok(name) = entry.file_name().into_string() {
                if let Ok(pid) = name.parse::<u32>() {
                    tasks_total += 1;
                    if let Ok(status) = fs::read_to_string(format!("/proc/{}/status", pid)) {
                        for line in status.lines() {
                            if line.starts_with("State:") {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if parts.len() > 1 && parts[1] == "R" {
                                    tasks_running += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    SystemStats {
        total_mem,
        free_mem,
        used_mem,
        uptime,
        tasks_total,
        tasks_running,
        load_avg_1: l1,
        load_avg_5: l5,
        load_avg_15: l15,
    }
}

fn get_process_info(pid: u32, prev_times: &mut std::collections::HashMap<u32, (u64, u64)>) -> Option<ProcessInfo> {
    let status_path = format!("/proc/{}/status", pid);
    let stat_path = format!("/proc/{}/stat", pid);

    let status = fs::read_to_string(&status_path).ok()?;
    let stat = fs::read_to_string(&stat_path).ok()?;

    let mut comm = String::new();
    let mut mem_percent = 0.0f64;
    let mut vss = 0u64;
    let mut rss = 0u64;
    let mut state = 'R';

    for line in status.lines() {
        if line.starts_with("Name:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                comm = parts[1..].join(" ");
            }
        } else if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                rss = parts[1].parse::<u64>().unwrap_or(0);
            }
        } else if line.starts_with("VmSize:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                vss = parts[1].parse::<u64>().unwrap_or(0);
            }
        } else if line.starts_with("VmPeak:") {
            // skip
        }
    }

    // Parse state from status
    for line in status.lines() {
        if line.starts_with("State:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                state = parts[1].chars().next().unwrap_or('R');
            }
        }
    }

    // Parse stat for CPU times
    let utime;
    let stime;
    {
        let stat_parts: Vec<&str> = stat.splitn(2, ')').collect();
        if stat_parts.len() < 2 {
            return None;
        }
        let after_paren = stat_parts[1];
        let fields: Vec<&str> = after_paren.split_whitespace().collect();
        if fields.len() < 14 {
            return None;
        }
        utime = fields[11].parse::<u64>().unwrap_or(0);
        stime = fields[12].parse::<u64>().unwrap_or(0);
    }

    // Calculate CPU percent
    let total_time = utime + stime;
    let cpu_percent = if let Some(prev) = prev_times.get(&pid) {
        let prev_total = prev.0 + prev.1;
        let delta_total = total_time.saturating_sub(prev_total);
        let num_cpus = num_cpus();
        if delta_total > 0 {
            (delta_total as f64 / 100.0) / (num_cpus as f64) * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Calculate memory percent
    let local_meminfo = read_proc_meminfo();
    for (key, val) in &local_meminfo {
        if key == "MemTotal" && *val > 0 {
            mem_percent = (rss as f64 / *val as f64) * 100.0;
        }
    }

    prev_times.insert(pid, (utime, stime));

    Some(ProcessInfo {
        pid,
        comm,
        cpu_percent,
        mem_percent,
        vss,
        rss,
        state,
        utime,
        stime,
    })
}

fn num_cpus() -> usize {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        content.lines().filter(|l| l.starts_with("processor")).count()
    } else {
        1
    }
}

fn format_size(kb: u64) -> String {
    if kb >= 1_048_576 {
        format!("{:.1}g", kb as f64 / 1_048_576.0)
    } else if kb >= 1024 {
        format!("{:.1}m", kb as f64 / 1024.0)
    } else {
        format!("{}k", kb)
    }
}

fn format_time(jiffies: u64) -> String {
    let total_secs = jiffies / 100;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{}h{:02}m", hours, mins)
    } else if mins > 0 {
        format!("{}m{:02}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

fn display_header(stats: &SystemStats) {
    let days = (stats.uptime / 86400.0) as u32;
    let hours = ((stats.uptime % 86400.0) / 3600.0) as u32;
    let mins = ((stats.uptime % 3600.0) / 60.0) as u32;
    let uptime_str = if days > 0 {
        format!("{} day{}, {:02}:{:02}", days, if days > 1 { "s" } else { "" }, hours, mins)
    } else {
        format!("{:02}:{:02}", hours, mins)
    };

    println!(
        "top - {} up {}, {} users,  load average: {:.2}, {:.2}, {:.2}",
        chrono_local_time(),
        uptime_str,
        1,
        stats.load_avg_1,
        stats.load_avg_5,
        stats.load_avg_15
    );
    println!(
        "Tasks: {:>4} total, {:>3} running, {:>4} sleeping, {:>3} stopped, {:>3} zombie",
        stats.tasks_total,
        stats.tasks_running,
        stats.tasks_total.saturating_sub(stats.tasks_running),
        0,
        0
    );
    println!(
        "%Cpu(s):  0.0 us,  0.0 sy,  0.0 ni,100.0 id,  0.0 wa,  0.0 hi,  0.0 si,  0.0 st"
    );
    println!(
        "MiB Mem : {:>8.1} total, {:>8.1} free, {:>8.1} used, {:>8.1} buff/cache",
        stats.total_mem as f64 / 1024.0,
        stats.free_mem as f64 / 1024.0,
        stats.used_mem as f64 / 1024.0,
        0.0
    );
    println!();
    println!("Note: /proc/stat not available, CPU% shows 0.0");
}

fn chrono_local_time() -> String {
    // Simple time formatting without external crate
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = ((secs / 3600) + 0) % 24; // UTC offset would need libc
    let mins = (secs / 60) % 60;
    format!("{:02}:{:02}:{:02}", hours % 24, mins, secs % 60)
}

fn print_usage() {
    println!("Usage: top [OPTIONS]");
    println!();
    println!("A simplified process viewer that reads from /proc filesystem.");
    println!();
    println!("Options:");
    println!("  -n NUM   Exit after NUM iterations");
    println!("  -d SEC   Delay between updates (default: 2 seconds)");
    println!("  -p PID   Monitor specific PID");
    println!("  --help   Show this help message");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut iterations: Option<u32> = None;
    let mut delay: u64 = 2;
    let mut watch_pid: Option<u32> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                return;
            }
            "-n" => {
                i += 1;
                if i < args.len() {
                    iterations = Some(args[i].parse::<u32>().unwrap_or(1));
                }
            }
            "-d" => {
                i += 1;
                if i < args.len() {
                    delay = args[i].parse::<u64>().unwrap_or(2);
                }
            }
            "-p" => {
                i += 1;
                if i < args.len() {
                    watch_pid = Some(args[i].parse::<u32>().unwrap_or(0));
                }
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_usage();
                return;
            }
        }
        i += 1;
    }

    let mut prev_times: std::collections::HashMap<u32, (u64, u64)> = std::collections::HashMap::new();
    let mut iter = 0u32;

    loop {
        iter += 1;

        // Get system stats
        let stats = get_system_stats();

        // Clear screen and print header
        print!("\x1b[2J\x1b[H");
        display_header(&stats);

        // Collect process info
        let mut processes: Vec<ProcessInfo> = Vec::new();
        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(name) = entry.file_name().into_string() {
                    if let Ok(pid) = name.parse::<u32>() {
                        if let Some(wp) = watch_pid {
                            if pid != wp {
                                continue;
                            }
                        }
                        if let Some(info) = get_process_info(pid, &mut prev_times) {
                            processes.push(info);
                        }
                    }
                }
            }
        }

        // Sort by CPU usage descending
        processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));

        // Limit to top 20
        processes.truncate(20);

        // Print header
        println!(
            "  PID USER      PR  NI    VIRT    RES %CPU %MEM   TIME+ COMMAND"
        );

        // Print processes
        for p in &processes {
            let state_char = match p.state {
                'R' => 'R',
                'S' => 'S',
                'D' => 'D',
                'Z' => 'Z',
                'T' => 'T',
                _ => 'S',
            };
            println!(
                "{:>6} {:<9} {:>2} {:>2} {:>7} {:>6} {:>4.1} {:>4.1} {} {}",
                p.pid,
                "root",
                20,
                0,
                format_size(p.vss),
                format_size(p.rss),
                p.cpu_percent,
                p.mem_percent,
                format_time(p.utime + p.stime),
                p.comm
            );
        }

        println!();

        if let Some(max_iter) = iterations {
            if iter >= max_iter {
                break;
            }
        }

        thread::sleep(Duration::from_secs(delay));
    }
}
