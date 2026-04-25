use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn print_help() {
    println!("chown - Change file owner and group");
    println!();
    println!("USAGE:");
    println!("    chown [OPTIONS] OWNER[:GROUP] FILE...");
    println!();
    println!("OPTIONS:");
    println!("    -R, --recursive    Change files and directories recursively");
    println!("    --help             Print this help message");
    println!();
    println!("OWNER[:GROUP]:");
    println!("    Specify the new owner and optionally the new group");
    println!("    Examples:");
    println!("        chown user file          - Change only owner");
    println!("        chown user:group file    - Change owner and group");
    println!("        chown user: file         - Change owner and primary group of user");
    println!("        chown :group file        - Change only group");
}

fn parse_owner_group(spec: &str) -> (Option<String>, Option<String>) {
    if let Some(colon_pos) = spec.find(':') {
        let user = &spec[..colon_pos];
        let group = &spec[colon_pos + 1..];
        let user = if user.is_empty() { None } else { Some(user.to_string()) };
        let group = if group.is_empty() { None } else { Some(group.to_string()) };
        (user, group)
    } else {
        (Some(spec.to_string()), None)
    }
}

fn lookup_uid(username: &str) -> Option<u32> {
    // Try parsing as numeric UID first
    if let Ok(uid) = username.parse::<u32>() {
        return Some(uid);
    }

    // Read /etc/passwd to find the UID
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    for line in passwd.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 && parts[0] == username {
            return parts[2].parse::<u32>().ok();
        }
    }
    None
}

fn lookup_gid(groupname: &str) -> Option<u32> {
    // Try parsing as numeric GID first
    if let Ok(gid) = groupname.parse::<u32>() {
        return Some(gid);
    }

    // Read /etc/group to find the GID
    let group = fs::read_to_string("/etc/group").ok()?;
    for line in group.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 && parts[0] == groupname {
            return parts[2].parse::<u32>().ok();
        }
    }
    None
}

fn change_ownership(path: &Path, uid: Option<u32>, gid: Option<u32>, recursive: bool) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|e| format!("chown: cannot access '{}': {}", path.display(), e))?;

    // Use nix crate functionality via libc
    let path_c = std::ffi::CString::new(path.to_str().ok_or("Invalid path")?)
        .map_err(|e| format!("chown: invalid path: {}", e))?;

    let uid = uid.unwrap_or_else(|| {
        use std::os::unix::fs::MetadataExt;
        metadata.uid()
    });

    let gid = gid.unwrap_or_else(|| {
        use std::os::unix::fs::MetadataExt;
        metadata.gid()
    });

    unsafe {
        let result = libc::chown(path_c.as_ptr(), uid, gid);
        if result != 0 {
            return Err(format!(
                "chown: changing ownership of '{}': {}",
                path.display(),
                std::io::Error::last_os_error()
            ));
        }
    }

    if recursive && metadata.is_dir() {
        let entries = fs::read_dir(path)
            .map_err(|e| format!("chown: cannot read directory '{}': {}", path.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("chown: error reading entry: {}", e))?;
            let entry_path = entry.path();
            change_ownership(&entry_path, Some(uid), Some(gid), true)?;
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
    let mut owner_spec = None;
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
                if owner_spec.is_none() {
                    owner_spec = Some(args[i].clone());
                } else {
                    files.push(args[i].clone());
                }
            }
        }
        i += 1;
    }

    let owner_spec = match owner_spec {
        Some(s) => s,
        None => {
            eprintln!("chown: missing operand");
            eprintln!("Try 'chown --help' for more information.");
            process::exit(1);
        }
    };

    if files.is_empty() {
        eprintln!("chown: missing file operand");
        eprintln!("Try 'chown --help' for more information.");
        process::exit(1);
    }

    let (user, group) = parse_owner_group(&owner_spec);

    let uid = match user {
        Some(u) => match lookup_uid(&u) {
            Some(uid) => Some(uid),
            None => {
                eprintln!("chown: invalid user: '{}'", u);
                process::exit(1);
            }
        },
        None => None,
    };

    let gid = match group {
        Some(g) => match lookup_gid(&g) {
            Some(gid) => Some(gid),
            None => {
                eprintln!("chown: invalid group: '{}'", g);
                process::exit(1);
            }
        },
        None => None,
    };

    let mut has_error = false;
    for file in &files {
        let path = Path::new(file);
        if let Err(e) = change_ownership(path, uid, gid, recursive) {
            eprintln!("{}", e);
            has_error = true;
        }
    }

    if has_error {
        process::exit(1);
    }
}
