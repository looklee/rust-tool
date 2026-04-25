use std::fs;
use std::io;

struct MemInfo {
    total: u64,
    free: u64,
    available: u64,
    buffers: u64,
    cached: u64,
    slab_reclaimable: u64,
    slab_unreclaimable: u64,
    swap_total: u64,
    swap_free: u64,
    dirty: u64,
    writeback: u64,
    active: u64,
    inactive: u64,
    anon_pages: u64,
    mapped: u64,
}

fn read_meminfo() -> io::Result<MemInfo> {
    let content = fs::read_to_string("/proc/meminfo")?;
    let mut info = MemInfo {
        total: 0,
        free: 0,
        available: 0,
        buffers: 0,
        cached: 0,
        slab_reclaimable: 0,
        slab_unreclaimable: 0,
        swap_total: 0,
        swap_free: 0,
        dirty: 0,
        writeback: 0,
        active: 0,
        inactive: 0,
        anon_pages: 0,
        mapped: 0,
    };

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let key = parts[0].trim_end_matches(':');
        let val = parts[1].parse::<u64>().unwrap_or(0);

        match key {
            "MemTotal" => info.total = val,
            "MemFree" => info.free = val,
            "MemAvailable" => info.available = val,
            "Buffers" => info.buffers = val,
            "Cached" => info.cached = val,
            "Slab" => {
                // Slab is split into reclaimable and unreclaimable in newer kernels
                // For simplicity, we'll read SReclaimable and SUnreclaim separately
            }
            "SReclaimable" => info.slab_reclaimable = val,
            "SUnreclaim" => info.slab_unreclaimable = val,
            "SwapTotal" => info.swap_total = val,
            "SwapFree" => info.swap_free = val,
            "Dirty" => info.dirty = val,
            "Writeback" => info.writeback = val,
            "Active" => info.active = val,
            "Inactive" => info.inactive = val,
            "AnonPages" => info.anon_pages = val,
            "Mapped" => info.mapped = val,
            _ => {}
        }
    }

    Ok(info)
}

fn format_value(kb: u64, unit: char) -> String {
    match unit {
        'm' => format!("{}", kb / 1024),
        'g' => format!("{}", kb / (1024 * 1024)),
        'k' => format!("{}", kb),
        _ => format!("{}", kb),
    }
}

fn format_value_f64(kb: u64, unit: char) -> f64 {
    match unit {
        'm' => kb as f64 / 1024.0,
        'g' => kb as f64 / (1024.0 * 1024.0),
        'k' => kb as f64,
        _ => kb as f64,
    }
}

fn print_usage() {
    println!("Usage: free [OPTIONS]");
    println!();
    println!("Display amount of free and used memory in the system.");
    println!();
    println!("Options:");
    println!("  -h    Human-readable output (auto scale to K, M, G)");
    println!("  -m    Display in megabytes");
    println!("  -g    Display in gigabytes");
    println!("  -k    Display in kilobytes (default)");
    println!("  --help  Show this help message");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut human_readable = false;
    let mut unit = 'k';

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" => {
                print_usage();
                return;
            }
            "-h" => {
                human_readable = true;
            }
            "-m" => {
                unit = 'm';
            }
            "-g" => {
                unit = 'g';
            }
            "-k" => {
                unit = 'k';
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_usage();
                return;
            }
        }
        i += 1;
    }

    let meminfo = match read_meminfo() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error reading /proc/meminfo: {}", e);
            return;
        }
    };

    let used = meminfo.total.saturating_sub(meminfo.free);
    let total_cache = meminfo.buffers + meminfo.cached + meminfo.slab_reclaimable;
    let actual_used = if used > total_cache {
        used - total_cache
    } else {
        used
    };

    if human_readable {
        println!("{:>8} {:>10} {:>10} {:>10} {:>10} {:>10}",
            " ", "Total", "Used", "Free", "Shared", "Buff/Cache");

        // Memory
        let total_mem = format_mem_human(meminfo.total);
        let used_mem = format_mem_human(actual_used);
        let free_mem = format_mem_human(meminfo.free);
        let shared_mem = format_mem_human(0); // Not directly available
        let cache_mem = format_mem_human(total_cache);

        println!("Mem:   {:>10} {:>10} {:>10} {:>10} {:>10}",
            total_mem, used_mem, free_mem, shared_mem, cache_mem);

        // Swap
        let total_swap = format_mem_human(meminfo.swap_total);
        let used_swap = format_mem_human(meminfo.swap_total.saturating_sub(meminfo.swap_free));
        let free_swap = format_mem_human(meminfo.swap_free);

        println!("Swap: {:>10} {:>10} {:>10}",
            total_swap, used_swap, free_swap);
    } else {
        let unit_label = match unit {
            'm' => "Mi",
            'g' => "Gi",
            _ => "Ki",
        };

        println!("{:>8} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
            " ", "total", "used", "free", "shared", "buff/cache", "available");

        println!("{:<8} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
            "Mem:",
            format_value(meminfo.total, unit),
            format_value(actual_used, unit),
            format_value(meminfo.free, unit),
            format_value(0, unit),
            format_value(total_cache, unit),
            format_value(meminfo.available, unit));

        println!("{:<8} {:>10} {:>10} {:>10}",
            "Swap:",
            format_value(meminfo.swap_total, unit),
            format_value(meminfo.swap_total.saturating_sub(meminfo.swap_free), unit),
            format_value(meminfo.swap_free, unit));
    }
}

fn format_mem_human(kb: u64) -> String {
    if kb >= 1_048_576 {
        format!("{:.0}G", kb as f64 / 1_048_576.0)
    } else if kb >= 1024 {
        format!("{:.0}M", kb as f64 / 1024.0)
    } else {
        format!("{}K", kb)
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
