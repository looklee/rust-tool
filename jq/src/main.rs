use serde_json::{Value, Number};
use std::io::{self, Read};
use std::env;

fn print_usage() {
    println!("jq - JSON processor (simplified)");
    println!();
    println!("Usage: jq [OPTIONS] FILTER [FILE...]");
    println!();
    println!("Options:");
    println!("  -r    Raw output (strings without quotes)");
    println!("  -c    Compact output (no whitespace)");
    println!("  -h    Show this help message");
    println!();
    println!("Filters:");
    println!("  .           Identity");
    println!("  .field      Access field by name");
    println!("  .[index]    Access array element by index");
    println!("  .[]         Iterate over array/object values");
    println!("  .[start:end] Slice array");
    println!("  keys        Get object keys as array");
    println!("  values      Get object values as array");
    println!("  length      Get length of array/string/object");
    println!("  type        Get type name");
    println!("  sort        Sort array");
    println!("  unique      Get unique elements");
    println!("  map(expr)   Apply expression to each element");
    println!("  select(expr) Select elements matching condition");
    println!("  .[field]    Access field with brackets");
    println!();
    println!("Examples:");
    println!("  echo '{{\"name\":\"Alice\",\"age\":30}}' | jq .name");
    println!("  echo '[1,2,3]' | jq '.[]'");
    println!("  cat data.json | jq '.users[] | .name'");
}

fn format_value(val: &Value, raw: bool, compact: bool) -> String {
    match val {
        Value::String(s) if raw => s.clone(),
        Value::Null if raw => String::new(),
        _ => {
            if compact {
                val.to_string()
            } else {
                serde_json::to_string_pretty(val).unwrap_or_else(|_| val.to_string())
            }
        }
    }
}

fn apply_filter(input: &Value, filter: &str) -> Vec<Value> {
    let mut results = Vec::new();
    apply_filter_recursive(input, filter, &mut results);
    results
}

fn apply_filter_recursive(input: &Value, filter: &str, results: &mut Vec<Value>) {
    let filter = filter.trim();
    
    if filter.is_empty() || filter == "." {
        results.push(input.clone());
        return;
    }

    // Handle pipe operator
    if let Some(pipe_pos) = find_unquoted_char(filter, '|') {
        let left = &filter[..pipe_pos];
        let right = &filter[pipe_pos + 1..];
        let left_results = apply_filter(input, left.trim());
        for val in left_results {
            apply_filter_recursive(&val, right.trim(), results);
        }
        return;
    }

    // Handle comma (multiple filters)
    if let Some(comma_pos) = find_unquoted_char(filter, ',') {
        let left = &filter[..comma_pos];
        let right = &filter[comma_pos + 1..];
        apply_filter_recursive(input, left.trim(), results);
        apply_filter_recursive(input, right.trim(), results);
        return;
    }

    // Handle select(expr)
    if filter.starts_with("select(") && filter.ends_with(')') {
        let inner = &filter[7..filter.len() - 1];
        if matches_select(input, inner) {
            results.push(input.clone());
        }
        return;
    }

    // Handle map(expr)
    if filter.starts_with("map(") && filter.ends_with(')') {
        let inner = &filter[4..filter.len() - 1];
        match input {
            Value::Array(arr) => {
                for item in arr {
                    let mapped = apply_filter(item, inner.trim());
                    if mapped.len() == 1 {
                        results.push(mapped[0].clone());
                    } else if !mapped.is_empty() {
                        results.push(Value::Array(mapped));
                    }
                }
            }
            _ => {}
        }
        return;
    }

    // Handle keys
    if filter == "keys" {
        match input {
            Value::Object(map) => {
                let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
                results.push(Value::Array(keys));
            }
            Value::Array(arr) => {
                let keys: Vec<Value> = (0..arr.len() as i64).map(|i| Value::Number(Number::from(i))).collect();
                results.push(Value::Array(keys));
            }
            _ => {}
        }
        return;
    }

    // Handle values
    if filter == "values" {
        match input {
            Value::Object(map) => {
                let vals: Vec<Value> = map.values().map(|v| v.clone()).collect();
                results.push(Value::Array(vals));
            }
            Value::Array(arr) => {
                results.push(Value::Array(arr.clone()));
            }
            _ => {}
        }
        return;
    }

    // Handle length
    if filter == "length" {
        match input {
            Value::Array(arr) => results.push(Value::Number(Number::from(arr.len() as i64))),
            Value::Object(map) => results.push(Value::Number(Number::from(map.len() as i64))),
            Value::String(s) => results.push(Value::Number(Number::from(s.len() as i64))),
            _ => {}
        }
        return;
    }

    // Handle type
    if filter == "type" {
        let type_name = match input {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };
        results.push(Value::String(type_name.to_string()));
        return;
    }

    // Handle sort
    if filter == "sort" {
        if let Value::Array(arr) = input {
            let mut sorted = arr.clone();
            sorted.sort_by(|a, b| {
                partial_cmp_values(a, b)
            });
            results.push(Value::Array(sorted));
        }
        return;
    }

    // Handle unique
    if filter == "unique" {
        if let Value::Array(arr) = input {
            let mut seen = Vec::new();
            let mut unique = Vec::new();
            for val in arr {
                if !seen.contains(val) {
                    seen.push(val.clone());
                    unique.push(val.clone());
                }
            }
            results.push(Value::Array(unique));
        }
        return;
    }

    // Handle .[] - iterate
    if filter == ".[]" {
        match input {
            Value::Array(arr) => {
                for item in arr {
                    results.push(item.clone());
                }
            }
            Value::Object(map) => {
                for val in map.values() {
                    results.push(val.clone());
                }
            }
            _ => {}
        }
        return;
    }

    // Handle .[start:end] - slice
    if let Some(slice_result) = parse_slice(filter) {
        if let Value::Array(arr) = input {
            let start = slice_result.0.unwrap_or(0);
            let end = slice_result.1.unwrap_or(arr.len() as i64);
            let start = clamp_index(start, arr.len() as i64);
            let end = clamp_index(end, arr.len() as i64);
            if start < end && start < arr.len() as i64 {
                let end_idx = (end as usize).min(arr.len());
                let slice: Vec<Value> = arr[start as usize..end_idx].to_vec();
                results.push(Value::Array(slice));
            } else {
                results.push(Value::Array(vec![]));
            }
        }
        return;
    }

    // Handle .[index] - array index or .[field] - bracket notation
    if filter.starts_with(".[") && filter.ends_with(']') {
        let inner = &filter[2..filter.len() - 1];
        let inner = inner.trim();
        
        // Try numeric index
        if let Ok(idx) = inner.parse::<i64>() {
            if let Value::Array(arr) = input {
                let idx = if idx < 0 { (arr.len() as i64 + idx) % arr.len() as i64 } else { idx };
                if idx >= 0 && idx < arr.len() as i64 {
                    results.push(arr[idx as usize].clone());
                }
            }
            return;
        }
        
        // Try quoted string for field access
        let field = strip_quotes(inner);
        if let Value::Object(map) = input {
            if let Some(val) = map.get(&field) {
                results.push(val.clone());
            }
        }
        return;
    }

    // Handle .field - field access
    if filter.starts_with('.') {
        let rest = &filter[1..];
        
        // Check for field followed by [] iteration (e.g., .users[])
        if let Some(bracket_pos) = rest.find("[]") {
            let field = &rest[..bracket_pos];
            let remaining = &rest[bracket_pos + 2..];
            if let Value::Object(map) = input {
                if let Some(val) = map.get(field) {
                    // Apply [] iteration to the field value
                    match val {
                        Value::Array(arr) => {
                            for item in arr {
                                if remaining.is_empty() {
                                    results.push(item.clone());
                                } else {
                                    apply_filter_recursive(item, remaining, results);
                                }
                            }
                        }
                        Value::Object(map2) => {
                            for v in map2.values() {
                                if remaining.is_empty() {
                                    results.push(v.clone());
                                } else {
                                    apply_filter_recursive(v, remaining, results);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            return;
        }
        
        // Check for nested access like .foo.bar
        if let Some(dot_pos) = find_next_field(rest) {
            let field = &rest[..dot_pos];
            let remaining = &rest[dot_pos..];
            if let Value::Object(map) = input {
                if let Some(val) = map.get(field) {
                    apply_filter_recursive(val, remaining, results);
                }
            }
            return;
        }
        
        let field = rest;
        if field.is_empty() {
            results.push(input.clone());
            return;
        }
        
        if let Value::Object(map) = input {
            if let Some(val) = map.get(field) {
                results.push(val.clone());
            }
        }
        return;
    }

    // Handle literal values
    if filter == "null" {
        results.push(Value::Null);
        return;
    }
    if filter == "true" {
        results.push(Value::Bool(true));
        return;
    }
    if filter == "false" {
        results.push(Value::Bool(false));
        return;
    }
    
    // Try parsing as number
    if let Ok(n) = filter.parse::<i64>() {
        results.push(Value::Number(Number::from(n)));
        return;
    }
    if let Ok(f) = filter.parse::<f64>() {
        if let Some(n) = Number::from_f64(f) {
            results.push(Value::Number(n));
        }
        return;
    }

    // Try parsing as string literal
    if (filter.starts_with('"') && filter.ends_with('"')) ||
       (filter.starts_with('\'') && filter.ends_with('\'')) {
        let s = &filter[1..filter.len() - 1];
        results.push(Value::String(s.to_string()));
        return;
    }

    // If nothing matched, return input
    results.push(input.clone());
}

fn find_unquoted_char(s: &str, ch: char) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        if c == '"' || c == '\'' {
            in_string = !in_string;
            continue;
        }
        if !in_string {
            if c == '(' || c == '[' {
                depth += 1;
            } else if c == ')' || c == ']' {
                depth -= 1;
            } else if c == ch && depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn find_next_field(s: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escape = false;
    let mut bracket_depth = 0;
    
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        if c == '"' || c == '\'' {
            in_string = !in_string;
            continue;
        }
        if !in_string {
            if c == '[' {
                bracket_depth += 1;
            } else if c == ']' {
                bracket_depth -= 1;
            } else if c == '.' && bracket_depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn parse_slice(filter: &str) -> Option<(Option<i64>, Option<i64>)> {
    if !filter.starts_with(".[") || !filter.ends_with(']') {
        return None;
    }
    let inner = &filter[2..filter.len() - 1];
    if let Some(colon_pos) = inner.find(':') {
        let start = if colon_pos > 0 {
            inner[..colon_pos].trim().parse::<i64>().ok()
        } else {
            None
        };
        let end = if colon_pos + 1 < inner.len() {
            inner[colon_pos + 1..].trim().parse::<i64>().ok()
        } else {
            None
        };
        Some((start, end))
    } else {
        None
    }
}

fn clamp_index(idx: i64, len: i64) -> i64 {
    if idx < 0 {
        (len + idx).max(0)
    } else {
        idx.min(len)
    }
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) ||
       (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn matches_select(input: &Value, condition: &str) -> bool {
    let condition = condition.trim();
    
    // Handle comparison operators
    for op in &["==", "!=", ">=", "<=", ">", "<"] {
        if let Some(pos) = condition.find(op) {
            let left = condition[..pos].trim();
            let right = condition[pos + op.len()..].trim();
            let left_val = resolve_value(input, left);
            let right_val = resolve_value(input, right);
            return compare_values(&left_val, op, &right_val);
        }
    }
    
    // Handle boolean check
    if condition == "true" || condition == "." {
        return !matches!(input, Value::Null | Value::Bool(false));
    }
    if condition == "false" {
        return matches!(input, Value::Null | Value::Bool(false));
    }
    
    // Handle field existence
    if condition.starts_with('.') {
        let results = apply_filter(input, condition);
        return !results.is_empty() && !matches!(&results[0], Value::Null);
    }
    
    false
}

fn resolve_value(input: &Value, expr: &str) -> Value {
    let expr = expr.trim();
    
    // Try as literal
    if expr == "null" { return Value::Null; }
    if expr == "true" { return Value::Bool(true); }
    if expr == "false" { return Value::Bool(false); }
    if let Ok(n) = expr.parse::<i64>() { return Value::Number(Number::from(n)); }
    if let Ok(f) = expr.parse::<f64>() {
        if let Some(n) = Number::from_f64(f) { return Value::Number(n); }
    }
    if (expr.starts_with('"') && expr.ends_with('"')) ||
       (expr.starts_with('\'') && expr.ends_with('\'')) {
        return Value::String(expr[1..expr.len()-1].to_string());
    }
    
    // Try as field access
    if expr.starts_with('.') {
        let results = apply_filter(input, expr);
        if !results.is_empty() {
            return results[0].clone();
        }
    }
    
    Value::Null
}

fn compare_values(left: &Value, op: &str, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => {
            let a_f = a.as_f64().unwrap_or(0.0);
            let b_f = b.as_f64().unwrap_or(0.0);
            match op {
                "==" => a_f == b_f,
                "!=" => a_f != b_f,
                ">" => a_f > b_f,
                "<" => a_f < b_f,
                ">=" => a_f >= b_f,
                "<=" => a_f <= b_f,
                _ => false,
            }
        }
        (Value::String(a), Value::String(b)) => {
            match op {
                "==" => a == b,
                "!=" => a != b,
                ">" => a > b,
                "<" => a < b,
                ">=" => a >= b,
                "<=" => a <= b,
                _ => false,
            }
        }
        (Value::Bool(a), Value::Bool(b)) => {
            match op {
                "==" => a == b,
                "!=" => a != b,
                _ => false,
            }
        }
        (Value::Null, Value::Null) => op == "==" || op == "<=",
        _ => {
            if op == "==" { left == right }
            else if op == "!=" { left != right }
            else { false }
        }
    }
}

fn partial_cmp_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let xf = x.as_f64().unwrap_or(0.0);
            let yf = y.as_f64().unwrap_or(0.0);
            xf.partial_cmp(&yf).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
        _ => std::cmp::Ordering::Equal,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }
    
    let mut raw = false;
    let mut compact = false;
    let mut filter = String::new();
    let mut files = Vec::new();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                return;
            }
            "-r" => raw = true,
            "-c" => compact = true,
            _ => {
                if filter.is_empty() {
                    filter = args[i].clone();
                } else {
                    files.push(args[i].clone());
                }
            }
        }
        i += 1;
    }
    
    if filter.is_empty() {
        eprintln!("jq: no filter provided");
        print_usage();
        std::process::exit(1);
    }
    
    // Read input
    let mut input_str = String::new();
    if files.is_empty() {
        io::stdin().read_to_string(&mut input_str).unwrap_or_else(|e| {
            eprintln!("jq: error reading stdin: {}", e);
            std::process::exit(1);
        });
    } else {
        for file in &files {
            let mut file_input = String::new();
            std::fs::File::open(file).unwrap_or_else(|e| {
                eprintln!("jq: error opening {}: {}", file, e);
                std::process::exit(1);
            }).read_to_string(&mut file_input).unwrap_or_else(|e| {
                eprintln!("jq: error reading {}: {}", file, e);
                std::process::exit(1);
            });
            input_str.push_str(&file_input);
        }
    }
    
    // Parse JSON
    let json: Value = serde_json::from_str(&input_str).unwrap_or_else(|e| {
        eprintln!("jq: error parsing JSON: {}", e);
        std::process::exit(1);
    });
    
    // Apply filter
    let results = apply_filter(&json, &filter);
    
    // Output results
    for (idx, val) in results.iter().enumerate() {
        let output = format_value(val, raw, compact);
        print!("{}", output);
        if idx < results.len() - 1 {
            println!();
        } else if !output.ends_with('\n') {
            println!();
        }
    }
}
