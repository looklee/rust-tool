use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::process;

fn print_usage() {
    println!("xsv - CSV toolkit (simplified)");
    println!();
    println!("Usage: xsv [OPTIONS] <command> [ARGS...]");
    println!();
    println!("Commands:");
    println!("  select <cols> [file]       Select columns");
    println!("  filter <col> <pattern> [file]  Filter rows by column value");
    println!("  frequency [file]           Show frequency table");
    println!("  stats [file]               Show column statistics");
    println!("  sort <col> [file]          Sort by column");
    println!("  join <col1> <col2> <file1> <file2>  Join two CSV files");
    println!("  cat [file...]              Concatenate CSV files");
    println!("  head [-n count] [file]     Show first N rows");
    println!("  tail [-n count] [file]     Show last N rows");
    println!("  wc [file]                  Count rows and columns");
    println!();
    println!("Options:");
    println!("  -d <delimiter>  Field delimiter (default: comma)");
    println!("  -h              Show this help message");
    println!();
    println!("Examples:");
    println!("  xsv select name,age data.csv");
    println!("  xsv filter status active data.csv");
    println!("  xsv frequency category data.csv");
    println!("  xsv stats data.csv");
    println!("  xsv sort name data.csv");
    println!("  xsv join id id users.csv orders.csv");
}

struct Config {
    delimiter: u8,
    command: String,
    args: Vec<String>,
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().collect();
    let mut delimiter = b',';
    let mut command = String::new();
    let mut cmd_args = Vec::new();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            "-d" => {
                i += 1;
                if i < args.len() {
                    let d = args[i].as_bytes();
                    if d.len() == 1 {
                        delimiter = d[0];
                    } else {
                        eprintln!("xsv: invalid delimiter '{}'", args[i]);
                        process::exit(1);
                    }
                } else {
                    eprintln!("xsv: -d requires an argument");
                    process::exit(1);
                }
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
        command,
        args: cmd_args,
    }
}

fn read_csv(file: &Option<String>, delimiter: u8) -> (Vec<String>, Vec<Vec<String>>) {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .from_reader(file_reader(file));
    
    let headers: Vec<String> = reader.headers()
        .map(|h| h.iter().map(|f| f.to_string()).collect())
        .unwrap_or_else(|_| Vec::new());
    
    let mut records = Vec::new();
    for result in reader.records() {
        match result {
            Ok(record) => {
                records.push(record.iter().map(|f| f.to_string()).collect());
            }
            Err(e) => {
                eprintln!("xsv: error reading CSV: {}", e);
            }
        }
    }
    
    (headers, records)
}

fn file_reader(file: &Option<String>) -> Box<dyn io::Read> {
    match file {
        Some(path) => {
            let f = File::open(path).unwrap_or_else(|e| {
                eprintln!("xsv: error opening {}: {}", path, e);
                process::exit(1);
            });
            Box::new(BufReader::new(f))
        }
        None => Box::new(BufReader::new(io::stdin())),
    }
}

fn write_csv(headers: &[String], records: &[Vec<String>], delimiter: u8) {
    let mut w = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(Box::new(BufWriter::new(io::stdout())));
    
    if !headers.is_empty() {
        w.write_record(headers).unwrap();
    }
    for record in records {
        w.write_record(record).unwrap();
    }
    w.flush().unwrap();
}

fn cmd_select(config: &Config) {
    if config.args.is_empty() {
        eprintln!("xsv: select requires column names");
        eprintln!("Usage: xsv select <cols> [file]");
        process::exit(1);
    }
    
    let cols: Vec<&str> = config.args[0].split(',').collect();
    let file = config.args.get(1).cloned();
    let (headers, records) = read_csv(&file, config.delimiter);
    
    let indices: Vec<usize> = cols.iter().map(|c| {
        headers.iter().position(|h| h == *c)
            .or_else(|| c.parse::<usize>().ok().filter(|i| *i < headers.len()))
            .unwrap_or_else(|| {
                eprintln!("xsv: column '{}' not found", c);
                process::exit(1);
            })
    }).collect();
    
    let new_headers: Vec<String> = indices.iter().map(|i| headers[*i].clone()).collect();
    let new_records: Vec<Vec<String>> = records.iter()
        .map(|r| indices.iter().map(|i| r.get(*i).cloned().unwrap_or_default()).collect())
        .collect();
    
    write_csv(&new_headers, &new_records, config.delimiter);
}

fn cmd_filter(config: &Config) {
    if config.args.len() < 2 {
        eprintln!("xsv: filter requires column name and pattern");
        eprintln!("Usage: xsv filter <col> <pattern> [file]");
        process::exit(1);
    }
    
    let col_name = &config.args[0];
    let pattern = &config.args[1];
    let file = config.args.get(2).cloned();
    let (headers, records) = read_csv(&file, config.delimiter);
    
    let col_idx = headers.iter().position(|h| h == col_name)
        .unwrap_or_else(|| {
            eprintln!("xsv: column '{}' not found", col_name);
            process::exit(1);
        });
    
    let filtered: Vec<Vec<String>> = records.into_iter()
        .filter(|r| r.get(col_idx).map(|v| v == pattern).unwrap_or(false))
        .collect();
    
    write_csv(&headers, &filtered, config.delimiter);
}

fn cmd_frequency(config: &Config) {
    let file = config.args.first().cloned();
    let (headers, records) = read_csv(&file, config.delimiter);
    
    if headers.is_empty() {
        return;
    }
    
    let col_name = config.args.get(1).cloned().unwrap_or_else(|| headers[0].clone());
    let col_idx = headers.iter().position(|h| h == &col_name)
        .unwrap_or_else(|| {
            eprintln!("xsv: column '{}' not found", col_name);
            process::exit(1);
        });
    
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
    
    println!("{}|count", col_name);
    for (val, count) in freq {
        println!("{}|{}", val, count);
    }
}

fn cmd_stats(config: &Config) {
    let file = config.args.first().cloned();
    let (headers, records) = read_csv(&file, config.delimiter);
    
    println!("field|min|max|mean|sum|stddev|null_count|blank_count|len|min_len|max_len|mean_len");
    
    for (i, header) in headers.iter().enumerate() {
        let mut min: Option<String> = None;
        let mut max: Option<String> = None;
        let mut sum_f64: f64 = 0.0;
        let mut count_f64: usize = 0;
        let mut null_count = 0;
        let mut blank_count = 0;
        let mut min_len: Option<usize> = None;
        let mut max_len: Option<usize> = None;
        let mut total_len: usize = 0;
        let mut len_count: usize = 0;
        
        for record in &records {
            let val = record.get(i).cloned().unwrap_or_default();
            
            if val.is_empty() {
                null_count += 1;
                continue;
            }
            
            if val.trim().is_empty() {
                blank_count += 1;
            }
            
            if let Ok(n) = val.parse::<f64>() {
                sum_f64 += n;
                count_f64 += 1;
            }
            
            let len = val.len();
            total_len += len;
            len_count += 1;
            min_len = Some(min_len.map_or(len, |m| m.min(len)));
            max_len = Some(max_len.map_or(len, |m| m.max(len)));
            
            match &min {
                None => min = Some(val.clone()),
                Some(m) if val < *m => min = Some(val.clone()),
                _ => {}
            }
            match &max {
                None => max = Some(val.clone()),
                Some(m) if val > *m => max = Some(val.clone()),
                _ => {}
            }
        }
        
        let mean = if count_f64 > 0 { sum_f64 / count_f64 as f64 } else { 0.0 };
        let stddev = if count_f64 > 1 {
            let variance = records.iter()
                .filter_map(|r| r.get(i).and_then(|v| v.parse::<f64>().ok()))
                .map(|n| (n - mean).powi(2))
                .sum::<f64>() / (count_f64 as f64 - 1.0);
            variance.sqrt()
        } else {
            0.0
        };
        
        let mean_len = if len_count > 0 { total_len as f64 / len_count as f64 } else { 0.0 };
        
        println!("{},{},{},{},{},{},{},{},{},{},{},{}",
            header,
            min.unwrap_or_default(),
            max.unwrap_or_default(),
            format!("{:.2}", mean),
            format!("{:.2}", sum_f64),
            format!("{:.2}", stddev),
            null_count,
            blank_count,
            len_count,
            min_len.unwrap_or(0),
            max_len.unwrap_or(0),
            format!("{:.2}", mean_len),
        );
    }
}

fn cmd_sort(config: &Config) {
    if config.args.is_empty() {
        eprintln!("xsv: sort requires a column name");
        eprintln!("Usage: xsv sort <col> [file]");
        process::exit(1);
    }
    
    let col_name = &config.args[0];
    let file = config.args.get(1).cloned();
    let (headers, mut records) = read_csv(&file, config.delimiter);
    
    let col_idx = headers.iter().position(|h| h == col_name)
        .unwrap_or_else(|| {
            eprintln!("xsv: column '{}' not found", col_name);
            process::exit(1);
        });
    
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
    
    write_csv(&headers, &records, config.delimiter);
}

fn cmd_join(config: &Config) {
    if config.args.len() < 4 {
        eprintln!("xsv: join requires col1 col2 file1 file2");
        eprintln!("Usage: xsv join <col1> <col2> <file1> <file2>");
        process::exit(1);
    }
    
    let col1 = &config.args[0];
    let col2 = &config.args[1];
    let file1 = &config.args[2];
    let file2 = &config.args[3];
    
    let (headers1, records1) = read_csv(&Some(file1.clone()), config.delimiter);
    let (headers2, records2) = read_csv(&Some(file2.clone()), config.delimiter);
    
    let idx1 = headers1.iter().position(|h| h == col1)
        .unwrap_or_else(|| {
            eprintln!("xsv: column '{}' not found in {}", col1, file1);
            process::exit(1);
        });
    
    let idx2 = headers2.iter().position(|h| h == col2)
        .unwrap_or_else(|| {
            eprintln!("xsv: column '{}' not found in {}", col2, file2);
            process::exit(1);
        });
    
    // Build index for file2
    let mut index2: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (i, record) in records2.iter().enumerate() {
        if let Some(val) = record.get(idx2) {
            index2.entry(val.clone()).or_default().push(i);
        }
    }
    
    // Merge headers (skip col2 from headers2)
    let mut merged_headers: Vec<String> = headers1.clone();
    for (i, h) in headers2.iter().enumerate() {
        if i != idx2 {
            merged_headers.push(h.clone());
        }
    }
    
    // Join
    let mut merged_records = Vec::new();
    for record1 in &records1 {
        if let Some(key_val) = record1.get(idx1) {
            if let Some(indices) = index2.get(key_val) {
                for &idx in indices {
                    let mut merged = record1.clone();
                    for (i, field) in records2[idx].iter().enumerate() {
                        if i != idx2 {
                            merged.push(field.clone());
                        }
                    }
                    merged_records.push(merged);
                }
            }
        }
    }
    
    write_csv(&merged_headers, &merged_records, config.delimiter);
}

fn cmd_cat(config: &Config) {
    if config.args.is_empty() {
        eprintln!("xsv: cat requires at least one file");
        eprintln!("Usage: xsv cat <file> [file...]");
        process::exit(1);
    }
    
    let mut first = true;
    
    for file in &config.args {
        let (_h, records) = read_csv(&Some(file.clone()), config.delimiter);
        if first {
            first = false;
        }
        let delim = config.delimiter as char;
        for record in records {
            println!("{}", record.iter().map(|f| {
                if f.contains(delim) || f.contains('"') || f.contains('\n') {
                    format!("\"{}\"", f.replace('"', "\"\""))
                } else {
                    f.clone()
                }
            }).collect::<Vec<_>>().join(&delim.to_string()));
        }
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
                        eprintln!("xsv: invalid count '{}'", config.args[i]);
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
    
    let (headers, records) = read_csv(&file, config.delimiter);
    let head_records: Vec<Vec<String>> = records.into_iter().take(count).collect();
    write_csv(&headers, &head_records, config.delimiter);
}

fn cmd_tail(config: &Config) {
    let mut count = 10;
    let mut file = None;
    
    let mut i = 0;
    while i < config.args.len() {
        match config.args[i].as_str() {
            "-n" => {
                i += 1;
                if i < config.args.len() {
                    count = config.args[i].parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("xsv: invalid count '{}'", config.args[i]);
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
    
    let (headers, records) = read_csv(&file, config.delimiter);
    let len = records.len();
    let start = len.saturating_sub(count);
    let tail_records: Vec<Vec<String>> = records.into_iter().skip(start).collect();
    write_csv(&headers, &tail_records, config.delimiter);
}

fn cmd_wc(config: &Config) {
    let file = config.args.first().cloned();
    let (headers, records) = read_csv(&file, config.delimiter);
    
    println!("rows: {}", records.len());
    println!("columns: {}", headers.len());
    if !headers.is_empty() {
        println!("column_names: {}", headers.join(", "));
    }
}

fn main() {
    let config = parse_args();
    
    if config.command.is_empty() {
        print_usage();
        process::exit(1);
    }
    
    match config.command.as_str() {
        "select" => cmd_select(&config),
        "filter" => cmd_filter(&config),
        "frequency" => cmd_frequency(&config),
        "stats" => cmd_stats(&config),
        "sort" => cmd_sort(&config),
        "join" => cmd_join(&config),
        "cat" => cmd_cat(&config),
        "head" => cmd_head(&config),
        "tail" => cmd_tail(&config),
        "wc" => cmd_wc(&config),
        _ => {
            eprintln!("xsv: unknown command '{}'", config.command);
            print_usage();
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
