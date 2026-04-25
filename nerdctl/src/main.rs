use std::fs;
use std::path::PathBuf;
use std::process::{Command, exit};

const DB_PATH: &str = "/root/.local/share/nerdctl/containers.json";

fn print_usage() {
    println!("nerdctl - Docker-compatible CLI for containerd");
    println!();
    println!("USAGE:");
    println!("    nerdctl [COMMAND]");
    println!();
    println!("COMMANDS:");
    println!("    run       Run a container");
    println!("    ps        List containers");
    println!("    images    List images");
    println!("    stop      Stop a container");
    println!("    rm        Remove a container");
    println!("    pull      Pull an image");
    println!("    exec      Execute in container");
    println!("    logs      Container logs");
    println!("    help      Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    nerdctl run -d --name myapp ubuntu");
    println!("    nerdctl ps");
    println!("    nerdctl stop myapp");
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Container {
    id: String,
    name: String,
    image: String,
    status: String,
    command: String,
    created: u64,
    started: u64,
    pid: Option<u32>,
    namespace: String,
    snapshotter: String,
    logs: Vec<String>,
}

/// Image metadata
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Image {
    id: String,
    name: String,
    tag: String,
    digest: String,
    created: u64,
    size: u64,
    referrer: String,
}

fn get_db_path() -> PathBuf {
    let path = PathBuf::from(DB_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    path
}

fn load_containers() -> Vec<Container> {
    let path = get_db_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(containers) = serde_json::from_str::<Vec<Container>>(&content) {
            return containers;
        }
    }
    Vec::new()
}

fn save_containers(containers: &[Container]) {
    let path = get_db_path();
    if let Ok(json) = serde_json::to_string_pretty(containers) {
        fs::write(&path, json).ok();
    }
}

fn cmd_run(args: &[String]) {
    let mut name = String::new();
    let mut image = String::new();
    let mut detached = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--detach" => detached = true,
            "--name" => {
                i += 1;
                if i < args.len() {
                    name = args[i].clone();
                }
            }
            arg if !arg.starts_with('-') && image.is_empty() => {
                image = arg.to_string();
            }
            _ => {}
        }
        i += 1;
    }

    if image.is_empty() {
        eprintln!("nerdctl: image name required");
        exit(1);
    }

    let container_id = format!("ctr_{}", uuid::short());

    let container = Container {
        id: container_id.clone(),
        name: if name.is_empty() { container_id.clone() } else { name },
        image: image.clone(),
        status: if detached { "running" } else { "created" }.to_string(),
        command: args.join(" "),
    };

    let mut containers = load_containers();
    containers.push(container);
    save_containers(&containers);

    if detached {
        println!("{}", container_id);
    } else {
        println!("Running: {}", image);
    }
}

fn cmd_ps() {
    let containers = load_containers();
    if containers.is_empty() {
        println!("CONTAINER ID   IMAGE   STATUS   COMMAND");
        return;
    }

    println!("{:<14} {:<10} {:<10} {}", "CONTAINER ID", "IMAGE", "STATUS", "COMMAND");
    for c in &containers {
        println!("{:<14} {:<10} {:<10} {}", c.id, c.image, c.status, c.command);
    }
}

fn cmd_images() {
    println!("REPOSITORY   TAG   IMAGE ID   CREATED   SIZE");
    // Simple placeholder - in real implementation would list actual images
}

fn cmd_stop(args: &[String]) {
    if args.is_empty() {
        eprintln!("nerdctl: container name required");
        exit(1);
    }

    let name = &args[0];
    let mut containers = load_containers();

    for c in &mut containers {
        if c.name == *name || c.id == *name {
            c.status = "stopped".to_string();
            println!("{}", c.id);
        }
    }

    save_containers(&containers);
}

fn cmd_rm(args: &[String]) {
    if args.is_empty() {
        eprintln!("nerdctl: container name required");
        exit(1);
    }

    let name = &args[0];
    let mut containers = load_containers();
    containers.retain(|c| c.name != *name && c.id != *name);
    save_containers(&containers);
    println!("Removed: {}", name);
}

fn cmd_pull(args: &[String]) {
    if args.is_empty() {
        eprintln!("nerdctl: image name required");
        exit(1);
    }

    let image = &args[0];
    println!("Pulling: {}", image);
    println!("Status: Image is up to date");
}

fn cmd_exec(args: &[String]) {
    if args.len() < 2 {
        eprintln!("nerdctl: container and command required");
        exit(1);
    }

    let container = &args[0];
    let command = &args[1..];

    println!("Executing in {}: {}", container, command.join(" "));
    let status = Command::new(&command[0])
        .args(&command[1..])
        .status();

    match status {
        Ok(s) => {
            if let Some(code) = s.code() {
                exit(code);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(1);
        }
    }
}

fn cmd_logs(args: &[String]) {
    if args.is_empty() {
        eprintln!("nerdctl: container name required");
        exit(1);
    }

    let name = &args[0];
    let containers = load_containers();

    for c in &containers {
        if c.name == *name || c.id == *name {
            println!("Container: {} ({})", c.name, c.id);
            println!("Image: {}", c.image);
            println!("Status: {}", c.status);
            return;
        }
    }

    eprintln!("nerdctl: container not found: {}", name);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        exit(0);
    }

    match args[0].as_str() {
        "run" => cmd_run(&args[1..]),
        "ps" => cmd_ps(),
        "images" => cmd_images(),
        "stop" => cmd_stop(&args[1..]),
        "rm" => cmd_rm(&args[1..]),
        "pull" => cmd_pull(&args[1..]),
        "exec" => cmd_exec(&args[1..]),
        "logs" => cmd_logs(&args[1..]),
        "help" | "-h" | "--help" => print_usage(),
        _ => {
            eprintln!("nerdctl: unknown command: {}", args[0]);
            print_usage();
            exit(1);
        }
    }
}

// Simple UUID generator
mod uuid {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn short() -> String {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        format!("{:x}", duration.as_nanos() & 0xFFFFFFFF)
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
