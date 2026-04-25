use std::fs;
use std::path::PathBuf;
use std::process::{Command, exit};

const DB_PATH: &str = "/root/.local/share/podman/containers.json";

fn print_usage() {
    println!("podman - Daemonless container engine");
    println!();
    println!("USAGE:");
    println!("    podman [COMMAND]");
    println!();
    println!("COMMANDS:");
    println!("    run       Run a container");
    println!("    ps        List containers");
    println!("    images    List images");
    println!("    stop      Stop a container");
    println!("    rm        Remove a container");
    println!("    help      Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    podman run -d --name myapp ubuntu");
    println!("    podman ps");
    println!("    podman stop myapp");
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Container {
    id: String,
    name: String,
    image: String,
    status: String,
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

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    format!("{:x}", duration.as_nanos() & 0xFFFFFFFF)
}

fn cmd_run(args: &[String]) {
    let mut name = String::new();
    let mut image = String::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--detach" => {}
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
        eprintln!("podman: image name required");
        exit(1);
    }

    let container_id = generate_id();
    let container = Container {
        id: container_id.clone(),
        name: if name.is_empty() { container_id.clone() } else { name },
        image,
        status: "running".to_string(),
    };

    let mut containers = load_containers();
    containers.push(container);
    save_containers(&containers);
    println!("{}", container_id);
}

fn cmd_ps() {
    let containers = load_containers();
    println!("{:<14} {:<10} {:<10}", "CONTAINER ID", "IMAGE", "STATUS");
    for c in &containers {
        println!("{:<14} {:<10} {:<10}", c.id, c.image, c.status);
    }
}

fn cmd_images() {
    println!("REPOSITORY   TAG   IMAGE ID   CREATED   SIZE");
}

fn cmd_stop(args: &[String]) {
    if args.is_empty() {
        eprintln!("podman: container name required");
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
        eprintln!("podman: container name required");
        exit(1);
    }

    let name = &args[0];
    let mut containers = load_containers();
    containers.retain(|c| c.name != *name && c.id != *name);
    save_containers(&containers);
    println!("Removed: {}", name);
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
        "help" | "-h" | "--help" => print_usage(),
        _ => {
            eprintln!("podman: unknown command: {}", args[0]);
            print_usage();
            exit(1);
        }
    }
}
