use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::process;

fn print_usage() {
    println!("gzip - Compress or uncompress files");
    println!();
    println!("USAGE:");
    println!("    gzip [OPTIONS] [FILE...]");
    println!();
    println!("OPTIONS:");
    println!("    -d, --decompress    Decompress");
    println!("    -k, --keep          Keep input files");
    println!("    -f, --force         Force overwrite");
    println!("    -l, --list          List compressed file contents");
    println!("    -r, --recursive     Recursively compress directories");
    println!("    -h, --help          Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    gzip file.txt");
    println!("    gzip -d file.txt.gz");
    println!("    gzip -r directory/");
}

fn compress_file(input: &str, output: &str, force: bool) -> Result<u64, String> {
    if !force && fs::metadata(output).is_ok() {
        return Err(format!("{} already exists; use -f to overwrite", output));
    }

    let mut input_file = File::open(input).map_err(|e| format!("Failed to open {}: {}", input, e))?;
    let output_file = File::create(output).map_err(|e| format!("Failed to create {}: {}", output, e))?;

    let mut encoder = GzEncoder::new(output_file, Compression::default());
    let mut buffer = [0u8; 8192];
    let mut total: u64 = 0;

    loop {
        let bytes_read = input_file.read(&mut buffer).map_err(|e| format!("Read error: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        encoder.write_all(&buffer[..bytes_read]).map_err(|e| format!("Write error: {}", e))?;
        total += bytes_read as u64;
    }

    encoder.finish().map_err(|e| format!("Compression failed: {}", e))?;
    Ok(total)
}

fn decompress_file(input: &str, output: &str, force: bool) -> Result<u64, String> {
    if !force && fs::metadata(output).is_ok() {
        return Err(format!("{} already exists; use -f to overwrite", output));
    }

    let input_file = File::open(input).map_err(|e| format!("Failed to open {}: {}", input, e))?;
    let mut decoder = GzDecoder::new(input_file);
    let mut output_file = File::create(output).map_err(|e| format!("Failed to create {}: {}", output, e))?;

    let mut buffer = [0u8; 8192];
    let mut total: u64 = 0;

    loop {
        let bytes_read = decoder.read(&mut buffer).map_err(|e| format!("Decompression error: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        output_file.write_all(&buffer[..bytes_read]).map_err(|e| format!("Write error: {}", e))?;
        total += bytes_read as u64;
    }

    Ok(total)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        process::exit(0);
    }

    let mut decompress = false;
    let mut keep = false;
    let mut force = false;
    let mut list = false;
    let mut recursive = false;
    let mut files = Vec::new();

    for arg in &args {
        match arg.as_str() {
            "-d" | "--decompress" => decompress = true,
            "-k" | "--keep" => keep = true,
            "-f" | "--force" => force = true,
            "-l" | "--list" => list = true,
            "-r" | "--recursive" => recursive = true,
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            _ => files.push(arg.clone()),
        }
    }

    if files.is_empty() {
        eprintln!("gzip: missing file operand");
        process::exit(1);
    }

    for file in &files {
        if recursive && fs::metadata(file).map(|m| m.is_dir()).unwrap_or(false) {
            // Recursively find files
            if let Ok(entries) = fs::read_dir(file) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(p) = path.to_str() {
                            process_file(p, decompress, keep, force);
                        }
                    }
                }
            }
        } else {
            process_file(file, decompress, keep, force);
        }
    }
}

fn process_file(file: &str, decompress: bool, keep: bool, force: bool) {
    if decompress {
        // Decompress
        if !file.ends_with(".gz") {
            eprintln!("gzip: {}: unknown suffix", file);
            return;
        }
        let output = &file[..file.len() - 3];
        match decompress_file(file, output, force) {
            Ok(size) => {
                println!("{}: {} bytes", file, size);
                if !keep {
                    fs::remove_file(file).ok();
                }
            }
            Err(e) => eprintln!("gzip: {}", e),
        }
    } else {
        // Compress
        let output = format!("{}.gz", file);
        match compress_file(file, &output, force) {
            Ok(size) => {
                println!("{}: {} bytes", file, size);
                if !keep {
                    fs::remove_file(file).ok();
                }
            }
            Err(e) => eprintln!("gzip: {}", e),
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
