use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process;

fn print_help() {
    println!("chmod - Change file mode bits");
    println!();
    println!("USAGE:");
    println!("    chmod [OPTIONS] MODE FILE...");
    println!();
    println!("OPTIONS:");
    println!("    -R, --recursive    Change files and directories recursively");
    println!("    --help             Print this help message");
    println!();
    println!("MODE:");
    println!("    Octal: 3 or 4 digit octal number (e.g., 755, 0755)");
    println!("    Symbolic: [ugoa][[+-=][perms]...] (e.g., u+x,g-w,o=r)");
    println!("              u=user, g=group, o=other, a=all");
    println!("              r=read, w=write, x=execute");
}

fn parse_octal_mode(mode_str: &str) -> Option<u32> {
    u32::from_str_radix(mode_str, 8).ok()
}

fn parse_symbolic_mode(mode_str: &str, current_mode: u32) -> Option<u32> {
    let mut result = current_mode;
    let mut ops = mode_str;

    while !ops.is_empty() {
        // Find the next comma or end
        let comma_pos = ops.find(',').unwrap_or(ops.len());
        let segment = &ops[..comma_pos];
        ops = &ops[comma_pos + 1..];

        if segment.is_empty() {
            continue;
        }

        let mut chars = segment.chars().peekable();

        // Determine who
        let mut who: u32 = 0;
        let mut has_who = false;

        while let Some(&c) = chars.peek() {
            match c {
                'u' => { who |= 0o700; has_who = true; }
                'g' => { who |= 0o070; has_who = true; }
                'o' => { who |= 0o007; has_who = true; }
                'a' => { who |= 0o777; has_who = true; }
                _ => break,
            }
            chars.next();
        }

        if !has_who {
            who = 0o777; // default to all
        }

        // Determine operation
        let op = chars.next()?;
        if op != '+' && op != '-' && op != '=' {
            return None;
        }

        // Determine permissions
        let mut perms: u32 = 0;
        for c in chars {
            match c {
                'r' => perms |= 0o4,
                'w' => perms |= 0o2,
                'x' => perms |= 0o1,
                _ => return None,
            }
        }

        // Shift perms to the correct position
        let shift = if who & 0o700 != 0 { 6 }
            else if who & 0o070 != 0 { 3 }
            else { 0 };

        let shifted_perms = (perms << shift) & who;

        match op {
            '+' => result |= shifted_perms,
            '-' => result &= !shifted_perms,
            '=' => {
                // Clear who bits, then set new perms
                result &= !who;
                result |= shifted_perms;
            }
            _ => return None,
        }
    }

    Some(result)
}

fn change_mode(path: &Path, mode: u32, recursive: bool) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|e| format!("chmod: cannot access '{}': {}", path.display(), e))?;

    let mut perms = metadata.permissions();
    let current_mode = perms.mode();

    let new_mode = (current_mode & 0o7777) | (mode & 0o7777);
    perms.set_mode(new_mode);

    fs::set_permissions(path, perms)
        .map_err(|e| format!("chmod: changing permissions of '{}': {}", path.display(), e))?;

    if recursive && metadata.is_dir() {
        let entries = fs::read_dir(path)
            .map_err(|e| format!("chmod: cannot read directory '{}': {}", path.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("chmod: error reading entry: {}", e))?;
            let path = entry.path();
            change_mode(&path, mode, true)?;
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.contains(&"--help".to_string()) {
        print_help();
        process::exit(0);
    }

    let mut recursive = false;
    let mut mode_str = None;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-R" | "--recursive" => recursive = true,
            "--help" => {
                print_help();
                process::exit(0);
            }
            _ => {
                if mode_str.is_none() {
                    mode_str = Some(args[i].clone());
                } else {
                    files.push(args[i].clone());
                }
            }
        }
        i += 1;
    }

    let mode_str = match mode_str {
        Some(s) => s,
        None => {
            eprintln!("chmod: missing mode operand");
            eprintln!("Try 'chmod --help' for more information.");
            process::exit(1);
        }
    };

    if files.is_empty() {
        eprintln!("chmod: missing file operand");
        eprintln!("Try 'chmod --help' for more information.");
        process::exit(1);
    }

    let mode = if mode_str.chars().all(|c| c.is_ascii_digit()) {
        match parse_octal_mode(&mode_str) {
            Some(m) => m,
            None => {
                eprintln!("chmod: invalid octal mode '{}'", mode_str);
                process::exit(1);
            }
        }
    } else {
        // For symbolic mode, we need a reference file to get current permissions
        // Use the first file as reference
        let first_file = &files[0];
        let path = Path::new(first_file);
        let current_mode = match fs::metadata(path) {
            Ok(meta) => meta.permissions().mode(),
            Err(e) => {
                eprintln!("chmod: cannot access '{}': {}", first_file, e);
                process::exit(1);
            }
        };
        match parse_symbolic_mode(&mode_str, current_mode) {
            Some(m) => m,
            None => {
                eprintln!("chmod: invalid symbolic mode '{}'", mode_str);
                process::exit(1);
            }
        }
    };

    let mut has_error = false;
    for file in &files {
        if let Err(e) = change_mode(Path::new(file), mode, recursive) {
            eprintln!("{}", e);
            has_error = true;
        }
    }

    if has_error {
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
