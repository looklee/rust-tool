use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process;

fn print_usage() {
    println!("7z - 7-Zip archive utility (simplified)");
    println!();
    println!("USAGE:");
    println!("    7z <COMMAND> [OPTIONS] [FILES...]");
    println!();
    println!("COMMANDS:");
    println!("    a              Add files to archive");
    println!("    x              Extract files from archive");
    println!("    l              List archive contents");
    println!();
    println!("OPTIONS:");
    println!("    -o<DIR>        Output directory");
    println!("    -r             Recurse subdirectories");
    println!("    -h, --help     Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    7z a archive.7z file1 file2");
    println!("    7z x archive.7z");
    println!("    7z l archive.7z");
}

fn add_files(archive: &str, files: &[String], recursive: bool) -> Result<(), String> {
    let file = File::create(archive).map_err(|e| format!("Failed to create archive: {}", e))?;
    let mut encoder = GzEncoder::new(file, Compression::default());

    // Write header: number of files
    encoder.write_all(&files.len().to_le_bytes()).map_err(|e| e.to_string())?;

    for file_path in files {
        let path = Path::new(file_path);
        if path.is_dir() && recursive {
            add_directory(&mut encoder, path, path)?;
        } else if path.is_file() {
            add_single_file(&mut encoder, path)?;
        }
    }

    encoder.finish().map_err(|e| format!("Failed to finalize: {}", e))?;
    println!("Archive created: {}", archive);
    Ok(())
}

fn add_single_file<W: Write>(encoder: &mut GzEncoder<W>, path: &Path) -> Result<(), String> {
    let name = path.to_string_lossy();
    let name_bytes = name.as_bytes();

    // Write name length and name
    encoder.write_all(&(name_bytes.len() as u32).to_le_bytes()).map_err(|e| e.to_string())?;
    encoder.write_all(name_bytes).map_err(|e| e.to_string())?;

    // Write file size
    let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
    encoder.write_all(&size.to_le_bytes()).map_err(|e| e.to_string())?;

    // Write file contents
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).map_err(|e| e.to_string())?;
    encoder.write_all(&contents).map_err(|e| e.to_string())?;

    println!("  adding: {} ({} bytes)", name, contents.len());
    Ok(())
}

fn add_directory<W: Write>(encoder: &mut GzEncoder<W>, path: &Path, base: &Path) -> Result<(), String> {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() {
                add_directory(encoder, &child, base)?;
            } else {
                add_single_file(encoder, &child)?;
            }
        }
    }
    Ok(())
}

fn extract_archive(archive: &str, output_dir: &str) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut decoder = GzDecoder::new(file);

    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    // Read number of files
    let mut count_bytes = [0u8; 8];
    decoder.read_exact(&mut count_bytes).map_err(|e| e.to_string())?;
    let count = u64::from_le_bytes(count_bytes);

    for _ in 0..count {
        // Read name
        let mut name_len_bytes = [0u8; 4];
        decoder.read_exact(&mut name_len_bytes).map_err(|e| e.to_string())?;
        let name_len = u32::from_le_bytes(name_len_bytes) as usize;

        let mut name_bytes = vec![0u8; name_len];
        decoder.read_exact(&mut name_bytes).map_err(|e| e.to_string())?;
        let name = String::from_utf8_lossy(&name_bytes);

        // Read size
        let mut size_bytes = [0u8; 8];
        decoder.read_exact(&mut size_bytes).map_err(|e| e.to_string())?;
        let size = u64::from_le_bytes(size_bytes);

        // Read contents
        let mut contents = vec![0u8; size as usize];
        decoder.read_exact(&mut contents).map_err(|e| e.to_string())?;

        // Write file
        let dest = PathBuf::from(output_dir).join(name.as_ref());
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&dest, &contents).map_err(|e| e.to_string())?;

        println!("  extracting: {} ({} bytes)", name, size);
    }

    println!("\nExtracted: {}", archive);
    Ok(())
}

fn list_archive(archive: &str) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut decoder = GzDecoder::new(file);

    let mut count_bytes = [0u8; 8];
    decoder.read_exact(&mut count_bytes).map_err(|e| e.to_string())?;
    let count = u64::from_le_bytes(count_bytes);

    println!("{:<12} {}", "Size", "Name");
    println!("{}", "-".repeat(60));

    for _ in 0..count {
        let mut name_len_bytes = [0u8; 4];
        decoder.read_exact(&mut name_len_bytes).map_err(|e| e.to_string())?;
        let name_len = u32::from_le_bytes(name_len_bytes) as usize;

        let mut name_bytes = vec![0u8; name_len];
        decoder.read_exact(&mut name_bytes).map_err(|e| e.to_string())?;
        let name = String::from_utf8_lossy(&name_bytes);

        let mut size_bytes = [0u8; 8];
        decoder.read_exact(&mut size_bytes).map_err(|e| e.to_string())?;
        let size = u64::from_le_bytes(size_bytes);

        println!("{:<12} {}", size, name);
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        process::exit(0);
    }

    let mut output_dir = ".".to_string();
    let mut recursive = false;
    let mut command = None;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "a" => command = Some("a"),
            "x" => command = Some("x"),
            "l" => command = Some("l"),
            arg if arg.starts_with("-o") => {
                output_dir = arg[2..].to_string();
            }
            "-r" => recursive = true,
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            arg if !arg.starts_with('-') => {
                files.push(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    let command = match command {
        Some(c) => c,
        None => {
            eprintln!("7z: missing command");
            print_usage();
            process::exit(1);
        }
    };

    match command {
        "a" => {
            if files.len() < 2 {
                eprintln!("7z: missing archive or files");
                process::exit(1);
            }
            let archive = &files[0];
            let input_files = &files[1..];
            if let Err(e) = add_files(archive, input_files, recursive) {
                eprintln!("7z: {}", e);
                process::exit(1);
            }
        }
        "x" => {
            if files.is_empty() {
                eprintln!("7z: missing archive");
                process::exit(1);
            }
            if let Err(e) = extract_archive(&files[0], &output_dir) {
                eprintln!("7z: {}", e);
                process::exit(1);
            }
        }
        "l" => {
            if files.is_empty() {
                eprintln!("7z: missing archive");
                process::exit(1);
            }
            if let Err(e) = list_archive(&files[0]) {
                eprintln!("7z: {}", e);
                process::exit(1);
            }
        }
        _ => {
            eprintln!("7z: unknown command: {}", command);
            process::exit(1);
        }
    }
}
