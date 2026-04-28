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
