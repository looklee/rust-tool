use std::fs;
use std::io;
use std::path::Path;

struct DiskInfo {
    device: String,
    mount_point: String,
    fs_type: String,
    total_blocks: u64,
    free_blocks: u64,
    avail_blocks: u64,
    used_blocks: u64,
    block_size: u64,
    use_percent: f64,
}

fn read_proc_mounts() -> io::Result<Vec<(String, String, String)>> {
    let content = fs::read_to_string("/proc/mounts")?;
    let mut mounts = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            mounts.push((
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].to_string(),
            ));
        }
    }
    Ok(mounts)
}

fn is_virtual_fs(fs_type: &str) -> bool {
    matches!(
        fs_type,
        "proc"
            | "sysfs"
            | "devtmpfs"
            | "devpts"
            | "tmpfs"
            | "cgroup"
            | "cgroup2"
            | "pstore"
            | "debugfs"
            | "tracefs"
            | "securityfs"
            | "hugetlbfs"
            | "mqueue"
            | "binfmt_misc"
            | "autofs"
            | "configfs"
            | "fusectl"
            | "rpc_pipefs"
            | "nfsd"
    )
}

fn get_disk_info(mount_point: &str, device: &str, fs_type: &str) -> Option<DiskInfo> {
    let path = Path::new(mount_point);
    let c_path = std::ffi::CString::new(path.to_str()?).ok()?;
    
    let mut statvfs: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut statvfs) } != 0 {
        return None;
    }

    let block_size = statvfs.f_bsize as u64;
    let total_blocks = statvfs.f_blocks;
    let free_blocks = statvfs.f_bfree;
    let avail_blocks = statvfs.f_bavail;
    let used_blocks = total_blocks.saturating_sub(free_blocks);

    let use_percent = if total_blocks > 0 {
        (used_blocks as f64 / total_blocks as f64) * 100.0
    } else {
        0.0
    };

    Some(DiskInfo {
        device: device.to_string(),
        mount_point: mount_point.to_string(),
        fs_type: fs_type.to_string(),
        total_blocks,
        free_blocks,
        avail_blocks,
        used_blocks,
        block_size,
        use_percent,
    })
}

fn format_size(bytes: u64, human_readable: bool) -> String {
    if !human_readable {
        // 1K blocks
        return format!("{}", bytes / 1024);
    }

    if bytes >= 1_073_741_824 {
        format!("{:.1}G", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1}M", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

fn print_usage() {
    println!("Usage: df [OPTIONS] [FILE...]");
    println!();
    println!("Report file system disk space usage.");
    println!();
    println!("Options:");
    println!("  -h    Human-readable output (show sizes in K, M, G)");
    println!("  -T    Show file system type");
    println!("  --help  Show this help message");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut human_readable = false;
    let mut show_type = false;

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
            "-T" => {
                show_type = true;
            }
            _ => {
                // Could be a path argument - for simplicity, just show all
                if args[i].starts_with('-') {
                    eprintln!("Unknown option: {}", args[i]);
                    print_usage();
                    return;
                }
            }
        }
        i += 1;
    }

    let mounts = match read_proc_mounts() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error reading /proc/mounts: {}", e);
            return;
        }
    };

    let mut disks: Vec<DiskInfo> = Vec::new();
    let mut seen_mounts: Vec<String> = Vec::new();

    for (device, mount_point, fs_type) in mounts {
        if seen_mounts.contains(&mount_point) {
            continue;
        }
        seen_mounts.push(mount_point.clone());

        // Skip virtual filesystems unless -T is specified
        if is_virtual_fs(&fs_type) && !show_type {
            continue;
        }

        // Only include block device mounts
        if !device.starts_with('/') && !device.starts_with("dev/") {
            continue;
        }

        if let Some(info) = get_disk_info(&mount_point, &device, &fs_type) {
            disks.push(info);
        }
    }

    // Print header
    if show_type {
        print!("{:<30} ", "Filesystem");
        print!("{:<8} ", "Type");
        if human_readable {
            print!("{:<10} ", "Size");
            print!("{:<10} ", "Used");
            print!("{:<10} ", "Avail");
        } else {
            print!("{:>10} ", "1K-blocks");
            print!("{:>10} ", "Used");
            print!("{:>10} ", "Available");
        }
        print!("{:>6} ", "Use%");
        println!("{:<20}", "Mounted on");
    } else {
        print!("{:<30} ", "Filesystem");
        if human_readable {
            print!("{:<10} ", "Size");
            print!("{:<10} ", "Used");
            print!("{:<10} ", "Avail");
        } else {
            print!("{:>10} ", "1K-blocks");
            print!("{:>10} ", "Used");
            print!("{:>10} ", "Available");
        }
        print!("{:>6} ", "Use%");
        println!("{:<20}", "Mounted on");
    }

    // Print disk info
    for disk in &disks {
        print!("{:<30} ", disk.device);
        if show_type {
            print!("{:<8} ", disk.fs_type);
        }
        let total_bytes = disk.total_blocks * disk.block_size;
        let used_bytes = disk.used_blocks * disk.block_size;
        let avail_bytes = disk.avail_blocks * disk.block_size;

        if human_readable {
            print!("{:<10} ", format_size(total_bytes, true));
            print!("{:<10} ", format_size(used_bytes, true));
            print!("{:<10} ", format_size(avail_bytes, true));
        } else {
            print!("{:>10} ", total_bytes / 1024);
            print!("{:>10} ", used_bytes / 1024);
            print!("{:>10} ", avail_bytes / 1024);
        }
        print!("{:>5.1}% ", disk.use_percent);
        println!("{:<20}", disk.mount_point);
    }
}
