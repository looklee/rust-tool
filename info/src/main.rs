use chrono::Local;
use serde::Serialize;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

/// 系统信息
#[derive(Serialize)]
struct SystemInfo {
    hostname: String,
    username: String,
    cwd: String,
    os: String,
    arch: String,
    shell: String,
    home: String,
    lang: String,
    timezone: String,
    rust_version: Option<String>,
    cargo_version: Option<String>,
}

/// 项目信息
#[derive(Serialize)]
struct ProjectInfo {
    has_cargo_toml: bool,
    has_package_json: bool,
    has_makefile: bool,
    has_git: bool,
    rust_projects: Vec<String>,
}

/// 输出格式
enum OutputFormat {
    Text,
    Json,
    PrettyJson,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let format = if args.iter().any(|a| a == "--json" || a == "-j") {
        OutputFormat::Json
    } else if args.iter().any(|a| a == "--pretty-json" || a == "-p") {
        OutputFormat::PrettyJson
    } else {
        OutputFormat::Text
    };

    let show_system = args.iter().any(|a| a == "--system" || a == "-s") || args.len() == 1;
    let show_project = args.iter().any(|a| a == "--project" || a == "-P");
    let show_env = args.iter().any(|a| a == "--env" || a == "-e");
    let show_all = args.iter().any(|a| a == "--all" || a == "-a");

    if args.iter().any(|a| a == "--help" || a == "-h") || args.len() == 1 {
        print_help();
        return Ok(());
    }

    let show_all = show_all || (!show_system && !show_project && !show_env);

    if show_all || show_system {
        let sys = get_system_info();
        output_system(&sys, &format)?;
    }

    if show_all || show_project {
        let proj = get_project_info();
        output_project(&proj, &format)?;
    }

    if show_all || show_env {
        output_env(&format)?;
    }

    Ok(())
}

fn get_system_info() -> SystemInfo {
    let hostname = env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string());
    let username = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let cwd = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let os = env::consts::OS.to_string();
    let arch = env::consts::ARCH.to_string();
    let shell = env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());
    let home = env::var("HOME").unwrap_or_else(|_| "unknown".to_string());
    let lang = env::var("LANG").unwrap_or_else(|_| "unknown".to_string());
    let timezone = Local::now().format("%Z").to_string();

    let rust_version = run_command("rustc", "--version");
    let cargo_version = run_command("cargo", "--version");

    SystemInfo {
        hostname,
        username,
        cwd,
        os,
        arch,
        shell,
        home,
        lang,
        timezone,
        rust_version,
        cargo_version,
    }
}

fn get_project_info() -> ProjectInfo {
    let cwd = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

    let has_cargo_toml = cwd.join("Cargo.toml").exists();
    let has_package_json = cwd.join("package.json").exists();
    let has_makefile = cwd.join("Makefile").exists() || cwd.join("makefile").exists();
    let has_git = cwd.join(".git").exists();

    let mut rust_projects = Vec::new();
    if let Ok(entries) = fs::read_dir(&cwd) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.join("Cargo.toml").exists() {
                    if let Some(name) = path.file_name() {
                        rust_projects.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    ProjectInfo {
        has_cargo_toml,
        has_package_json,
        has_makefile,
        has_git,
        rust_projects,
    }
}

fn run_command(cmd: &str, arg: &str) -> Option<String> {
    std::process::Command::new(cmd)
        .arg(arg)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}

fn output_system(sys: &SystemInfo, format: &OutputFormat) -> io::Result<()> {
    match format {
        OutputFormat::Text => {
            println!("System Information:");
            println!("  Hostname:     {}", sys.hostname);
            println!("  Username:     {}", sys.username);
            println!("  CWD:          {}", sys.cwd);
            println!("  OS:           {}", sys.os);
            println!("  Arch:         {}", sys.arch);
            println!("  Shell:        {}", sys.shell);
            println!("  Home:         {}", sys.home);
            println!("  Lang:         {}", sys.lang);
            println!("  Timezone:     {}", sys.timezone);
            if let Some(ref v) = sys.rust_version {
                println!("  Rust:         {}", v);
            }
            if let Some(ref v) = sys.cargo_version {
                println!("  Cargo:        {}", v);
            }
            println!();
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(sys).unwrap());
        }
        OutputFormat::PrettyJson => {
            println!("{}", serde_json::to_string_pretty(sys).unwrap());
        }
    }
    Ok(())
}

fn output_project(proj: &ProjectInfo, format: &OutputFormat) -> io::Result<()> {
    match format {
        OutputFormat::Text => {
            println!("Project Information:");
            println!("  Cargo.toml:     {}", if proj.has_cargo_toml { "yes" } else { "no" });
            println!("  package.json:   {}", if proj.has_package_json { "yes" } else { "no" });
            println!("  Makefile:       {}", if proj.has_makefile { "yes" } else { "no" });
            println!("  .git:           {}", if proj.has_git { "yes" } else { "no" });
            if !proj.rust_projects.is_empty() {
                println!("  Rust projects:  {}", proj.rust_projects.join(", "));
            }
            println!();
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(proj).unwrap());
        }
        OutputFormat::PrettyJson => {
            println!("{}", serde_json::to_string_pretty(proj).unwrap());
        }
    }
    Ok(())
}

fn output_env(format: &OutputFormat) -> io::Result<()> {
    let env_vars: Vec<(String, String)> = env::vars().collect();

    match format {
        OutputFormat::Text => {
            println!("Environment Variables:");
            for (key, value) in &env_vars {
                println!("  {}={}", key, value);
            }
            println!();
        }
        OutputFormat::Json => {
            let map: std::collections::HashMap<_, _> = env_vars.iter().cloned().collect();
            println!("{}", serde_json::to_string(&map).unwrap());
        }
        OutputFormat::PrettyJson => {
            let map: std::collections::HashMap<_, _> = env_vars.iter().cloned().collect();
            println!("{}", serde_json::to_string_pretty(&map).unwrap());
        }
    }
    Ok(())
}

fn print_help() {
    println!("info - display system and project information");
    println!();
    println!("Usage: info [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -s, --system        show system information");
    println!("  -P, --project       show project information");
    println!("  -e, --env           show environment variables");
    println!("  -a, --all           show all information");
    println!("  -j, --json          output as JSON");
    println!("  -p, --pretty-json   output as pretty-printed JSON");
    println!("  -h, --help          show this help message");
    println!();
    println!("With no options, shows all information in text format.");
}
