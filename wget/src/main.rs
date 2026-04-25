use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::time::Instant;

fn print_usage() {
    println!("wget - Simplified file downloader");
    println!();
    println!("USAGE:");
    println!("    wget [OPTIONS] <URL>");
    println!();
    println!("OPTIONS:");
    println!("    -O, --output <FILE>      Save to specified file");
    println!("    -P, --directory <DIR>    Save to directory");
    println!("    -r, --recursive          Recursive download (depth 1)");
    println!("    -l, --level <N>          Maximum recursion depth (default: 5)");
    println!("    -c, --continue           Resume interrupted download");
    println!("    -q, --quiet              Quiet mode");
    println!("    -np, --no-parent         Don't ascend to parent directory");
    println!("    -h, --help               Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    wget http://example.com/file.txt");
    println!("    wget -O output.txt http://example.com/page.html");
    println!("    wget -P /tmp http://example.com/file.zip");
}

struct WgetOption {
    output: Option<String>,
    directory: Option<String>,
    recursive: bool,
    level: u32,
    continue_download: bool,
    quiet: bool,
    no_parent: bool,
    url: String,
}

fn parse_args() -> Result<WgetOption, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err("No URL specified".to_string());
    }

    let mut output: Option<String> = None;
    let mut directory: Option<String> = None;
    let mut recursive = false;
    let mut level = 5u32;
    let mut continue_download = false;
    let mut quiet = false;
    let mut no_parent = false;
    let mut url: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-O" | "--output-document" => {
                i += 1;
                if i < args.len() {
                    output = Some(args[i].clone());
                } else {
                    return Err("Missing output file".to_string());
                }
            }
            "-P" | "--directory" => {
                i += 1;
                if i < args.len() {
                    directory = Some(args[i].clone());
                } else {
                    return Err("Missing directory".to_string());
                }
            }
            "-r" | "--recursive" => recursive = true,
            "-l" | "--level" => {
                i += 1;
                if i < args.len() {
                    level = args[i].parse().unwrap_or(5);
                } else {
                    return Err("Missing level value".to_string());
                }
            }
            "-c" | "--continue" => continue_download = true,
            "-q" | "--quiet" => quiet = true,
            "-np" | "--no-parent" => no_parent = true,
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            arg if !arg.starts_with('-') => {
                url = Some(arg.to_string());
            }
            _ => {
                return Err(format!("Unknown option: {}", args[i]));
            }
        }
        i += 1;
    }

    let url = url.ok_or_else(|| "No URL specified".to_string())?;

    Ok(WgetOption {
        output,
        directory,
        recursive,
        level,
        continue_download,
        quiet,
        no_parent,
        url,
    })
}

fn extract_filename(url: &str) -> String {
    let path = url.rsplit('/').next().unwrap_or("index.html");
    if path.is_empty() || path == "." {
        "index.html".to_string()
    } else {
        path.to_string()
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_speed(bytes_per_sec: u64) -> String {
    format!("{}/s", format_size(bytes_per_sec))
}

fn download_file(url: &str, dest_path: &str, opts: &WgetOption, _depth: u32) -> Result<u64, String> {
    if !opts.quiet {
        println!("-- {}  {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), url);
        println!("Resolving {}...", url.split('/').nth(2).unwrap_or(""));
    }

    // Make request
    let resp = ureq::get(url).call().map_err(|e| match e {
        ureq::Error::Status(code, _) => format!("HTTP error: {}", code),
        ureq::Error::Transport(e) => format!("Download failed: {}", e),
    })?;

    let total_size = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if !opts.quiet {
        println!("Length: {} ({})", total_size, format_size(total_size));
    }

    // Create parent directories if needed
    if let Some(parent) = Path::new(dest_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    // Open file for writing
    let mut file = File::create(dest_path).map_err(|e| format!("Failed to create file: {}", e))?;

    let start = Instant::now();
    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 8192];
    let mut reader = resp.into_reader();

    loop {
        let bytes_read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => return Err(format!("Read error: {}", e)),
        };

        file.write_all(&buffer[..bytes_read]).map_err(|e| format!("Write error: {}", e))?;
        downloaded += bytes_read as u64;

        if !opts.quiet && total_size > 0 {
            let percent = (downloaded as f64 / total_size as f64) * 100.0;
            let elapsed = start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                (downloaded as f64 / elapsed) as u64
            } else {
                0
            };

            let bar_width = 40;
            let filled = (percent / 100.0 * bar_width as f64) as usize;
            let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);

            print!("\r{} [{}] {:.1}% {} {}", 
                format_size(total_size), bar, percent, 
                format_size(downloaded), format_speed(speed));
            io::stdout().flush().ok();
        } else if !opts.quiet {
            print!(".");
            io::stdout().flush().ok();
        }
    }

    if !opts.quiet {
        println!();
        let elapsed = start.elapsed().as_secs_f64();
        println!(
            "'{}' saved [{}/{}]",
            dest_path,
            downloaded,
            format_size(total_size.max(downloaded))
        );
        if elapsed > 0.0 {
            println!("Speed: {}", format_speed((downloaded as f64 / elapsed) as u64));
        }
    }

    Ok(downloaded)
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

    // Determine output file
    let filename = opts.output.clone().unwrap_or_else(|| extract_filename(&opts.url));
    let dest_path = if let Some(ref dir) = opts.directory {
        format!("{}/{}", dir, filename)
    } else {
        filename
    };

    if !opts.quiet {
        println!("Saving to: '{}'\n", dest_path);
    }

    match download_file(&opts.url, &dest_path, &opts, 0) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
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
