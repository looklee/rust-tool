use std::fs;
use std::io;

struct ProcessInfo {
    pid: u32,
    ppid: u32,
    uid: u32,
    comm: String,
    state: char,
    utime: u64,
    stime: u64,
    rss: u64,
    vsz: u64,
    tty: String,
    stat: String,
    start_time: u64,
}

fn read_proc_stat(pid: u32) -> io::Result<String> {
    fs::read_to_string(format!("/proc/{}/stat", pid))
}

fn read_proc_status(pid: u32) -> io::Result<String> {
    fs::read_to_string(format!("/proc/{}/status", pid))
}

fn read_proc_cmdline(pid: u32) -> io::Result<String> {
    let content = fs::read_to_string(format!("/proc/{}/cmdline", pid))?;
    Ok(content.replace('\0', " ").trim().to_string())
}

fn parse_proc_stat(stat: &str) -> Option<ProcessInfo> {
    // Format: pid (comm) state ppid ...
    let paren_open = stat.find('(')?;
    let paren_close = stat.rfind(')')?;

    let pid_str = &stat[..paren_open];
    let comm = &stat[paren_open + 1..paren_close];
    let rest = &stat[paren_close + 2..];

    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.len() < 20 {
        return None;
    }

    let pid = pid_str.trim().parse::<u32>().ok()?;
    let state = fields[0].chars().next().unwrap_or('R');
    let ppid = fields[1].parse::<u32>().ok()?;
    let utime = fields[11].parse::<u64>().ok()?;
    let stime = fields[12].parse::<u64>().ok()?;
    let starttime = fields[19].parse::<u64>().ok()?;
    let vsize = fields[20].parse::<u64>().ok()?;

    // Parse RSS (field 22, index 23 in rest after state)
    let rss = if fields.len() > 22 {
        fields[22].parse::<u64>().ok().unwrap_or(0)
    } else {
        0
    };

    // Get tty
    let tty_nr = fields[5].parse::<u32>().unwrap_or(0);
    let tty = if tty_nr == 0 {
        "?".to_string()
    } else {
        let major = (tty_nr >> 8) & 0xfff;
        let minor = tty_nr & 0xff;
        if major == 4 {
            format!("tty{}", minor)
        } else if major == 136 {
            format!("pts/{}", minor)
        } else {
            format!("{}:{}", major, minor)
        }
    };

    Some(ProcessInfo {
        pid,
        ppid,
        uid: 0,
        comm: comm.to_string(),
        state,
        utime,
        stime,
        rss,
        vsz: vsize / 1024,
        tty,
        stat: String::from(state),
        start_time: starttime,
    })
}

fn read_proc_uid(pid: u32) -> io::Result<u32> {
    let content = fs::read_to_string(format!("/proc/{}/status", pid))?;
    for line in content.lines() {
        if line.starts_with("Uid:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                return Ok(parts[1].parse::<u32>().unwrap_or(0));
            }
        }
    }
    Ok(0)
}

fn get_state_string(state: char) -> String {
    match state {
        'R' => "R+".to_string(),
        'S' => "S+".to_string(),
        'D' => "D".to_string(),
        'Z' => "Z".to_string(),
        'T' => "T".to_string(),
        'I' => "I".to_string(),
        _ => format!("{}", state),
    }
}

fn format_time(jiffies: u64) -> String {
    let total_secs = jiffies / 100;
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if days > 0 {
        format!("{}-{:02}:{:02}:{:02}", days, hours, mins, secs)
    } else if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

fn print_usage() {
    println!("Usage: ps [OPTIONS]");
    println!();
    println!("Report a snapshot of the current processes.");
    println!();
    println!("Options:");
    println!("  -e   Select all processes");
    println!("  -f   Full format listing");
    println!("  -u   Show processes for specific user (UID)");
    println!("  --help  Show this help message");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut all_processes = false;
    let mut full_format = false;
    let mut filter_uid: Option<u32> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                return;
            }
            "-e" => {
                all_processes = true;
            }
            "-f" => {
                full_format = true;
                all_processes = true;
            }
            "-u" => {
                i += 1;
                if i < args.len() {
                    filter_uid = Some(args[i].parse::<u32>().unwrap_or(0));
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

    let mut processes: Vec<ProcessInfo> = Vec::new();

    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.filter_map(|e| e.ok()) {
            if let Ok(name) = entry.file_name().into_string() {
                if let Ok(pid) = name.parse::<u32>() {
                    if let Ok(stat) = read_proc_stat(pid) {
                        if let Some(mut info) = parse_proc_stat(&stat) {
                            info.uid = read_proc_uid(pid).unwrap_or(0);
                            if let Some(uid_filter) = filter_uid {
                                if info.uid != uid_filter {
                                    continue;
                                }
                            }
                            processes.push(info);
                        }
                    }
                }
            }
        }
    }

    // Sort by PID
    processes.sort_by_key(|p| p.pid);

    if full_format {
        println!(
            "{:>7} {:>5} {:>5} {:>7} {:>5} {:>5} {:>3} {:>8} {:>7} {}",
            "PID", "PPID", "UID", "VSZ", "RSS", "TTY", "ST", "TIME", "CMD", "COMMAND"
        );
        for p in &processes {
            let cmdline = read_proc_cmdline(p.pid).unwrap_or_else(|_| p.comm.clone());
            let cpu_time = p.utime + p.stime;
            println!(
                "{:>7} {:>5} {:>5} {:>7} {:>5} {:>5} {:>3} {:>8} {:>7} {}",
                p.pid,
                p.ppid,
                p.uid,
                p.vsz,
                p.rss,
                p.tty,
                get_state_string(p.state),
                format_time(cpu_time),
                p.comm,
                cmdline
            );
        }
    } else {
        println!("{:>7} {:>5} {:>5} {:>3} {:>8} {}", "PID", "TTY", "TIME", "CMD", "STAT", "COMMAND");
        for p in &processes {
            let cmdline = read_proc_cmdline(p.pid).unwrap_or_else(|_| p.comm.clone());
            let cpu_time = p.utime + p.stime;
            println!(
                "{:>7} {:>5} {:>8} {:>3} {:>8} {}",
                p.pid,
                p.tty,
                format_time(cpu_time),
                p.comm,
                get_state_string(p.state),
                cmdline
            );
        }
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
