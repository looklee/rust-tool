use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{self, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Container database path
const CONTAINER_DB: &str = "/tmp/docker-containers.json";
/// Image database path
const IMAGE_DB: &str = "/tmp/docker-images.json";
/// Container log directory
const LOG_DIR: &str = "/tmp/docker-logs";

/// Container state
#[derive(Debug, Clone)]
struct Container {
    id: String,
    name: String,
    image: String,
    command: Vec<String>,
    status: String,
    created: u64,
    started: u64,
    pid: Option<u32>,
    logs: Vec<String>,
}

/// Image metadata
#[derive(Debug, Clone)]
struct Image {
    id: String,
    name: String,
    tag: String,
    created: u64,
    size: u64,
}

/// Generate a short container ID
fn generate_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now % (1u128 << 32))
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Format duration
fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{} seconds ago", seconds)
    } else if seconds < 3600 {
        format!("{} minutes ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{} hours ago", seconds / 3600)
    } else {
        format!("{} days ago", seconds / 86400)
    }
}

/// Load containers from database
fn load_containers() -> Vec<Container> {
    let path = Path::new(CONTAINER_DB);
    if !path.exists() {
        return Vec::new();
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut containers = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.is_empty() {
            continue;
        }

        // Simple JSON-like parsing (no external deps)
        let mut fields: HashMap<String, String> = HashMap::new();
        let line = line.trim_matches(|c| c == '{' || c == '}');

        for part in line.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once(':') {
                let key = key.trim().trim_matches('"');
                let value = value.trim().trim_matches('"');
                fields.insert(key.to_string(), value.to_string());
            }
        }

        if let (Some(id), Some(name), Some(image)) = (
            fields.get("id"),
            fields.get("name"),
            fields.get("image"),
        ) {
            let status = fields.get("status").cloned().unwrap_or_else(|| "unknown".to_string());
            let created = fields
                .get("created")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let started = fields
                .get("started")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let pid = fields
                .get("pid")
                .and_then(|v| v.parse().ok());

            let logs = load_container_logs(id);
            
            let command_vec = fields.get("command").map(|s| {
                s.split_whitespace().map(String::from).collect()
            }).unwrap_or_default();

            containers.push(Container {
                id: id.clone(),
                name: name.clone(),
                image: image.clone(),
                command: command_vec,
                status,
                created,
                started,
                pid,
                logs,
            });
        }
    }

    containers
}

/// Save containers to database
fn save_containers(containers: &[Container]) {
    let path = Path::new(CONTAINER_DB);
    let mut file = File::create(path).expect("Failed to create container database");

    for c in containers {
        let pid_str = c.pid.map(|p| p.to_string()).unwrap_or_else(|| "none".to_string());
        let cmd_str = c.command.join(" ");
        writeln!(
            file,
            "{{\"id\":\"{}\",\"name\":\"{}\",\"image\":\"{}\",\"command\":\"{}\",\"status\":\"{}\",\"created\":{},\"started\":{},\"pid\":\"{}\"}}",
            c.id, c.name, c.image, cmd_str, c.status, c.created, c.started, pid_str
        ).expect("Failed to write container");
    }
}

/// Load container logs
fn load_container_logs(id: &str) -> Vec<String> {
    let log_path = format!("{}/{}.log", LOG_DIR, id);
    let path = Path::new(&log_path);
    if !path.exists() {
        return Vec::new();
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    reader.lines().filter_map(|l| l.ok()).collect()
}

/// Save container log
fn save_container_log(id: &str, message: &str) {
    fs::create_dir_all(LOG_DIR).ok();
    let log_path = format!("{}/{}.log", LOG_DIR, id);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .expect("Failed to open log file");
    writeln!(file, "{}", message).ok();
}

/// Load images from database
fn load_images() -> Vec<Image> {
    let path = Path::new(IMAGE_DB);
    if !path.exists() {
        return Vec::new();
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut images = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.is_empty() {
            continue;
        }

        let mut fields: HashMap<String, String> = HashMap::new();
        let line = line.trim_matches(|c| c == '{' || c == '}');

        for part in line.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once(':') {
                let key = key.trim().trim_matches('"');
                let value = value.trim().trim_matches('"');
                fields.insert(key.to_string(), value.to_string());
            }
        }

        if let (Some(id), Some(name), Some(tag)) = (
            fields.get("id"),
            fields.get("name"),
            fields.get("tag"),
        ) {
            let created = fields
                .get("created")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let size = fields
                .get("size")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);

            images.push(Image {
                id: id.clone(),
                name: name.clone(),
                tag: tag.clone(),
                created,
                size,
            });
        }
    }

    images
}

/// Save images to database
fn save_images(images: &[Image]) {
    let path = Path::new(IMAGE_DB);
    let mut file = File::create(path).expect("Failed to create image database");

    for img in images {
        writeln!(
            file,
            "{{\"id\":\"{}\",\"name\":\"{}\",\"tag\":\"{}\",\"created\":{},\"size\":{}}}",
            img.id, img.name, img.tag, img.created, img.size
        ).expect("Failed to write image");
    }
}

/// Check if a process is running
fn is_process_running(pid: u32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}

/// Update container statuses based on actual process state
fn update_container_statuses(containers: &mut Vec<Container>) {
    for c in containers {
        if c.status == "running" {
            if let Some(pid) = c.pid {
                if !is_process_running(pid) {
                    c.status = "exited".to_string();
                    save_container_log(&c.id, &format!("Container {} exited (PID {})", &c.id[..12], pid));
                }
            }
        }
    }
}

/// Run a command in a new namespace (simplified container)
fn run_in_namespace(
    cmd: &[String],
    container_id: &str,
    container_name: &str,
) -> io::Result<Option<u32>> {
    if cmd.is_empty() {
        return Ok(None);
    }

    let binary = &cmd[0];
    let args = &cmd[1..];

    // Try to use unshare for namespace isolation
    let mut command = Command::new("unshare");
    command
        .arg("--pid")
        .arg("--fork")
        .arg("--mount-proc")
        .arg(binary);

    for arg in args {
        command.arg(arg);
    }

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let child = match command.spawn() {
        Ok(c) => c,
        Err(_) => {
            // Fallback: run without namespace isolation
            let mut fallback = Command::new(binary);
            for arg in args {
                fallback.arg(arg);
            }
            fallback.stdout(Stdio::piped());
            fallback.stderr(Stdio::piped());
            fallback.spawn()?
        }
    };

    let pid = child.id();

    // Log container start
    save_container_log(
        container_id,
        &format!(
            "Container {} ({}) started with PID {}",
            &container_id[..12.min(container_id.len())],
            container_name,
            pid
        ),
    );

    Ok(Some(pid))
}

/// Print help message
fn print_help() {
    println!("Docker - Simplified Container Manager");
    println!();
    println!("Usage: docker [COMMAND] [OPTIONS]");
    println!();
    println!("Commands:");
    println!("  run       Run a new container");
    println!("  ps        List running containers");
    println!("  images    List available images");
    println!("  stop      Stop a running container");
    println!("  rm        Remove a container");
    println!("  logs      View container logs");
    println!("  exec      Execute a command in a running container");
    println!("  pull      Pull an image");
    println!("  --help    Show this help message");
    println!();
    println!("Examples:");
    println!("  docker run -d --name myapp ubuntu /bin/sh -c 'while true; do echo hello; sleep 1; done'");
    println!("  docker ps");
    println!("  docker logs myapp");
    println!("  docker stop myapp");
    println!("  docker rm myapp");
}

/// Command: run
fn cmd_run(args: &[String]) {
    let mut detached = false;
    let mut name = String::new();
    let mut image = String::new();
    let mut command = Vec::new();
    let mut parsing_command = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--detach" => {
                detached = true;
            }
            "-n" | "--name" => {
                i += 1;
                if i < args.len() {
                    name = args[i].clone();
                }
            }
            "--" => {
                parsing_command = true;
            }
            _ => {
                if image.is_empty() && !parsing_command {
                    image = args[i].clone();
                } else {
                    command.push(args[i].clone());
                }
            }
        }
        i += 1;
    }

    if image.is_empty() {
        eprintln!("Error: image name is required");
        eprintln!("Usage: docker run [OPTIONS] IMAGE [COMMAND]");
        process::exit(1);
    }

    if command.is_empty() {
        command = vec!["/bin/sh".to_string()];
    }

    // Create image if not exists
    let mut images = load_images();
    if !images.iter().any(|img| img.name == image) {
        let img_id = generate_id();
        images.push(Image {
            id: img_id.clone(),
            name: image.clone(),
            tag: "latest".to_string(),
            created: current_timestamp(),
            size: 0,
        });
        save_images(&images);
        println!("Unable to find image '{}' locally", image);
        println!("Pulling image {}...", image);
        println!("Downloaded new image: {}", img_id);
    }

    // Create container
    let container_id = generate_id();
    let container_name = if name.is_empty() {
        format!("container-{}", &container_id[..8])
    } else {
        name.clone()
    };

    let now = current_timestamp();

    // Run the container
    let pid = match run_in_namespace(&command, &container_id, &container_name) {
        Ok(Some(p)) => Some(p),
        _ => None,
    };

    let mut containers = load_containers();

    let container = Container {
        id: container_id.clone(),
        name: container_name.clone(),
        image: image.clone(),
        command: command.clone(),
        status: if pid.is_some() {
            "running".to_string()
        } else {
            "created".to_string()
        },
        created: now,
        started: now,
        pid,
        logs: Vec::new(),
    };

    containers.push(container);
    save_containers(&containers);

    if detached {
        println!("{}", container_id);
    } else {
        println!("Container {} started", &container_id[..12]);
        println!("Name: {}", container_name);
        println!("Image: {}", image);
        if let Some(p) = pid {
            println!("PID: {}", p);
        }
    }
}

/// Command: ps
fn cmd_ps(args: &[String]) {
    let mut show_all = false;
    for arg in args {
        if arg == "-a" || arg == "--all" {
            show_all = true;
        }
    }

    let mut containers = load_containers();
    update_container_statuses(&mut containers);
    save_containers(&containers);

    if !show_all {
        containers.retain(|c| c.status == "running");
    }

    if containers.is_empty() {
        println!("CONTAINER ID   NAME   IMAGE   STATUS   CREATED");
        return;
    }

    println!(
        "{:<14} {:<20} {:<20} {:<15} {}",
        "CONTAINER ID", "NAME", "IMAGE", "STATUS", "CREATED"
    );

    let now = current_timestamp();
    for c in &containers {
        let short_id = &c.id[..12.min(c.id.len())];
        let created = format_duration(now - c.created);
        println!(
            "{:<14} {:<20} {:<20} {:<15} {}",
            short_id, c.name, c.image, c.status, created
        );
    }
}

/// Command: images
fn cmd_images() {
    let images = load_images();

    if images.is_empty() {
        println!("REPOSITORY   TAG   IMAGE ID   CREATED   SIZE");
        return;
    }

    println!(
        "{:<20} {:<10} {:<14} {:<20} {}",
        "REPOSITORY", "TAG", "IMAGE ID", "CREATED", "SIZE"
    );

    let now = current_timestamp();
    for img in &images {
        let created = format_duration(now - img.created);
        let size_str = if img.size == 0 {
            "0B".to_string()
        } else if img.size < 1024 {
            format!("{}B", img.size)
        } else if img.size < 1024 * 1024 {
            format!("{}KB", img.size / 1024)
        } else {
            format!("{}MB", img.size / (1024 * 1024))
        };
        println!(
            "{:<20} {:<10} {:<14} {:<20} {}",
            img.name, img.tag, &img.id[..12.min(img.id.len())], created, size_str
        );
    }
}

/// Command: stop
fn cmd_stop(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: container name or ID is required");
        eprintln!("Usage: docker stop CONTAINER");
        process::exit(1);
    }

    let target = &args[0];
    let mut containers = load_containers();

    let container_idx = match containers.iter().position(|c| {
        c.name == *target || c.id.starts_with(target)
    }) {
        Some(idx) => idx,
        None => {
            eprintln!("Error: container '{}' not found", target);
            process::exit(1);
        }
    };

    if containers[container_idx].status != "running" {
        println!("Container {} is not running", target);
        return;
    }

    // Try to kill the process
    if let Some(pid) = containers[container_idx].pid {
        let _ = Command::new("kill")
            .arg(pid.to_string())
            .status();
    }

    containers[container_idx].status = "exited".to_string();
    let container_id = containers[container_idx].id.clone();
    save_container_log(&container_id, &format!("Container {} stopped", &container_id[..12]));
    save_containers(&containers);

    println!("{}", container_id);
}

/// Command: rm
fn cmd_rm(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: container name or ID is required");
        eprintln!("Usage: docker rm CONTAINER");
        process::exit(1);
    }

    let target = &args[0];
    let mut containers = load_containers();

    let pos = match containers.iter().position(|c| {
        c.name == *target || c.id.starts_with(target)
    }) {
        Some(p) => p,
        None => {
            eprintln!("Error: container '{}' not found", target);
            process::exit(1);
        }
    };

    let container = &containers[pos];

    if container.status == "running" {
        eprintln!("Error: cannot remove running container {}. Stop it first.", target);
        process::exit(1);
    }

    // Remove log file
    let log_path = format!("{}/{}.log", LOG_DIR, container.id);
    let _ = fs::remove_file(log_path);

    containers.remove(pos);
    save_containers(&containers);

    println!("{}", target);
}

/// Command: logs
fn cmd_logs(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: container name or ID is required");
        eprintln!("Usage: docker logs CONTAINER");
        process::exit(1);
    }

    let target = &args[0];
    let containers = load_containers();

    let container = match containers.iter().find(|c| {
        c.name == *target || c.id.starts_with(target)
    }) {
        Some(c) => c,
        None => {
            eprintln!("Error: container '{}' not found", target);
            process::exit(1);
        }
    };

    if container.logs.is_empty() {
        println!("No logs available for container {}", &container.id[..12.min(container.id.len())]);
    } else {
        for log in &container.logs {
            println!("{}", log);
        }
    }
}

/// Command: exec
fn cmd_exec(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Error: container and command are required");
        eprintln!("Usage: docker exec CONTAINER COMMAND");
        process::exit(1);
    }

    let target = &args[0];
    let exec_cmd = &args[1..];

    let containers = load_containers();

    let container = match containers.iter().find(|c| {
        c.name == *target || c.id.starts_with(target)
    }) {
        Some(c) => c,
        None => {
            eprintln!("Error: container '{}' not found", target);
            process::exit(1);
        }
    };

    if container.status != "running" {
        eprintln!("Error: container {} is not running", target);
        process::exit(1);
    }

    println!("Executing in container {}:", &container.id[..12.min(container.id.len())]);
    println!("Command: {}", exec_cmd.join(" "));

    // Execute command using nsenter if possible
    if let Some(pid) = container.pid {
        let status = Command::new("nsenter")
            .arg("-t")
            .arg(pid.to_string())
            .arg("-p")
            .arg("-u")
            .arg("-n")
            .arg("-i")
            .args(exec_cmd)
            .status();

        match status {
            Ok(s) => {
                if s.success() {
                    println!("Command executed successfully");
                } else {
                    eprintln!("Command failed with exit code");
                }
            }
            Err(e) => {
                eprintln!("Failed to execute: {}", e);
            }
        }
    }
}

/// Command: pull
fn cmd_pull(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: image name is required");
        eprintln!("Usage: docker pull IMAGE");
        process::exit(1);
    }

    let image_name = &args[0];
    let mut images = load_images();

    // Check if already exists
    if images.iter().any(|img| img.name == *image_name) {
        println!("Image {} already exists", image_name);
        return;
    }

    println!("Pulling {}...", image_name);

    // Simulate pull
    let img_id = generate_id();
    images.push(Image {
        id: img_id.clone(),
        name: image_name.clone(),
        tag: "latest".to_string(),
        created: current_timestamp(),
        size: 0,
    });
    save_images(&images);

    println!("Status: Downloaded newer image for {}", image_name);
    println!("Image ID: {}", &img_id[..12]);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        process::exit(0);
    }

    let command = &args[1];
    let sub_args = &args[2..];

    match command.as_str() {
        "--help" | "-h" | "help" => {
            print_help();
        }
        "run" => {
            cmd_run(sub_args);
        }
        "ps" => {
            cmd_ps(sub_args);
        }
        "images" => {
            cmd_images();
        }
        "stop" => {
            cmd_stop(sub_args);
        }
        "rm" => {
            cmd_rm(sub_args);
        }
        "logs" => {
            cmd_logs(sub_args);
        }
        "exec" => {
            cmd_exec(sub_args);
        }
        "pull" => {
            cmd_pull(sub_args);
        }
        _ => {
            eprintln!("docker: '{}' is not a docker command.", command);
            eprintln!("See 'docker --help'");
            process::exit(1);
        }
    }
}
