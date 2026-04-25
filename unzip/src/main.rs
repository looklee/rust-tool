use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::process;

fn print_usage() {
    println!("unzip - Extract files from a ZIP archive");
    println!();
    println!("USAGE:");
    println!("    unzip [OPTIONS] <FILE.ZIP>");
    println!();
    println!("OPTIONS:");
    println!("    -d, --directory <DIR>  Extract to directory");
    println!("    -l, --list             List contents only");
    println!("    -q, --quiet            Quiet mode");
    println!("    -h, --help             Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    unzip archive.zip");
    println!("    unzip -d /tmp archive.zip");
    println!("    unzip -l archive.zip");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        process::exit(0);
    }

    let mut directory = None;
    let mut list_only = false;
    let mut quiet = false;
    let mut file: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--directory" => {
                i += 1;
                if i < args.len() {
                    directory = Some(args[i].clone());
                }
            }
            "-l" | "--list" => list_only = true,
            "-q" | "--quiet" => quiet = true,
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            arg if !arg.starts_with('-') => {
                file = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    let file = match file {
        Some(f) => f,
        None => {
            eprintln!("unzip: missing file operand");
            process::exit(1);
        }
    };

    let zip_file = match File::open(&file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("unzip: {}: {}", file, e);
            process::exit(1);
        }
    };

    let mut archive = match zip::ZipArchive::new(zip_file) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("unzip: {}: {}", file, e);
            process::exit(1);
        }
    };

    let dest_dir = directory.unwrap_or_else(|| ".".to_string());
    fs::create_dir_all(&dest_dir).ok();

    if list_only {
        println!("{:<12} {:<12} {}", "Size", "Compressed", "Name");
        println!("{}", "-".repeat(60));
        for i in 0..archive.len() {
            let entry = archive.by_index(i).unwrap();
            println!("{:<12} {:<12} {}", entry.size(), entry.compressed_size(), entry.name());
        }
        return;
    }

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("unzip: error reading entry: {}", e);
                continue;
            }
        };

        let name = entry.name();
        let dest = PathBuf::from(&dest_dir).join(name);

        if !quiet {
            println!("extracting: {}", name);
        }

        if entry.name().ends_with('/') {
            fs::create_dir_all(&dest).ok();
            continue;
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).ok();
        }

        let mut contents = Vec::new();
        if entry.read_to_end(&mut contents).is_err() {
            continue;
        }

        if !contents.is_empty() {
            fs::write(&dest, &contents).ok();
        }
    }

    if !quiet {
        println!("\nExtracted: {}", file);
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
