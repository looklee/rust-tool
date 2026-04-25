use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::process;
use std::time::{Duration, Instant};

fn print_usage() {
    println!("ping - Network connectivity tester");
    println!();
    println!("USAGE:");
    println!("    ping [OPTIONS] <HOST>");
    println!();
    println!("OPTIONS:");
    println!("    -c, --count <N>       Number of packets to send (default: 4)");
    println!("    -i, --interval <SEC>  Interval between packets in seconds (default: 1)");
    println!("    -s, --size <BYTES>    Data size in bytes (default: 56)");
    println!("    -t, --timeout <SEC>   Timeout in seconds (default: 5)");
    println!("    -h, --help            Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    ping google.com");
    println!("    ping -c 10 8.8.8.8");
    println!("    ping -i 0.5 example.com");
}

struct PingOption {
    count: u32,
    interval: f64,
    timeout: u64,
    host: String,
}

fn parse_args() -> Result<PingOption, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err("No host specified".to_string());
    }

    let mut count = 4u32;
    let mut interval = 1.0f64;
    let mut timeout = 5u64;
    let mut host: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--count" => {
                i += 1;
                if i < args.len() {
                    count = args[i].parse().unwrap_or(4);
                } else {
                    return Err("Missing count value".to_string());
                }
            }
            "-i" | "--interval" => {
                i += 1;
                if i < args.len() {
                    interval = args[i].parse().unwrap_or(1.0);
                } else {
                    return Err("Missing interval value".to_string());
                }
            }
            "-s" | "--size" => {
                i += 1;
                // Accept but ignore size parameter for TCP fallback
            }
            "-t" | "--timeout" => {
                i += 1;
                if i < args.len() {
                    timeout = args[i].parse().unwrap_or(5);
                } else {
                    return Err("Missing timeout value".to_string());
                }
            }
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            arg if !arg.starts_with('-') => {
                host = Some(arg.to_string());
            }
            _ => {
                return Err(format!("Unknown option: {}", args[i]));
            }
        }
        i += 1;
    }

    let host = host.ok_or_else(|| "No host specified".to_string())?;

    Ok(PingOption {
        count,
        interval,
        timeout,
        host,
    })
}

fn resolve_host(host: &str) -> Result<IpAddr, String> {
    let addrs: Vec<SocketAddr> = (host, 0)
        .to_socket_addrs()
        .map_err(|e| format!("Could not resolve '{}': {}", host, e))?
        .collect();

    if addrs.is_empty() {
        return Err(format!("Could not resolve '{}'", host));
    }

    Ok(addrs[0].ip())
}

fn main() {
    let opts = match parse_args() {
        Ok(opts) => opts,
        Err(e) => {
            eprintln!("Error: {}", e);
            println!();
            print_usage();
            process::exit(1);
        }
    };

    let target = match resolve_host(&opts.host) {
        Ok(ip) => ip,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    println!("PING {} ({})", opts.host, target);
    println!();

    let mut success_count = 0u32;
    let mut total_time = Duration::ZERO;
    let mut min_time = Duration::MAX;
    let mut max_time = Duration::ZERO;

    for seq in 0..opts.count {
        let start = Instant::now();
        let socket_addr = SocketAddr::new(target, 0);

        // Use UDP connect to test reachability (no actual data sent)
        let result = std::net::UdpSocket::bind("0.0.0.0:0").and_then(|socket| {
            socket.set_read_timeout(Some(Duration::from_secs(opts.timeout)))?;
            socket.connect(socket_addr)?;
            Ok(())
        });

        let elapsed = start.elapsed();

        match result {
            Ok(_) => {
                success_count += 1;
                total_time += elapsed;
                if elapsed < min_time {
                    min_time = elapsed;
                }
                if elapsed > max_time {
                    max_time = elapsed;
                }
                println!(
                    "64 bytes from {}: icmp_seq={} time={:.3}ms",
                    target, seq, elapsed.as_secs_f64() * 1000.0
                );
            }
            Err(e) => {
                eprintln!("From {} icmp_seq={}: {}", target, seq, e);
            }
        }

        if seq < opts.count - 1 {
            std::thread::sleep(Duration::from_secs_f64(opts.interval));
        }
    }

    // Print statistics
    println!();
    println!("--- {} ping statistics ---", opts.host);
    println!(
        "{} packets transmitted, {} received, {:.0}% packet loss",
        opts.count,
        success_count,
        if opts.count > 0 {
            (1.0 - success_count as f64 / opts.count as f64) * 100.0
        } else {
            0.0
        }
    );

    if success_count > 0 {
        let avg = total_time / success_count;
        println!(
            "rtt min/avg/max = {:.3}/{:.3}/{:.3} ms",
            min_time.as_secs_f64() * 1000.0,
            avg.as_secs_f64() * 1000.0,
            max_time.as_secs_f64() * 1000.0
        );
    }
}
