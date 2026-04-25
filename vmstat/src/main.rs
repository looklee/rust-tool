use std::fs;
use std::io;
use std::thread;
use std::time::Duration;

struct VmStats {
    page_in: u64,
    page_out: u64,
    swap_in: u64,
    swap_out: u64,
    io_in: u64,
    io_out: u64,
    interrupts: u64,
    context_switches: u64,
    cpu_user: u64,
    cpu_nice: u64,
    cpu_system: u64,
    cpu_idle: u64,
    cpu_iowait: u64,
    cpu_irq: u64,
    cpu_softirq: u64,
    cpu_steal: u64,
    cpu_guest: u64,
}

struct MemStats {
    active: u64,
    inactive: u64,
    dirty: u64,
    writeback: u64,
    anon_pages: u64,
    mapped: u64,
    slab_reclaimable: u64,
    slab_unreclaimable: u64,
    page_tables: u64,
    kernel_stack: u64,
    commit_limit: u64,
    committed_as: u64,
    vmalloc_total: u64,
    vmalloc_used: u64,
    cma_total: u64,
}

struct DiskStats {
    reads_completed: u64,
    reads_merged: u64,
    sectors_read: u64,
    time_reading: u64,
    writes_completed: u64,
    writes_merged: u64,
    sectors_written: u64,
    time_writing: u64,
    io_in_progress: u64,
    time_io: u64,
    weighted_time_io: u64,
}

fn read_proc_stat() -> io::Result<VmStats> {
    let content = fs::read_to_string("/proc/stat")?;
    let mut stats = VmStats {
        page_in: 0,
        page_out: 0,
        swap_in: 0,
        swap_out: 0,
        io_in: 0,
        io_out: 0,
        interrupts: 0,
        context_switches: 0,
        cpu_user: 0,
        cpu_nice: 0,
        cpu_system: 0,
        cpu_idle: 0,
        cpu_iowait: 0,
        cpu_irq: 0,
        cpu_softirq: 0,
        cpu_steal: 0,
        cpu_guest: 0,
    };

    for line in content.lines() {
        if line.starts_with("cpu ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 { stats.cpu_user = parts[1].parse().unwrap_or(0); }
            if parts.len() > 2 { stats.cpu_nice = parts[2].parse().unwrap_or(0); }
            if parts.len() > 3 { stats.cpu_system = parts[3].parse().unwrap_or(0); }
            if parts.len() > 4 { stats.cpu_idle = parts[4].parse().unwrap_or(0); }
            if parts.len() > 5 { stats.cpu_iowait = parts[5].parse().unwrap_or(0); }
            if parts.len() > 6 { stats.cpu_irq = parts[6].parse().unwrap_or(0); }
            if parts.len() > 7 { stats.cpu_softirq = parts[7].parse().unwrap_or(0); }
            if parts.len() > 8 { stats.cpu_steal = parts[8].parse().unwrap_or(0); }
            if parts.len() > 9 { stats.cpu_guest = parts[9].parse().unwrap_or(0); }
        } else if line.starts_with("intr ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 { stats.interrupts = parts[1].parse().unwrap_or(0); }
        } else if line.starts_with("ctxt ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 { stats.context_switches = parts[1].parse().unwrap_or(0); }
        } else if line.starts_with("page_") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if line.starts_with("page_pgpgin ") && parts.len() > 1 { stats.io_in = parts[1].parse().unwrap_or(0); }
            if line.starts_with("page_pgpgout ") && parts.len() > 1 { stats.io_out = parts[1].parse().unwrap_or(0); }
        }
    }

    // Read vmstat for page and swap stats
    if let Ok(vmstat) = fs::read_to_string("/proc/vmstat") {
        for line in vmstat.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 { continue; }
            match parts[0] {
                "pgpgin" => stats.page_in = parts[1].parse().unwrap_or(0),
                "pgpgout" => stats.page_out = parts[1].parse().unwrap_or(0),
                "pswpin" => stats.swap_in = parts[1].parse().unwrap_or(0),
                "pswpout" => stats.swap_out = parts[1].parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    Ok(stats)
}

fn read_vmstat() -> io::Result<MemStats> {
    let content = fs::read_to_string("/proc/vmstat")?;
    let mut stats = MemStats {
        active: 0,
        inactive: 0,
        dirty: 0,
        writeback: 0,
        anon_pages: 0,
        mapped: 0,
        slab_reclaimable: 0,
        slab_unreclaimable: 0,
        page_tables: 0,
        kernel_stack: 0,
        commit_limit: 0,
        committed_as: 0,
        vmalloc_total: 0,
        vmalloc_used: 0,
        cma_total: 0,
    };

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        let val: u64 = parts[1].parse().unwrap_or(0);
        match parts[0] {
            "nr_active_anon" | "nr_active_file" => stats.active += val,
            "nr_inactive_anon" | "nr_inactive_file" => stats.inactive += val,
            "nr_dirty" => stats.dirty = val,
            "nr_writeback" => stats.writeback = val,
            "nr_anon_pages" => stats.anon_pages = val,
            "nr_mapped" => stats.mapped = val,
            "nr_slab_reclaimable" => stats.slab_reclaimable = val,
            "nr_slab_unreclaimable" => stats.slab_unreclaimable = val,
            "nr_page_table_pages" => stats.page_tables = val,
            "nr_kernel_stack" => stats.kernel_stack = val,
            "commit_limit" => stats.commit_limit = val,
            "committed_as" => stats.committed_as = val,
            "vmalloc_total" => stats.vmalloc_total = val,
            "vmalloc_used" => stats.vmalloc_used = val,
            "nr_free_cma" => stats.cma_total = val,
            _ => {}
        }
    }

    Ok(stats)
}

fn read_disk_stats() -> io::Result<Vec<(String, DiskStats)>> {
    let content = fs::read_to_string("/proc/diskstats")?;
    let mut disks = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 14 { continue; }

        let name = parts[2].to_string();
        let disk = DiskStats {
            reads_completed: parts[3].parse().unwrap_or(0),
            reads_merged: parts[4].parse().unwrap_or(0),
            sectors_read: parts[5].parse().unwrap_or(0),
            time_reading: parts[6].parse().unwrap_or(0),
            writes_completed: parts[7].parse().unwrap_or(0),
            writes_merged: parts[8].parse().unwrap_or(0),
            sectors_written: parts[9].parse().unwrap_or(0),
            time_writing: parts[10].parse().unwrap_or(0),
            io_in_progress: parts[11].parse().unwrap_or(0),
            time_io: parts[12].parse().unwrap_or(0),
            weighted_time_io: parts[13].parse().unwrap_or(0),
        };
        disks.push((name, disk));
    }

    Ok(disks)
}

fn format_unit(value: u64, unit: char) -> String {
    match unit {
        'k' => format!("{}", value / 1024),
        'm' => format!("{}", value / (1024 * 1024)),
        'g' => format!("{}", value / (1024 * 1024 * 1024)),
        _ => format!("{}", value),
    }
}

fn print_vmstat_header() {
    println!("{:>12} {:>12} {:>8} {:>8} {:>8} {:>8} {:>10} {:>10} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "procs", "", "memory", "", "", "", "swap", "", "io", "", "system", "", "cpu", "");
    println!("{:>4} {:>4} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>4} {:>4} {:>4} {:>4}",
        "r", "b", "swpd", "free", "buff", "cache", "si", "so", "bi", "bo", "in", "cs", "us", "sy", "id", "wa");
}

fn print_vmstat(stats: &VmStats, mem: &MemStats, unit: char) {
    let page_size = 4; // 4KB pages typically

    // Get current meminfo for free/buff/cache
    let mut free_mem = 0u64;
    let mut buff_mem = 0u64;
    let mut cache_mem = 0u64;
    let mut swap_total = 0u64;
    let mut swap_free = 0u64;

    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 { continue; }
            match parts[0].trim_end_matches(':') {
                "MemFree" => free_mem = parts[1].parse().unwrap_or(0),
                "Buffers" => buff_mem = parts[1].parse().unwrap_or(0),
                "Cached" => cache_mem = parts[1].parse().unwrap_or(0),
                "SwapTotal" => swap_total = parts[1].parse().unwrap_or(0),
                "SwapFree" => swap_free = parts[1].parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    let swap_used = swap_total.saturating_sub(swap_free);

    let num_cpus = if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        content.lines().filter(|l| l.starts_with("processor")).count().max(1)
    } else {
        1
    };

    let total_cpu = stats.cpu_user + stats.cpu_nice + stats.cpu_system + stats.cpu_idle
        + stats.cpu_iowait + stats.cpu_irq + stats.cpu_softirq + stats.cpu_steal;

    let us = if total_cpu > 0 {
        ((stats.cpu_user + stats.cpu_nice) as f64 / total_cpu as f64 * 100.0) as u64 / num_cpus as u64
    } else { 0 };

    let sy = if total_cpu > 0 {
        ((stats.cpu_system + stats.cpu_irq + stats.cpu_softirq) as f64 / total_cpu as f64 * 100.0) as u64 / num_cpus as u64
    } else { 0 };

    let id = if total_cpu > 0 {
        (stats.cpu_idle as f64 / total_cpu as f64 * 100.0) as u64 / num_cpus as u64
    } else { 0 };

    let wa = if total_cpu > 0 {
        (stats.cpu_iowait as f64 / total_cpu as f64 * 100.0) as u64 / num_cpus as u64
    } else { 0 };

    println!("{:>4} {:>4} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>4} {:>4} {:>4} {:>4}",
        0, 0,
        format_unit(swap_used, unit),
        format_unit(free_mem, unit),
        format_unit(buff_mem, unit),
        format_unit(cache_mem, unit),
        format_unit(stats.swap_in, unit),
        format_unit(stats.swap_out, unit),
        format_unit(stats.io_in, unit),
        format_unit(stats.io_out, unit),
        stats.interrupts,
        stats.context_switches,
        us, sy, id, wa
    );
}

fn print_disk_header() {
    println!();
    println!("------- --------------- --------------- ---------------");
    println!("       reads        merged   sectors read    writes");
    println!("------- --------------- --------------- ---------------");
}

fn print_disk(disk_name: &str, stats: &DiskStats) {
    println!("{:<8} {:>12} {:>12} {:>12} {:>12}",
        disk_name,
        stats.reads_completed,
        stats.reads_merged,
        stats.sectors_read,
        stats.writes_completed
    );
}

fn print_usage() {
    println!("Usage: vmstat [OPTIONS] [DELAY [COUNT]]");
    println!();
    println!("Report virtual memory statistics.");
    println!();
    println!("Options:");
    println!("  -S unit    Memory display unit: k(1024), m(1048576), g(1073741824)");
    println!("  -d         Display disk statistics");
    println!("  -p device  Display partition statistics");
    println!("  --help     Show this help message");
    println!();
    println!("If DELAY is specified, updates are repeated every DELAY seconds.");
    println!("If COUNT is specified, updates stop after COUNT iterations.");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut unit = 'k';
    let mut show_disk = false;
    let mut show_partition: Option<String> = None;
    let mut delay: Option<u64> = None;
    let mut count: Option<u32> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                return;
            }
            "-S" => {
                i += 1;
                if i < args.len() {
                    match args[i].as_str() {
                        "k" => unit = 'k',
                        "m" => unit = 'm',
                        "g" => unit = 'g',
                        _ => {
                            eprintln!("Invalid unit: {}. Use k, m, or g.", args[i]);
                            return;
                        }
                    }
                }
            }
            "-d" => {
                show_disk = true;
            }
            "-p" => {
                i += 1;
                if i < args.len() {
                    show_partition = Some(args[i].clone());
                }
            }
            _ => {
                if let Ok(d) = args[i].parse::<u64>() {
                    delay = Some(d);
                    i += 1;
                    if i < args.len() {
                        if let Ok(c) = args[i].parse::<u32>() {
                            count = Some(c);
                        }
                    }
                } else if args[i].starts_with('-') {
                    eprintln!("Unknown option: {}", args[i]);
                    print_usage();
                    return;
                }
            }
        }
        i += 1;
    }

    if show_disk {
        // Show disk stats
        match read_disk_stats() {
            Ok(disks) => {
                print_disk_header();
                for (name, stats) in &disks {
                    print_disk(name, stats);
                }
            }
            Err(e) => {
                eprintln!("Error reading disk stats: {}", e);
            }
        }
        return;
    }

    if let Some(ref part) = show_partition {
        // Show partition stats
        match read_disk_stats() {
            Ok(disks) => {
                println!("{:<8} {:>12} {:>12} {:>12} {:>12}",
                    "partition", "reads", "merged", "sectors", "writes");
                for (name, stats) in &disks {
                    if name.contains(part) || part.contains(name) {
                        println!("{:<8} {:>12} {:>12} {:>12} {:>12}",
                            name,
                            stats.reads_completed,
                            stats.reads_merged,
                            stats.sectors_read,
                            stats.writes_completed);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading partition stats: {}", e);
            }
        }
        return;
    }

    // Show vmstat
    let mut iter = 0u32;

    loop {
        iter += 1;

        if iter == 1 {
            print_vmstat_header();
        }

        match (read_proc_stat(), read_vmstat()) {
            (Ok(stats), Ok(_mem)) => {
                print_vmstat(&stats, &_mem, unit);
            }
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("Warning: {}", e);
                eprintln!("Note: Some /proc files may not be available in this environment.");
                // Print a row with zeros as fallback
                let mut free_mem = 0u64;
                let mut buff_mem = 0u64;
                let mut cache_mem = 0u64;
                let mut swap_total = 0u64;
                let mut swap_free = 0u64;

                if let Ok(content) = fs::read_to_string("/proc/meminfo") {
                    for line in content.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() < 2 { continue; }
                        match parts[0].trim_end_matches(':') {
                            "MemFree" => free_mem = parts[1].parse().unwrap_or(0),
                            "Buffers" => buff_mem = parts[1].parse().unwrap_or(0),
                            "Cached" => cache_mem = parts[1].parse().unwrap_or(0),
                            "SwapTotal" => swap_total = parts[1].parse().unwrap_or(0),
                            "SwapFree" => swap_free = parts[1].parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }

                let swap_used = swap_total.saturating_sub(swap_free);
                println!("{:>4} {:>4} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>4} {:>4} {:>4} {:>4}",
                    0, 0,
                    format_unit(swap_used, unit),
                    format_unit(free_mem, unit),
                    format_unit(buff_mem, unit),
                    format_unit(cache_mem, unit),
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0
                );
            }
        }

        if let Some(max_count) = count {
            if iter >= max_count {
                break;
            }
        }

        if let Some(d) = delay {
            thread::sleep(Duration::from_secs(d));
        } else {
            break;
        }
    }
}
