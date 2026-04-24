use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use std::thread;

/// 服务器配置
#[derive(Clone)]
struct Config {
    port: u16,
    root: String,
    /// 是否启用日志
    logging: bool,
    /// 工作线程数
    workers: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8080,
            root: ".".to_string(),
            logging: true,
            workers: 4,
        }
    }
}

fn main() -> io::Result<()> {
    let config = parse_args();

    let addr = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(&addr)?;

    println!("🚀 HTTP Server running at http://{}", addr);
    println!("📁 Serving files from: {}", fs::canonicalize(&config.root)?.display());
    println!("👷 Workers: {}", config.workers);
    println!("Press Ctrl+C to stop\n");

    // 创建线程池处理连接
    let (tx, rx) = std::sync::mpsc::channel::<TcpStream>();
    let rx = std::sync::Arc::new(std::sync::Mutex::new(rx));

    let mut handles = Vec::with_capacity(config.workers);
    for _ in 0..config.workers {
        let rx = rx.clone();
        let config = config.clone();
        let handle = thread::spawn(move || {
            loop {
                let stream = rx.lock().unwrap().recv();
                match stream {
                    Ok(stream) => {
                        if let Err(e) = handle_connection(stream, &config) {
                            eprintln!("Connection handler error: {}", e);
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        handles.push(handle);
    }

    // 主线程接受连接并分发给工作线程
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if tx.send(stream).is_err() {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
            }
        }
    }

    drop(tx);
    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args() -> Config {
    let mut config = Config::default();

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    config.port = args[i + 1].parse().unwrap_or(8080);
                    i += 1;
                }
            }
            "--root" | "-r" => {
                if i + 1 < args.len() {
                    config.root = args[i + 1].clone();
                    i += 1;
                }
            }
            "--workers" | "-w" => {
                if i + 1 < args.len() {
                    config.workers = args[i + 1].parse().unwrap_or(4);
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("HTTP Server");
                println!("Usage: httpserver [OPTIONS]");
                println!("Options:");
                println!("  -p, --port <PORT>    Port number (default: 8080)");
                println!("  -r, --root <DIR>     Root directory (default: .)");
                println!("  -w, --workers <N>    Number of worker threads (default: 4)");
                println!("  --no-log             Disable access logging");
                println!("  -h, --help           Show help");
                std::process::exit(0);
            }
            "--no-log" => {
                config.logging = false;
            }
            _ => {}
        }
        i += 1;
    }

    config
}

/// 处理 HTTP 连接
fn handle_connection(mut stream: TcpStream, config: &Config) -> io::Result<()> {
    let peer_addr = stream.peer_addr().ok();
    let start_time = SystemTime::now();
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    // 读取请求行
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return Ok(());
    }

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    // 读取并忽略其他请求头
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_ok() && line.trim().is_empty() {
            break;
        }
    }

    // 处理请求
    let (status, status_text, body, is_file) = handle_request(method, path, &config.root);

    // 构建响应
    let content_type = if is_file {
        get_mime_type(path)
    } else {
        "text/html"
    };

    let response = if method == "HEAD" {
        // HEAD 请求只返回头部，不返回 body
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            status, status_text, content_type, body.len()
        )
    } else {
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status, status_text, content_type, body.len(), body
        )
    };

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    // 记录日志
    if config.logging {
        let elapsed = start_time.elapsed().unwrap_or_default();
        let method_str = if method == "HEAD" { "HEAD" } else { "GET" };
        log_access(peer_addr, method_str, path, status, elapsed.as_micros() as u64);
    }

    Ok(())
}

/// 处理 HTTP 请求
fn handle_request(method: &str, path: &str, root: &str) -> (u16, String, String, bool) {
    if method != "GET" && method != "HEAD" {
        return (405, "Method Not Allowed".to_string(), "Method Not Allowed".to_string(), false);
    }
    
    // URL 解码和路径清理
    let decoded_path = url_decode(path);
    let clean_path = if decoded_path == "/" {
        "index.html".to_string()
    } else {
        decoded_path.trim_start_matches('/').to_string()
    };
    
    let full_path = Path::new(root).join(&clean_path);
    
    // 安全检查：防止目录遍历攻击
    let canonical_root = fs::canonicalize(root).unwrap_or_else(|_| Path::new(root).to_path_buf());
    let canonical_path = fs::canonicalize(&full_path).ok();
    
    if let Some(ref cp) = canonical_path {
        if !cp.starts_with(&canonical_root) {
            return (403, "Forbidden".to_string(), "Forbidden".to_string(), false);
        }
    }
    
    // 检查文件/目录是否存在
    if let Some(meta) = fs::metadata(&full_path).ok() {
        if meta.is_dir() {
            // 目录列表
            let (status, status_text, body) = list_directory(&full_path, path);
            return (status, status_text, body, false);
        } else {
            // 文件内容
            let (status, status_text, body) = serve_file(&full_path);
            return (status, status_text, body, true);
        }
    }

    (404, "Not Found".to_string(), "404 Not Found".to_string(), false)
}

/// URL 解码
fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    
    result
}

/// 列出目录内容
fn list_directory(path: &Path, url_path: &str) -> (u16, String, String) {
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return (403, "Forbidden".to_string(), "Forbidden".to_string()),
    };

    let mut files = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
        files.push((name, is_dir));
    }

    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut html = String::new();
    html.push_str(&format!("<html><head><title>Index of {}</title></head><body>", url_path));
    html.push_str(&format!("<h1>Index of {}</h1>", url_path));
    html.push_str("<ul>");

    if url_path != "/" {
        html.push_str("<li><a href=\"../\">..</a>/</li>");
    }

    for (name, is_dir) in files {
        let suffix = if is_dir { "/" } else { "" };
        html.push_str(&format!("<li><a href=\"{}{}\">{}</a>{}</li>", name, suffix, name, if is_dir { "/" } else { "" }));
    }

    html.push_str("</ul></body></html>");

    (200, "OK".to_string(), html)
}

/// 提供文件内容
fn serve_file(path: &Path) -> (u16, String, String) {
    match fs::read_to_string(path) {
        Ok(content) => (200, "OK".to_string(), content),
        Err(_) => {
            // 可能是二进制文件，尝试读取原始字节
            match fs::read(path) {
                Ok(bytes) => (200, "OK".to_string(), String::from_utf8_lossy(&bytes).to_string()),
                Err(_) => (404, "Not Found".to_string(), "404 Not Found".to_string()),
            }
        }
    }
}

/// 获取文件的 MIME 类型
fn get_mime_type(path: &str) -> &'static str {
    let path_lower = path.to_lowercase();
    let ext = std::path::Path::new(&path_lower)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "xml" => "application/xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// 记录访问日志
fn log_access(peer_addr: Option<std::net::SocketAddr>, method: &str, path: &str, status: u16, elapsed_micros: u64) {
    let timestamp = chrono::Local::now().format("%d/%b/%Y %H:%M:%S").to_string();
    let addr_str = peer_addr.map(|a| a.ip().to_string()).unwrap_or_else(|| "-".to_string());
    println!("{} - {} \"{} {}\" {} {}μs", addr_str, timestamp, method, path, status, elapsed_micros);
}
