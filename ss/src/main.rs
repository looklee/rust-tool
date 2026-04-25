use std::fs;
use std::io::{self, BufRead};
use std::process;

fn print_usage() {
    println!("ss - Socket statistics");
    println!();
    println!("USAGE:");
    println!("    ss [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -t, --tcp          Show TCP sockets");
    println!("    -u, --udp          Show UDP sockets");
    println!("    -l, --listening    Show listening sockets only");
    println!("    -a, --all          Show all sockets (default)");
    println!("    -n, --numeric      Don't resolve service names");
    println!("    -p, --processes    Show process info");
    println!("    -h, --help         Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    ss -tuln");
    println!("    ss -t");
    println!("    ss -a");
}

#[derive(Debug)]
struct Socket {
    proto: String,
    state: String,
    local_addr: String,
    remote_addr: String,
    process: String,
}

fn hex_to_ip_port(hex_str: &str) -> String {
    let parts: Vec<&str> = hex_str.split(':').collect();
    if parts.len() != 2 {
        return hex_str.to_string();
    }

    let ip_hex = parts[0];
    let port_hex = parts[1];

    if let (Ok(ip_num), Ok(port)) = (u32::from_str_radix(ip_hex, 16), u16::from_str_radix(port_hex, 16)) {
        let ip = format!(
            "{}.{}.{}.{}",
            ip_num & 0xFF,
            (ip_num >> 8) & 0xFF,
            (ip_num >> 16) & 0xFF,
            (ip_num >> 24) & 0xFF
        );
        format!("{}:{}", ip, port)
    } else {
        hex_str.to_string()
    }
}

fn tcp_state_str(state: u8) -> &'static str {
    match state {
        0x01 => "ESTAB",
        0x02 => "SYN-SENT",
        0x03 => "SYN-RECV",
        0x04 => "FIN-WAIT-1",
        0x05 => "FIN-WAIT-2",
        0x06 => "TIME-WAIT",
        0x07 => "CLOSE",
        0x08 => "CLOSE-WAIT",
        0x09 => "LAST-ACK",
        0x0A => "LISTEN",
        0x0B => "CLOSING",
        _ => "UNKNOWN",
    }
}

fn parse_tcp_sockets() -> Vec<Socket> {
    let mut sockets = Vec::new();

    if let Ok(file) = fs::File::open("/proc/net/tcp") {
        let reader = io::BufReader::new(file);
        for line in reader.lines().flatten().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                let state = u8::from_str_radix(parts[3], 16).unwrap_or(0);
                let local_addr = hex_to_ip_port(parts[1]);
                let remote_addr = hex_to_ip_port(parts[2]);

                sockets.push(Socket {
                    proto: "tcp".to_string(),
                    state: tcp_state_str(state).to_string(),
                    local_addr,
                    remote_addr,
                    process: String::new(),
                });
            }
        }
    }

    // Also check TCP6
    if let Ok(file) = fs::File::open("/proc/net/tcp6") {
        let reader = io::BufReader::new(file);
        for line in reader.lines().flatten().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                let state = u8::from_str_radix(parts[3], 16).unwrap_or(0);
                let local_hex = parts[1];
                let remote_hex = parts[2];

                // Parse IPv6
                let local_addr = parse_ipv6_port(local_hex);
                let remote_addr = parse_ipv6_port(remote_hex);

                sockets.push(Socket {
                    proto: "tcp6".to_string(),
                    state: tcp_state_str(state).to_string(),
                    local_addr,
                    remote_addr,
                    process: String::new(),
                });
            }
        }
    }

    sockets
}

fn parse_ipv6_port(hex_str: &str) -> String {
    let parts: Vec<&str> = hex_str.split(':').collect();
    if parts.len() != 2 {
        return hex_str.to_string();
    }

    let ip_hex = parts[0];
    let port_hex = parts[1];

    if let Ok(port) = u16::from_str_radix(port_hex, 16) {
        // Convert hex to IPv6
        let mut ip = String::new();
        for (i, chunk) in ip_hex.as_bytes().chunks(4).enumerate() {
            if i > 0 {
                ip.push(':');
            }
            ip.push_str(&String::from_utf8_lossy(chunk));
        }
        format!("[{}]:{}", ip, port)
    } else {
        hex_str.to_string()
    }
}

fn parse_udp_sockets() -> Vec<Socket> {
    let mut sockets = Vec::new();

    if let Ok(file) = fs::File::open("/proc/net/udp") {
        let reader = io::BufReader::new(file);
        for line in reader.lines().flatten().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                let local_addr = hex_to_ip_port(parts[1]);
                let remote_addr = hex_to_ip_port(parts[2]);

                sockets.push(Socket {
                    proto: "udp".to_string(),
                    state: "UNCONN".to_string(),
                    local_addr,
                    remote_addr,
                    process: String::new(),
                });
            }
        }
    }

    // Also check UDP6
    if let Ok(file) = fs::File::open("/proc/net/udp6") {
        let reader = io::BufReader::new(file);
        for line in reader.lines().flatten().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                let local_hex = parts[1];
                let remote_hex = parts[2];

                let local_addr = parse_ipv6_port(local_hex);
                let remote_addr = parse_ipv6_port(remote_hex);

                sockets.push(Socket {
                    proto: "udp6".to_string(),
                    state: "UNCONN".to_string(),
                    local_addr,
                    remote_addr,
                    process: String::new(),
                });
            }
        }
    }

    sockets
}

fn show_sockets(sockets: &[Socket], show_all: bool, show_listening: bool) {
    let mut filtered: Vec<&Socket> = Vec::new();

    for socket in sockets {
        if show_listening && socket.state == "LISTEN" {
            filtered.push(socket);
        } else if show_all {
            filtered.push(socket);
        }
    }

    println!("{:<6} {:<14} {:<30} {:<30} {}", "Netid", "State", "Local Address:Port", "Remote Address:Port", "Process");

    for socket in &filtered {
        println!("{:<6} {:<14} {:<30} {:<30} {}",
            socket.proto,
            socket.state,
            socket.local_addr,
            socket.remote_addr,
            socket.process
        );
    }

    println!();
    println!("Total: {} sockets", filtered.len());
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut show_tcp = false;
    let mut show_udp = false;
    let mut show_listening = false;
    let mut show_all = true;

    for arg in &args {
        match arg.as_str() {
            "-t" | "--tcp" => show_tcp = true,
            "-u" | "--udp" => show_udp = true,
            "-l" | "--listening" => show_listening = true,
            "-a" | "--all" => show_all = true,
            "-n" | "--numeric" | "-p" | "--processes" => {
                // Accept but not implemented
            }
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            arg if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                // Handle combined flags like -tuln
                for c in arg.chars().skip(1) {
                    match c {
                        't' => show_tcp = true,
                        'u' => show_udp = true,
                        'l' => show_listening = true,
                        'a' => show_all = true,
                        'n' | 'p' => {}
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    // If no specific protocol specified, show both
    if !show_tcp && !show_udp {
        show_tcp = true;
        show_udp = true;
    }

    let mut all_sockets = Vec::new();

    if show_tcp {
        all_sockets.extend(parse_tcp_sockets());
    }
    if show_udp {
        all_sockets.extend(parse_udp_sockets());
    }

    show_sockets(&all_sockets, show_all, show_listening);
}
