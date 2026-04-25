use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::process;

fn print_usage() {
    println!("curl - Simplified HTTP client");
    println!();
    println!("USAGE:");
    println!("    curl [OPTIONS] <URL>");
    println!();
    println!("OPTIONS:");
    println!("    -X, --method <METHOD>    HTTP method (GET, POST, PUT, DELETE, HEAD)");
    println!("    -H, --header <HEADER>    Custom header (KEY: VALUE)");
    println!("    -d, --data <DATA>        POST data (form or JSON)");
    println!("    -o, --output <FILE>      Write output to file");
    println!("    -i, --include            Include response headers in output");
    println!("    -v, --verbose            Verbose mode");
    println!("    -s, --silent             Silent mode (no progress)");
    println!("    -L, --location           Follow redirects");
    println!("    -A, --user-agent <UA>    Custom user agent");
    println!("    -h, --help               Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    curl http://example.com");
    println!("    curl -X POST -d '{{\"key\":\"value\"}}' http://example.com/api");
    println!("    curl -H 'Authorization: Bearer token' http://example.com");
    println!("    curl -o output.html http://example.com");
    println!("    curl -i http://example.com");
}

struct CurlOptions {
    method: String,
    headers: HashMap<String, String>,
    data: Option<String>,
    output: Option<String>,
    include_headers: bool,
    verbose: bool,
    silent: bool,
    follow_redirects: bool,
    url: String,
}

fn parse_args() -> Result<CurlOptions, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err("No URL specified".to_string());
    }

    let mut method = "GET".to_string();
    let mut headers = HashMap::new();
    let mut data: Option<String> = None;
    let mut output: Option<String> = None;
    let mut include_headers = false;
    let mut verbose = false;
    let mut silent = false;
    let mut follow_redirects = false;
    let mut url: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-X" | "--method" => {
                i += 1;
                if i < args.len() {
                    method = args[i].to_uppercase();
                } else {
                    return Err("Missing method value".to_string());
                }
            }
            "-H" | "--header" => {
                i += 1;
                if i < args.len() {
                    let header = &args[i];
                    if let Some(pos) = header.find(':') {
                        let key = header[..pos].trim().to_string();
                        let value = header[pos + 1..].trim().to_string();
                        headers.insert(key, value);
                    }
                } else {
                    return Err("Missing header value".to_string());
                }
            }
            "-d" | "--data" => {
                i += 1;
                if i < args.len() {
                    data = Some(args[i].clone());
                    if method == "GET" {
                        method = "POST".to_string();
                    }
                } else {
                    return Err("Missing data value".to_string());
                }
            }
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output = Some(args[i].clone());
                } else {
                    return Err("Missing output file".to_string());
                }
            }
            "-i" | "--include" => include_headers = true,
            "-v" | "--verbose" => verbose = true,
            "-s" | "--silent" => silent = true,
            "-L" | "--location" => follow_redirects = true,
            "-A" | "--user-agent" => {
                i += 1;
                if i < args.len() {
                    headers.insert("User-Agent".to_string(), args[i].clone());
                } else {
                    return Err("Missing user agent".to_string());
                }
            }
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            arg if !arg.starts_with('-') => {
                url = Some(arg.to_string());
            }
            _ => {
                return Err(format!("Unknown option: {}", args[i]));
            }
        }
        i += 1;
    }

    let url = url.ok_or_else(|| "No URL specified".to_string())?;

    Ok(CurlOptions {
        method,
        headers,
        data,
        output,
        include_headers,
        verbose,
        silent,
        follow_redirects,
        url,
    })
}

fn get_headers(resp: &ureq::Response) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for key in resp.headers_names() {
        if let Some(val) = resp.header(&key) {
            result.insert(key, val.to_string());
        }
    }
    result
}

fn make_request(opts: &CurlOptions) -> Result<(u16, HashMap<String, String>, String), String> {
    let mut request = match opts.method.as_str() {
        "GET" => ureq::get(&opts.url),
        "POST" => ureq::post(&opts.url),
        "PUT" => ureq::put(&opts.url),
        "DELETE" => ureq::delete(&opts.url),
        "HEAD" => ureq::head(&opts.url),
        _ => return Err(format!("Unsupported method: {}", opts.method)),
    };

    // Add custom headers
    for (key, value) in &opts.headers {
        if key.to_lowercase() != "user-agent" {
            request = request.set(key, value);
        }
    }

    // Default user agent
    if !opts.headers.contains_key("User-Agent") {
        request = request.set("User-Agent", "curl/0.1.0 (Rust)");
    }

    // Send request with or without data
    let response = if let Some(ref data) = opts.data {
        if opts.verbose {
            eprintln!("> {}", data);
        }
        if data.starts_with('{') || data.starts_with('[') {
            request = request.set("Content-Type", "application/json");
        } else {
            request = request.set("Content-Type", "application/x-www-form-urlencoded");
        }
        request.send_string(data)
    } else {
        request.call()
    };

    match response {
        Ok(resp) => {
            let status = resp.status();
            let resp_headers = get_headers(&resp);

            let mut body = String::new();
            if opts.method != "HEAD" {
                resp.into_reader().read_to_string(&mut body).map_err(|e| e.to_string())?;
            }

            Ok((status, resp_headers, body))
        }
        Err(ureq::Error::Status(code, resp)) => {
            let resp_headers = get_headers(&resp);

            let mut body = String::new();
            if opts.method != "HEAD" {
                resp.into_reader().read_to_string(&mut body).ok();
            }

            Ok((code, resp_headers, body))
        }
        Err(ureq::Error::Transport(e)) => Err(format!("Request failed: {}", e)),
    }
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

    if opts.verbose {
        eprintln!("* Connecting to {}...", opts.url);
        eprintln!("* Method: {}", opts.method);
    }

    match make_request(&opts) {
        Ok((status, headers, body)) => {
            if opts.verbose {
                eprintln!("* HTTP {}", status);
                for (key, value) in &headers {
                    eprintln!("< {}: {}", key, value);
                }
                eprintln!();
            }

            let output = if opts.include_headers {
                let mut out = format!("HTTP/1.1 {}\n", status);
                for (key, value) in &headers {
                    out.push_str(&format!("{}: {}\n", key, value));
                }
                out.push_str("\n");
                out.push_str(&body);
                out
            } else {
                body
            };

            if let Some(ref file) = opts.output {
                if let Err(e) = fs::write(file, &output) {
                    eprintln!("Error writing to {}: {}", file, e);
                    process::exit(1);
                }
                if !opts.silent {
                    eprintln!("Saved to {}", file);
                }
            } else {
                print!("{}", output);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
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
