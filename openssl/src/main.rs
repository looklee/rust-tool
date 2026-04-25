use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use md5::Md5;
use sha1::Sha1;
use sha2::Sha256;
use sha2::Digest;

fn print_help() {
    println!("openssl - Basic cryptographic hash operations");
    println!();
    println!("USAGE:");
    println!("    openssl <command> [OPTIONS] [FILE...]");
    println!();
    println!("COMMANDS:");
    println!("    md5       Compute MD5 hash");
    println!("    sha1      Compute SHA-1 hash");
    println!("    sha256    Compute SHA-256 hash");
    println!();
    println!("OPTIONS:");
    println!("    --help    Print this help message");
    println!();
    println!("DESCRIPTION:");
    println!("    If no FILE is given, reads from stdin.");
    println!("    Outputs the hash digest in hexadecimal format.");
}

fn compute_hash<D: Digest + Default>(data: &[u8]) -> String {
    let mut hasher = D::default();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

fn read_file_or_stdin(files: &[String]) -> Result<Vec<u8>, String> {
    if files.is_empty() {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)
            .map_err(|e| format!("openssl: failed to read stdin: {}", e))?;
        Ok(buffer)
    } else {
        let mut all_data = Vec::new();
        for file in files {
            let data = fs::read(file)
                .map_err(|e| format!("openssl: cannot read '{}': {}", file, e))?;
            all_data.extend_from_slice(&data);
        }
        Ok(all_data)
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.contains(&"--help".to_string()) {
        print_help();
        process::exit(0);
    }

    let command = args[0].as_str();
    let files = &args[1..];

    let data = match read_file_or_stdin(files) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let hash = match command {
        "md5" => compute_hash::<Md5>(&data),
        "sha1" => compute_hash::<Sha1>(&data),
        "sha256" => compute_hash::<Sha256>(&data),
        _ => {
            eprintln!("openssl: unknown command '{}'", command);
            eprintln!("Try 'openssl --help' for more information.");
            process::exit(1);
        }
    };

    println!("{}", hash);
}
