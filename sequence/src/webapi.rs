use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;

/// Web API Server - HTTP interface for SEQUENCE OS
pub struct WebServer {
    pub port: u16,
    pub running: bool,
    pub dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: String,
    pub data: serde_json::Value,
}

impl WebServer {
    pub fn new(port: u16, dir: &str) -> Self {
        WebServer {
            port,
            running: false,
            dir: dir.to_string(),
        }
    }

    /// Start the web server (blocking)
    pub fn start(&mut self) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

        self.running = true;
        println!("🌐 Web server started on http://{}", addr);

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buffer = [0; 1024];
                    let bytes_read = match stream.read(&mut buffer) {
                        Ok(n) => n,
                        Err(_) => continue,
                    };

                    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                    let response = self.handle_request(&request);

                    let http_response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        response.len(),
                        response
                    );

                    let _ = stream.write_all(http_response.as_bytes());
                }
                Err(_) => continue,
            }
        }

        Ok(())
    }

    /// Handle an HTTP request
    fn handle_request(&self, request: &str) -> String {
        let lines: Vec<&str> = request.lines().collect();
        if lines.is_empty() {
            return self.json_error("Empty request");
        }

        let request_line = lines[0];
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return self.json_error("Invalid request");
        }

        let method = parts[0];
        let path = parts[1];

        // Parse query parameters
        let path_parts: Vec<&str> = path.splitn(2, '?').collect();
        let route = path_parts[0];
        let query = path_parts.get(1).unwrap_or(&"");

        match (method, route) {
            ("GET", "/api/status") => self.api_status(),
            ("GET", "/api/health") => self.json_success(serde_json::json!({ "status": "ok" })),
            ("GET", "/api/tools") => self.api_tools(),
            ("GET", "/api/memory") => self.api_memory(query),
            ("POST", "/api/execute") => self.api_execute(request),
            ("GET", "/api/docs") => self.api_docs(),
            _ => self.json_error(&format!("Not found: {} {}", method, route)),
        }
    }

    fn api_status(&self) -> String {
        let output = Command::new("evolve").arg("diagnose").output();
        let status = match output {
            Ok(result) => String::from_utf8_lossy(&result.stdout).to_string(),
            Err(e) => format!("Error: {}", e),
        };

        self.json_success(serde_json::json!({
            "server": "running",
            "port": self.port,
            "tools_status": status
        }))
    }

    fn api_tools(&self) -> String {
        let output = Command::new("ls").args(&["/root/.cargo/bin/"]).output();
        let tools: Vec<String> = match output {
            Ok(result) => {
                let list = String::from_utf8_lossy(&result.stdout);
                list.lines()
                    .filter(|t| !t.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            }
            Err(_) => vec![],
        };

        self.json_success(serde_json::json!({
            "tools": tools,
            "count": tools.len()
        }))
    }

    fn api_memory(&self, query: &str) -> String {
        let memory_path = format!("{}/.sequence/memory/episodic", self.dir);
        let entries = if let Ok(entries) = fs::read_dir(&memory_path) {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        self.json_success(serde_json::json!({
            "entries": entries,
            "count": entries.len(),
            "query": query
        }))
    }

    fn api_execute(&self, _request: &str) -> String {
        // Parse JSON body for command to execute
        self.json_success(serde_json::json!({
            "message": "Execute endpoint - use /run command instead"
        }))
    }

    fn api_docs(&self) -> String {
        self.json_success(serde_json::json!({
            "endpoints": {
                "GET /api/status": "System status",
                "GET /api/health": "Health check",
                "GET /api/tools": "List all tools",
                "GET /api/memory": "List memory entries",
                "POST /api/execute": "Execute a command",
                "GET /api/docs": "This documentation"
            }
        }))
    }

    fn json_success(&self, data: serde_json::Value) -> String {
        serde_json::to_string(&ApiResponse {
            status: "success".to_string(),
            data,
        }).unwrap_or_else(|_| "{}".to_string())
    }

    fn json_error(&self, message: &str) -> String {
        serde_json::to_string(&ApiResponse {
            status: "error".to_string(),
            data: serde_json::json!({ "message": message }),
        }).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_success() {
        let server = WebServer::new(8080, "/tmp");
        let response = server.json_success(serde_json::json!({ "key": "value" }));
        let parsed: ApiResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed.status, "success");
    }

    #[test]
    fn test_json_error() {
        let server = WebServer::new(8080, "/tmp");
        let response = server.json_error("test error");
        let parsed: ApiResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed.status, "error");
    }

    #[test]
    fn test_handle_health() {
        let server = WebServer::new(8080, "/tmp");
        let request = "GET /api/health HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = server.handle_request(request);
        assert!(response.contains("\"status\":\"ok\""));
    }

    #[test]
    fn test_handle_docs() {
        let server = WebServer::new(8080, "/tmp");
        let request = "GET /api/docs HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = server.handle_request(request);
        assert!(response.contains("endpoints"));
    }

    #[test]
    fn test_handle_not_found() {
        let server = WebServer::new(8080, "/tmp");
        let request = "GET /unknown HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = server.handle_request(request);
        assert!(response.contains("Not found"));
    }
}
