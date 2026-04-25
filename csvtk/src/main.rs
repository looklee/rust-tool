use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::process;

fn print_usage() {
    println!("csvtk - CSV text toolkit (simplified)");
    println!();
    println!("Usage: csvtk [OPTIONS] <command> [ARGS...]");
    println!();
    println!("Commands:");
    println!("  cat <file...>              Concatenate CSV files");
    println!("  filter <col> <pattern> [file]  Filter rows");
    println!("  filter2 <expression> [file]    Filter with expression");
    println!("  freq [col] [file]            Frequency table");
    println!("  head [-n count] [file]       Show first N rows");
    println!("  stat [file]                  Show statistics");
    println!("  sort <col> [file]            Sort by column");
    println!("  uniq [col] [file]            Remove duplicates");
    println!("  split <n> [file]             Split into N parts");
    println!("  summary [file]               Column summary");
    println!();
    println!("Options:");
    println!("  -H    No header row");
    println!("  -t    Tab-separated input/output");
    println!("  -h    Show this help message");
    println!();
    println!("Examples:");
    println!("  csvtk cat data1.csv data2.csv");
    println!("  csvtk filter name Alice data.csv");
    println!("  csvtk freq category data.csv");
    println!("  csvtk head -n 5 data.csv");
    println!("  csvtk stat data.csv");
    println!("  csvtk sort age data.csv");
}

struct Config {
    delimiter: u8,
    has_header: bool,
    command: String,
    args: Vec<String>,
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().collect();
    let mut delimiter = b',';
    let mut has_header = true;
    let mut command = String::new();
    let mut cmd_args = Vec::new();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            "-H" => {
                has_header = false;
            }
            "-t" => {
                delimiter = b'\t';
            }
            _ => {
                if command.is_empty() {
                    command = args[i].clone();
                } else {
                    cmd_args.push(args[i].clone());
                }
            }
        }
        i += 1;
    }
    
    Config {
        delimiter,
        has_header,
        command,
        args: cmd_args,
    }
}

fn read_csv(config: &Config, file: &Option<String>) -> (Vec<String>, Vec<Vec<String>>) {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(config.delimiter)
        .has_headers(config.has_header)
        .from_reader(file_reader(file));
    
    let headers: Vec<String> = if config.has_header {
        reader.headers()
            .map(|h| h.iter().map(|f| f.to_string()).collect())
            .unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };
    
    let mut records = Vec::new();
    for result in reader.records() {
        match result {
            Ok(record) => {
                records.push(record.iter().map(|f| f.to_string()).collect());
            }
            Err(e) => {
                eprintln!("csvtk: error reading CSV: {}", e);
            }
        }
    }
    
    (headers, records)
}

fn file_reader(file: &Option<String>) -> Box<dyn io::Read> {
    match file {
        Some(path) => {
            let f = File::open(path).unwrap_or_else(|e| {
                eprintln!("csvtk: error opening {}: {}", path, e);
                process::exit(1);
            });
            Box::new(BufReader::new(f))
        }
        None => Box::new(BufReader::new(io::stdin())),
    }
}

fn write_csv(headers: &[String], records: &[Vec<String>], delimiter: u8, has_header: bool) {
    let mut w = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .has_headers(has_header)
        .from_writer(Box::new(BufWriter::new(io::stdout())));
    
    if has_header && !headers.is_empty() {
        w.write_record(headers).unwrap();
    }
    for record in records {
        w.write_record(record).unwrap();
    }
    w.flush().unwrap();
}

fn cmd_cat(config: &Config) {
    if config.args.is_empty() {
        eprintln!("csvtk: cat requires at least one file");
        eprintln!("Usage: csvtk cat <file> [file...]");
        process::exit(1);
    }
    
    let mut first = true;
    let mut headers: Vec<String> = Vec::new();
    let mut all_records = Vec::new();
    
    for file in &config.args {
        let (h, records) = read_csv(config, &Some(file.clone()));
        if first {
            headers = h;
            first = false;
        }
        all_records.extend(records);
    }
    
    write_csv(&headers, &all_records, config.delimiter, config.has_header);
}

fn cmd_filter(config: &Config) {
    if config.args.len() < 2 {
        eprintln!("csvtk: filter requires column name and pattern");
        eprintln!("Usage: csvtk filter <col> <pattern> [file]");
        process::exit(1);
    }
    
    let col_name = &config.args[0];
    let pattern = &config.args[1];
    let file = config.args.get(2).cloned();
    let (headers, records) = read_csv(config, &file);
    
    let col_idx = if headers.is_empty() {
        col_name.parse::<usize>().unwrap_or_else(|_| {
            eprintln!("csvtk: column '{}' not found", col_name);
            process::exit(1);
        })
    } else {
        headers.iter().position(|h| h == col_name)
            .unwrap_or_else(|| {
                eprintln!("csvtk: column '{}' not found", col_name);
                process::exit(1);
            })
    };
    
    let filtered: Vec<Vec<String>> = records.into_iter()
        .filter(|r| r.get(col_idx).map(|v| v.contains(pattern)).unwrap_or(false))
        .collect();
    
    write_csv(&headers, &filtered, config.delimiter, config.has_header);
}

fn cmd_filter2(config: &Config) {
    if config.args.is_empty() {
        eprintln!("csvtk: filter2 requires an expression");
        eprintln!("Usage: csvtk filter2 <expression> [file]");
        process::exit(1);
    }
    
    let expression = &config.args[0];
    let file = config.args.get(1).cloned();
    let (headers, records) = read_csv(config, &file);
    
    // Parse simple expression: col op value
    let filtered: Vec<Vec<String>> = records.into_iter()
        .filter(|record| {
            evaluate_expression(record, &headers, expression)
        })
        .collect();
    
    write_csv(&headers, &filtered, config.delimiter, config.has_header);
}

fn evaluate_expression(record: &[String], headers: &[String], expression: &str) -> bool {
    let expr = expression.trim();
    
    // Try to parse as: column_name operator value
    for op in &["==", "!=", ">=", "<=", ">", "<", "=~"] {
        if let Some(pos) = expr.find(op) {
            let col = expr[..pos].trim();
            let val = expr[pos + op.len()..].trim();
            
            let col_idx = match headers.iter().position(|h| h == col) {
                Some(i) => i,
                None => return false,
            };
            
            let cell = record.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            
            match *op {
                "==" => return cell == val,
                "!=" => return cell != val,
                "=~" => return cell.contains(val),
                _ => {
                    // Try numeric comparison
                    if let (Ok(n1), Ok(n2)) = (cell.parse::<f64>(), val.parse::<f64>()) {
                        match *op {
                            ">" => return n1 > n2,
                            "<" => return n1 < n2,
                            ">=" => return n1 >= n2,
                            "<=" => return n1 <= n2,
                            _ => {}
                        }
                    } else {
                        // String comparison
                        match *op {
                            ">" => return cell > val,
                            "<" => return cell < val,
                            ">=" => return cell >= val,
                            "<=" => return cell <= val,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    
    true
}

fn cmd_freq(config: &Config) {
    let file = if config.args.len() > 1 {
        config.args.last().cloned()
    } else {
        config.args.first().cloned()
    };
    
    let col_name = config.args.first().cloned();
    let (headers, records) = read_csv(config, &file);
    
    let col_idx = if headers.is_empty() {
        0
    } else if let Some(name) = col_name {
        headers.iter().position(|h| h == &name)
            .unwrap_or_else(|| {
                eprintln!("csvtk: column '{}' not found", name);
                process::exit(1);
            })
    } else {
        0
    };
    
    let col_label = headers.get(col_idx).cloned().unwrap_or_else(|| format!("col_{}", col_idx + 1));
    
    let mut freq: Vec<(String, usize)> = Vec::new();
    for record in &records {
        if let Some(val) = record.get(col_idx) {
            match freq.iter_mut().find(|(v, _)| v == val) {
                Some((_, count)) => *count += 1,
                None => freq.push((val.clone(), 1)),
            }
        }
    }
    
    freq.sort_by(|a, b| b.1.cmp(&a.1));
    
    println!("{}|count|percent", col_label);
    let total = freq.iter().map(|(_, c)| c).sum::<usize>();
    for (val, count) in freq {
        let percent = if total > 0 { (count as f64 / total as f64) * 100.0 } else { 0.0 };
        println!("{}|{}|{:.2}%", val, count, percent);
    }
}

fn cmd_head(config: &Config) {
    let mut count = 10;
    let mut file = None;
    
    let mut i = 0;
    while i < config.args.len() {
        match config.args[i].as_str() {
            "-n" => {
                i += 1;
                if i < config.args.len() {
                    count = config.args[i].parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("csvtk: invalid count '{}'", config.args[i]);
                        process::exit(1);
                    });
                }
            }
            _ => {
                file = Some(config.args[i].clone());
            }
        }
        i += 1;
    }
    
    let (headers, records) = read_csv(config, &file);
    let head_records: Vec<Vec<String>> = records.into_iter().take(count).collect();
    write_csv(&headers, &head_records, config.delimiter, config.has_header);
}

fn cmd_stat(config: &Config) {
    let file = config.args.first().cloned();
    let (headers, records) = read_csv(config, &file);
    
    println!("column|type|null|blank|unique|min|max|max_len");
    
    let col_count = if headers.is_empty() {
        records.first().map_or(0, |r| r.len())
    } else {
        headers.len()
    };
    
    for idx in 0..col_count {
        let col_name = headers.get(idx).cloned().unwrap_or_else(|| format!("col_{}", idx + 1));
        let mut null_count = 0;
        let mut blank_count = 0;
        let mut unique: Vec<String> = Vec::new();
        let mut min_val: Option<String> = None;
        let mut max_val: Option<String> = None;
        let mut max_len = 0;
        let mut is_numeric = true;
        
        for record in &records {
            let val = record.get(idx).cloned().unwrap_or_default();
            
            if val.is_empty() {
                null_count += 1;
                continue;
            }
            
            if val.trim().is_empty() {
                blank_count += 1;
            }
            
            if !unique.contains(&val) {
                unique.push(val.clone());
            }
            
            max_len = max_len.max(val.len());
            
            if val.parse::<f64>().is_err() {
                is_numeric = false;
            }
            
            match &min_val {
                None => min_val = Some(val.clone()),
                Some(m) if val < *m => min_val = Some(val.clone()),
                _ => {}
            }
            match &max_val {
                None => max_val = Some(val.clone()),
                Some(m) if val > *m => max_val = Some(val.clone()),
                _ => {}
            }
        }
        
        let col_type = if is_numeric && null_count < records.len() / 2 { "numeric" } else { "string" };
        
        println!("{},{},{},{},{},{},{},{}",
            col_name,
            col_type,
            null_count,
            blank_count,
            unique.len(),
            min_val.unwrap_or_default(),
            max_val.unwrap_or_default(),
            max_len,
        );
    }
}

fn cmd_sort(config: &Config) {
    if config.args.is_empty() {
        eprintln!("csvtk: sort requires a column name");
        eprintln!("Usage: csvtk sort <col> [file]");
        process::exit(1);
    }
    
    let col_name = &config.args[0];
    let file = config.args.get(1).cloned();
    let (headers, mut records) = read_csv(config, &file);
    
    let col_idx = if headers.is_empty() {
        col_name.parse::<usize>().unwrap_or_else(|_| {
            eprintln!("csvtk: column '{}' not found", col_name);
            process::exit(1);
        })
    } else {
        headers.iter().position(|h| h == col_name)
            .unwrap_or_else(|| {
                eprintln!("csvtk: column '{}' not found", col_name);
                process::exit(1);
            })
    };
    
    records.sort_by(|a, b| {
        let empty = String::new();
        let va = a.get(col_idx).unwrap_or(&empty);
        let vb = b.get(col_idx).unwrap_or(&empty);
        // Try numeric comparison first
        match (va.parse::<f64>(), vb.parse::<f64>()) {
            (Ok(na), Ok(nb)) => na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal),
            _ => va.cmp(vb),
        }
    });
    
    write_csv(&headers, &records, config.delimiter, config.has_header);
}

fn cmd_uniq(config: &Config) {
    let file = if config.args.len() > 1 {
        config.args.last().cloned()
    } else {
        config.args.first().cloned()
    };
    
    let col_name = config.args.first().cloned();
    let (headers, records) = read_csv(config, &file);
    
    let col_idx = if headers.is_empty() {
        0
    } else if let Some(name) = col_name {
        headers.iter().position(|h| h == &name)
            .unwrap_or_else(|| {
                eprintln!("csvtk: column '{}' not found", name);
                process::exit(1);
            })
    } else {
        0
    };
    
    let mut seen = Vec::new();
    let unique: Vec<Vec<String>> = records.into_iter()
        .filter(|record| {
            let val = record.get(col_idx).cloned().unwrap_or_default();
            if seen.contains(&val) {
                false
            } else {
                seen.push(val);
                true
            }
        })
        .collect();
    
    write_csv(&headers, &unique, config.delimiter, config.has_header);
}

fn cmd_split(config: &Config) {
    if config.args.is_empty() {
        eprintln!("csvtk: split requires a count");
        eprintln!("Usage: csvtk split <n> [file]");
        process::exit(1);
    }
    
    let n = config.args[0].parse::<usize>().unwrap_or_else(|_| {
        eprintln!("csvtk: invalid count '{}'", config.args[0]);
        process::exit(1);
    });
    
    let file = config.args.get(1).cloned();
    let (headers, records) = read_csv(config, &file);
    
    let chunk_size = (records.len() + n - 1) / n;
    
    for (i, chunk) in records.chunks(chunk_size).enumerate() {
        println!("=== Part {} ===", i + 1);
        write_csv(&headers, chunk, config.delimiter, config.has_header);
        println!();
    }
}

fn cmd_summary(config: &Config) {
    let file = config.args.first().cloned();
    let (headers, records) = read_csv(config, &file);
    
    println!("total_rows: {}", records.len());
    println!("total_columns: {}", headers.len());
    
    if !headers.is_empty() {
        println!();
        println!("columns:");
        for (i, header) in headers.iter().enumerate() {
            let mut non_empty = 0;
            let mut unique = std::collections::HashSet::new();
            
            for record in &records {
                if let Some(val) = record.get(i) {
                    if !val.is_empty() {
                        non_empty += 1;
                        unique.insert(val.clone());
                    }
                }
            }
            
            println!("  {}: {} non-empty, {} unique", header, non_empty, unique.len());
        }
    }
}

fn main() {
    let config = parse_args();
    
    if config.command.is_empty() {
        print_usage();
        process::exit(1);
    }
    
    match config.command.as_str() {
        "cat" => cmd_cat(&config),
        "filter" => cmd_filter(&config),
        "filter2" => cmd_filter2(&config),
        "freq" => cmd_freq(&config),
        "head" => cmd_head(&config),
        "stat" => cmd_stat(&config),
        "sort" => cmd_sort(&config),
        "uniq" => cmd_uniq(&config),
        "split" => cmd_split(&config),
        "summary" => cmd_summary(&config),
        _ => {
            eprintln!("csvtk: unknown command '{}'", config.command);
            print_usage();
            process::exit(1);
        }
    }
}
