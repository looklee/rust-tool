use std::env;

use super::errors::{ToolError, ToolResult};

pub struct CliOptions {
    pub args: Vec<String>,
    pub flags: Vec<String>,
    pub options: Vec<(String, String)>,
    pub positional: Vec<String>,
}

impl CliOptions {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        Self::from_args(&args[1..])
    }
    
    pub fn from_args(args: &[String]) -> Self {
        let mut flags = Vec::new();
        let mut options = Vec::new();
        let mut positional = Vec::new();
        
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            
            if arg.starts_with("--") {
                if let Some((key, value)) = arg.split_once('=') {
                    options.push((key[2..].to_string(), value.to_string()));
                } else {
                    flags.push(arg[2..].to_string());
                }
            } else if arg.starts_with('-') && arg.len() > 1 {
                if arg.len() == 2 {
                    flags.push(arg[1..].to_string());
                } else {
                    for c in arg[1..].chars() {
                        flags.push(c.to_string());
                    }
                }
            } else {
                positional.push(arg.clone());
            }
            
            i += 1;
        }
        
        Self {
            args: args.to_vec(),
            flags,
            options,
            positional,
        }
    }
    
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(&flag.to_string()) || 
        self.flags.contains(&flag.to_lowercase())
    }
    
    pub fn get_option(&self, name: &str) -> Option<&str> {
        self.options.iter()
            .find(|(k, _)| k == name || k.to_lowercase() == name.to_lowercase())
            .map(|(_, v)| v.as_str())
    }
    
    pub fn parse_flag(args: &[String], flag: &str) -> bool {
        args.iter().any(|a| a == flag)
    }
    
    pub fn parse_opt_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        for i in 0..args.len() {
            if args[i] == flag && i + 1 < args.len() {
                return Some(&args[i + 1]);
            }
        }
        None
    }
    
    pub fn parse_opt_value_with_equals(args: &[String], flag: &str) -> Option<&str> {
        for arg in args {
            if let Some(val) = arg.strip_prefix(&format!("{}=", flag)) {
                return Some(val);
            }
        }
        None
    }
    
    pub fn extract_files_from_args(args: &[String]) -> Vec<String> {
        args.iter()
            .filter(|a| !a.starts_with('-'))
            .cloned()
            .collect()
    }
    
    pub fn extract_positional_args_after(args: &[String], marker: &str) -> Vec<String> {
        let mut found_marker = false;
        let mut result = Vec::new();
        
        for arg in args {
            if found_marker {
                if !arg.starts_with('-') {
                    result.push(arg.clone());
                }
            } else if arg == marker {
                found_marker = true;
            }
        }
        
        result
    }
    
    pub fn parse_bool(&self, name: &str, default: bool) -> bool {
        match self.get_option(name) {
            Some("true") | Some("1") | Some("yes") => true,
            Some("false") | Some("0") | Some("no") => false,
            None => default,
            _ => default,
        }
    }
    
    pub fn parse_int(&self, name: &str, default: i64) -> ToolResult<i64> {
        match self.get_option(name) {
            Some(v) => v.parse().map_err(|e| ToolError::Parse(format!("Invalid integer: {}", e))),
            None => Ok(default),
        }
    }
    
    pub fn parse_float(&self, name: &str, default: f64) -> ToolResult<f64> {
        match self.get_option(name) {
            Some(v) => v.parse().map_err(|e| ToolError::Parse(format!("Invalid float: {}", e))),
            None => Ok(default),
        }
    }
}