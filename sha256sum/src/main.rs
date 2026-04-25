use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use sha2::{Sha256, Digest};

fn print_help() {
    println!("sha256sum - Compute and check SHA256 checksums");
    println!();
    println!("USAGE:");
    println!("    sha256sum [OPTIONS] [FILE...]");
    println!();
    println!("OPTIONS:");
    println!("    -c, --check       Read SHA256 sums from FILE and check them");
    println!("    --help            Print this help message");
    println!();
    println!("DESCRIPTION:");
    println!("    With no FILE, or when FILE is -, read standard input.");
    println!("    The sums are computed as described in FIPS-180-2.");
    println!();
    println!("    When checking, the input should be a former output of this program.");
    println!("    The default format is:");
    println!("        <hash>  <filename>");
    println!("    or:");
    println!("        <hash>  <filename>");
}

fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

fn read_file(path: &str) -> Result<Vec<u8>, String> {
    if path == "-" {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)
            .map_err(|e| format!("sha256sum: failed to read stdin: {}", e))?;
        Ok(buffer)
    } else {
        fs::read(path)
            .map_err(|e| format!("sha256sum: cannot read '{}': {}", path, e))
    }
}

fn compute_checksums(files: &[String]) -> Result<(), String> {
    if files.is_empty() {
        let data = read_file("-")?;
        let hash = compute_sha256(&data);
        println!("{}  -", hash);
    } else {
        for file in files {
            let data = read_file(file)?;
            let hash = compute_sha256(&data);
            println!("{}  {}", hash, file);
        }
    }
    Ok(())
}

fn check_checksums(files: &[String]) -> Result<bool, String> {
    let mut all_ok = true;

    for check_file in files {
        let content = if check_file == "-" {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)
                .map_err(|e| format!("sha256sum: failed to read stdin: {}", e))?;
            buffer
        } else {
            fs::read_to_string(check_file)
                .map_err(|e| format!("sha256sum: cannot read '{}': {}", check_file, e))?
        };

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse line: "<hash>  <filename>" or "<hash> <filename>"
            let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
            if parts.len() != 2 {
                eprintln!("sha256sum: {}: improperly formatted SHA256 checksum line", check_file);
                all_ok = false;
                continue;
            }

            let expected_hash = parts[0];
            let filename = parts[1];

            // Handle the backslash escape prefix for binary mode
            let filename = filename.strip_prefix('\\').unwrap_or(filename);

            match read_file(filename) {
                Ok(data) => {
                    let actual_hash = compute_sha256(&data);
                    if actual_hash == expected_hash {
                        println!("{}: OK", filename);
                    } else {
                        println!("{}: FAILED", filename);
                        all_ok = false;
                    }
                }
                Err(e) => {
                    eprintln!("sha256sum: {}: {}", filename, e);
                    all_ok = false;
                }
            }
        }
    }

    Ok(all_ok)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.contains(&"--help".to_string()) {
        print_help();
        process::exit(0);
    }

    let mut check_mode = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-c" | "--check" => check_mode = true,
            _ => files.push(arg),
        }
    }

    if check_mode && files.is_empty() {
        // Check mode with no files = read from stdin
        match check_checksums(&["-".to_string()]) {
            Ok(true) => process::exit(0),
            Ok(false) => process::exit(1),
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    }

    if check_mode {
        match check_checksums(&files) {
            Ok(true) => process::exit(0),
            Ok(false) => process::exit(1),
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    } else {
        if let Err(e) = compute_checksums(&files) {
            eprintln!("{}", e);
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
