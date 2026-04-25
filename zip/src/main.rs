use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::process;

fn print_usage() {
    println!("zip - Package and compress files");
    println!();
    println!("USAGE:");
    println!("    zip [OPTIONS] <OUTPUT.ZIP> [FILES...]");
    println!();
    println!("OPTIONS:");
    println!("    -r, --recursive     Recursively add directory contents");
    println!("    -q, --quiet         Quiet mode");
    println!("    -h, --help          Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    zip archive.zip file1 file2");
    println!("    zip -r archive.zip directory/");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        print_usage();
        process::exit(0);
    }

    let mut recursive = false;
    let mut quiet = false;
    let mut files = Vec::new();

    for arg in &args {
        match arg.as_str() {
            "-r" | "--recursive" => recursive = true,
            "-q" | "--quiet" => quiet = true,
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            _ => files.push(arg.clone()),
        }
    }

    if files.is_empty() {
        eprintln!("zip: missing arguments");
        process::exit(1);
    }

    let output = &files[0];
    let input_files = &files[1..];

    let file = File::create(output).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default();

    for input in input_files {
        let path = Path::new(input);
        if !path.exists() {
            eprintln!("zip: {}: No such file or directory", input);
            continue;
        }

        if recursive && path.is_dir() {
            add_directory(&mut writer, path, path, options, quiet);
        } else {
            add_file(&mut writer, path, input, options, quiet);
        }
    }

    match writer.finish() {
        Ok(_) => {
            if !quiet {
                println!("\nArchive created: {}", output);
            }
        }
        Err(e) => {
            eprintln!("zip: failed to finalize archive: {}", e);
            process::exit(1);
        }
    }
}

fn add_file<W: Write + Seek>(
    writer: &mut zip::ZipWriter<W>,
    path: &Path,
    name: &str,
    options: zip::write::FileOptions,
    quiet: bool,
) {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("zip: {}: {}", name, e);
            return;
        }
    };

    let mut contents = Vec::new();
    if file.read_to_end(&mut contents).is_err() {
        return;
    }

    if let Err(e) = writer.start_file(name, options) {
        eprintln!("zip: failed to add {}: {}", name, e);
        return;
    }
    if let Err(e) = writer.write_all(&contents) {
        eprintln!("zip: write error: {}", e);
        return;
    }

    if !quiet {
        println!("adding: {} ({} bytes)", name, contents.len());
    }
}

fn add_directory<W: Write + Seek>(
    writer: &mut zip::ZipWriter<W>,
    path: &Path,
    base: &Path,
    options: zip::write::FileOptions,
    quiet: bool,
) {
    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let child = entry.path();
                let name = child.strip_prefix(base).unwrap_or(&child).to_string_lossy().replace('\\', "/");
                if child.is_dir() {
                    add_directory(writer, &child, base, options, quiet);
                } else {
                    add_file(writer, &child, &name, options, quiet);
                }
            }
        }
    }
}
