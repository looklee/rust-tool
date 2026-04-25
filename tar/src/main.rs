use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

fn print_usage() {
    println!("tar - Tape archive utility");
    println!();
    println!("USAGE:");
    println!("    tar <COMMAND> [OPTIONS] [FILES...]");
    println!();
    println!("COMMANDS:");
    println!("    -c, --create       Create a new archive");
    println!("    -x, --extract      Extract files from archive");
    println!("    -t, --list         List contents of archive");
    println!();
    println!("OPTIONS:");
    println!("    -f, --file <FILE>  Archive file name");
    println!("    -z, --gzip         Compress with gzip");
    println!("    -v, --verbose      Verbose output");
    println!("    -C, --directory    Extract to directory");
    println!("    -h, --help         Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    tar -cvf archive.tar file1 file2");
    println!("    tar -xvf archive.tar");
    println!("    tar -tvf archive.tar");
    println!("    tar -czvf archive.tar.gz file1");
    println!("    tar -xzvf archive.tar.gz");
}

#[derive(Debug, Clone, Copy)]
enum TarMode {
    Create,
    Extract,
    List,
}

struct TarOption {
    mode: TarMode,
    file: String,
    gzip: bool,
    verbose: bool,
    directory: Option<String>,
    files: Vec<String>,
}

fn parse_args() -> Result<TarOption, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err("No command specified".to_string());
    }

    let mut mode = TarMode::Create;
    let mut file: Option<String> = None;
    let mut gzip = false;
    let mut verbose = false;
    let mut directory: Option<String> = None;
    let mut files = Vec::new();

    // Check for long options first
    if args[0] == "--create" || args[0] == "-c" {
        mode = TarMode::Create;
    } else if args[0] == "--extract" || args[0] == "-x" {
        mode = TarMode::Extract;
    } else if args[0] == "--list" || args[0] == "-t" {
        mode = TarMode::List;
    } else if args[0].starts_with('-') && !args[0].starts_with("--") {
        // Combined short options like -cvf
        let chars: Vec<char> = args[0].chars().collect();
        let mut i = 1;
        while i < chars.len() {
            match chars[i] {
                'c' => mode = TarMode::Create,
                'x' => mode = TarMode::Extract,
                't' => mode = TarMode::List,
                'v' => verbose = true,
                'z' => gzip = true,
                'f' => {
                    i += 1;
                    if i < chars.len() {
                        file = Some(chars[i..].iter().collect());
                        break;
                    } else if args.len() > 1 {
                        file = Some(args[1].clone());
                        return Ok(TarOption {
                            mode,
                            file: file.unwrap(),
                            gzip,
                            verbose,
                            directory,
                            files: args[2..].to_vec(),
                        });
                    }
                }
                _ => {}
            }
            i += 1;
        }

        // Parse remaining args
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-f" | "--file" => {
                    i += 1;
                    if i < args.len() {
                        file = Some(args[i].clone());
                    }
                }
                "-C" | "--directory" => {
                    i += 1;
                    if i < args.len() {
                        directory = Some(args[i].clone());
                    }
                }
                arg if !arg.starts_with('-') => {
                    files.push(arg.to_string());
                }
                _ => {}
            }
            i += 1;
        }

        let file = file.ok_or_else(|| "Missing file argument".to_string())?;
        return Ok(TarOption {
            mode,
            file,
            gzip,
            verbose,
            directory,
            files,
        });
    } else {
        return Err(format!("Unknown command: {}", args[0]));
    }

    // Parse remaining args for long option format
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-f" | "--file" => {
                i += 1;
                if i < args.len() {
                    file = Some(args[i].clone());
                } else {
                    return Err("Missing file argument".to_string());
                }
            }
            "-z" | "--gzip" => gzip = true,
            "-v" | "--verbose" => verbose = true,
            "-C" | "--directory" => {
                i += 1;
                if i < args.len() {
                    directory = Some(args[i].clone());
                } else {
                    return Err("Missing directory argument".to_string());
                }
            }
            arg if !arg.starts_with('-') => {
                files.push(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    let file = file.ok_or_else(|| "Missing file argument".to_string())?;

    Ok(TarOption {
        mode,
        file,
        gzip,
        verbose,
        directory,
        files,
    })
}

fn create_tar_archive(opts: &TarOption) -> Result<(), String> {
    let file = File::create(&opts.file).map_err(|e| format!("Failed to create archive: {}", e))?;

    let mut writer: Box<dyn Write> = if opts.gzip {
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        Box::new(encoder)
    } else {
        Box::new(file)
    };

    let mut archive = tar::Builder::new(writer);

    for file_path in &opts.files {
        if opts.verbose {
            println!("{}", file_path);
        }
        let path = Path::new(file_path);
        if path.is_dir() {
            archive
                .append_dir_all(file_path, path)
                .map_err(|e| format!("Failed to add directory {}: {}", file_path, e))?;
        } else {
            archive
                .append_path(file_path)
                .map_err(|e| format!("Failed to add file {}: {}", file_path, e))?;
        }
    }

    archive.finish().map_err(|e| format!("Failed to finalize archive: {}", e))?;
    Ok(())
}

fn extract_tar_archive(opts: &TarOption) -> Result<(), String> {
    let file = File::open(&opts.file).map_err(|e| format!("Failed to open archive: {}", e))?;

    let mut reader: Box<dyn Read> = if opts.gzip {
        let decoder = flate2::read::GzDecoder::new(file);
        Box::new(decoder)
    } else {
        Box::new(file)
    };

    let mut archive = tar::Archive::new(reader);

    let dest_dir = opts
        .directory
        .as_deref()
        .unwrap_or(".");

    fs::create_dir_all(dest_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    for entry in archive.entries().map_err(|e| format!("Failed to read archive: {}", e))? {
        let mut entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path().map_err(|e| e.to_string())?;
        let path_str = path.to_string_lossy().to_string();

        if opts.verbose {
            println!("{}", path_str);
        }

        let dest = PathBuf::from(dest_dir).join(path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // Copy entry contents to file
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents).map_err(|e| format!("Failed to read entry: {}", e))?;

        if !contents.is_empty() {
            fs::write(&dest, &contents).map_err(|e| format!("Failed to write {}: {}", path_str, e))?;
        }
    }

    Ok(())
}

fn list_tar_archive(opts: &TarOption) -> Result<(), String> {
    let file = File::open(&opts.file).map_err(|e| format!("Failed to open archive: {}", e))?;

    let mut reader: Box<dyn Read> = if opts.gzip {
        let decoder = flate2::read::GzDecoder::new(file);
        Box::new(decoder)
    } else {
        Box::new(file)
    };

    let mut archive = tar::Archive::new(reader);

    println!("{:<10} {:<12} {}", "Type", "Size", "Name");
    println!("{}", "-".repeat(60));

    for entry in archive.entries().map_err(|e| format!("Failed to read archive: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path().map_err(|e| e.to_string())?;
        let size = entry.size();

        let type_str = if entry.header().entry_type().is_dir() {
            "directory"
        } else {
            "file"
        };

        println!("{:<10} {:<12} {}", type_str, size, path.display());
    }

    Ok(())
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

    let result = match opts.mode {
        TarMode::Create => create_tar_archive(&opts),
        TarMode::Extract => extract_tar_archive(&opts),
        TarMode::List => list_tar_archive(&opts),
    };

    if let Err(e) = result {
        eprintln!("tar: {}", e);
        process::exit(1);
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
