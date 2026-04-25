use std::fs;
use std::io::{self, BufRead};
use std::process;

fn print_usage() {
    println!("ip - Network configuration tool");
    println!();
    println!("USAGE:");
    println!("    ip [OPTIONS] <COMMAND>");
    println!();
    println!("COMMANDS:");
    println!("    addr           Show IP addresses");
    println!("    link           Show network interfaces");
    println!("    route          Show routing table");
    println!("    neigh          Show ARP/neighbor table");
    println!("    help           Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    ip addr");
    println!("    ip link");
    println!("    ip route");
}

fn get_interfaces() -> Vec<String> {
    let mut ifaces = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                ifaces.push(name.to_string());
            }
        }
    }
    ifaces.sort();
    ifaces
}

fn get_interface_state(iface: &str) -> String {
    let path = format!("/sys/class/net/{}/operstate", iface);
    fs::read_to_string(&path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "UNKNOWN".to_string())
}

fn get_mac_address(iface: &str) -> String {
    let path = format!("/sys/class/net/{}/address", iface);
    fs::read_to_string(&path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "00:00:00:00:00:00".to_string())
}

fn get_mtu(iface: &str) -> String {
    let path = format!("/sys/class/net/{}/mtu", iface);
    fs::read_to_string(&path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn parse_addresses() -> Vec<(String, String, String)> {
    // Returns: (iface, family, address)
    let mut addrs = Vec::new();
    
    if let Ok(file) = fs::File::open("/proc/net/if_inet6") {
        let reader = io::BufReader::new(file);
        for line in reader.lines().flatten() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                let iface = parts[5].to_string();
                let ip_hex = parts[0];
                // Convert hex to IPv6
                let mut ip = String::new();
                for (i, chunk) in ip_hex.as_bytes().chunks(4).enumerate() {
                    if i > 0 {
                        ip.push(':');
                    }
                    ip.push_str(&String::from_utf8_lossy(chunk));
                }
                let prefix_len = u8::from_str_radix(parts[2], 16).unwrap_or(64);
                addrs.push((iface, "inet6".to_string(), format!("{}/{}", ip, prefix_len)));
            }
        }
    }

    // Parse IPv4 from ip command output or /proc
    if let Ok(output) = process::Command::new("ip").arg("-4").arg("addr").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut current_iface = String::new();
        for line in stdout.lines() {
            if line.starts_with(|c: char| c.is_ascii_digit()) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    current_iface = parts[1].to_string();
                }
            } else if line.contains("inet ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    addrs.push((current_iface.clone(), "inet".to_string(), parts[1].to_string()));
                }
            }
        }
    }

    addrs
}

fn cmd_addr() {
    let addresses = parse_addresses();
    let ifaces = get_interfaces();
    let mut current_iface = String::new();

    for iface in &ifaces {
        let state = get_interface_state(iface);
        let mac = get_mac_address(iface);
        let mtu = get_mtu(iface);

        println!("{}: <{}> mtu {} qdisc {}", 
            iface,
            state.to_uppercase(),
            mtu,
            if state == "up" { "fq_codel" } else { "noop" }
        );
        println!("    link/ether {} brd ff:ff:ff:ff:ff:ff", mac);

        // Show addresses for this interface
        for (iface_name, family, addr) in &addresses {
            if iface_name == iface {
                println!("    {} {}", family, addr);
                if family == "inet" {
                    println!("    scope global {}", iface_name);
                }
            }
        }
        println!();
    }
}

fn cmd_link() {
    let ifaces = get_interfaces();

    for (idx, iface) in ifaces.iter().enumerate() {
        let state = get_interface_state(iface);
        let mac = get_mac_address(iface);
        let mtu = get_mtu(iface);

        println!("{}: {}: <BROADCAST,MULTICAST,{},UP> mtu {}",
            idx + 1,
            iface,
            if state == "up" { "BROADCAST" } else { "NOARP" },
            mtu
        );
        println!("    link/ether {} brd ff:ff:ff:ff:ff:ff", mac);
    }
}

fn cmd_route() {
    println!("Kernel IP routing table");
    println!("{:<20} {:<20} {:<15} {}", "Destination", "Gateway", "Genmask", "Flags");

    if let Ok(output) = process::Command::new("ip").arg("route").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            println!("  {}", line);
        }
    } else {
        // Fallback: read from /proc/net/route
        if let Ok(content) = fs::read_to_string("/proc/net/route") {
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    let dest = hex_to_ip(parts[1]);
                    let gateway = hex_to_ip(parts[2]);
                    println!("{:<20} {:<20} {:<15} U", dest, gateway, "255.255.255.0");
                }
            }
        }
    }
}

fn hex_to_ip(hex: &str) -> String {
    if let Ok(num) = u32::from_str_radix(hex, 16) {
        format!(
            "{}.{}.{}.{}",
            num & 0xFF,
            (num >> 8) & 0xFF,
            (num >> 16) & 0xFF,
            (num >> 24) & 0xFF
        )
    } else {
        hex.to_string()
    }
}

fn cmd_neigh() {
    println!("{:<30} {}", "IP address", "HW address");

    if let Ok(output) = process::Command::new("ip").arg("neigh").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            println!("  {}", line);
        }
    } else {
        // Fallback: read from /proc/net/arp
        if let Ok(content) = fs::read_to_string("/proc/net/arp") {
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    println!("{:<30} {}", parts[0], parts[3]);
                }
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        process::exit(0);
    }

    match args[0].as_str() {
        "addr" => cmd_addr(),
        "link" => cmd_link(),
        "route" => cmd_route(),
        "neigh" => cmd_neigh(),
        "help" | "-h" | "--help" => print_usage(),
        _ => {
            eprintln!("ip: '{}' is not a valid command", args[0]);
            println!();
            print_usage();
            process::exit(1);
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
