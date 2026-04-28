use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

use common::{Colors, is_terminal};

struct DiffConfig {
    ignore_blank: bool,
    ignore_case: bool,
    brief: bool,
    color: bool,
    format: OutputFormat,
    context_lines: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum OutputFormat {
    Normal,
    Unified,
    Context,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            ignore_blank: false,
            ignore_case: false,
            brief: false,
            color: false,
            format: OutputFormat::Normal,
            context_lines: 3,
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let (config, files) = parse_args(&args[1..])?;

    if files.len() != 2 {
        eprintln!("Usage: diff [OPTION]... FILE1 FILE2");
        eprintln!("Try 'diff --help' for more information.");
        std::process::exit(1);
    }

    let file1 = &files[0];
    let file2 = &files[1];

    diff_files(file1, file2, &config)?;

    Ok(())
}

fn parse_args(args: &[String]) -> io::Result<(DiffConfig, Vec<String>)> {
    let mut config = DiffConfig::default();
    let mut files = Vec::new();

    for arg in args {
        if arg == "--help" || arg == "-h" {
            print_help();
            std::process::exit(0);
        } else if arg == "-w" || arg == "--ignore-all-space" {
            config.ignore_blank = true;
        } else if arg == "-i" || arg == "--ignore-case" {
            config.ignore_case = true;
        } else if arg == "-q" || arg == "--brief" {
            config.brief = true;
        } else if arg == "--color" {
            config.color = true;
        } else if arg == "--no-color" {
            config.color = false;
        } else if arg == "-u" || arg == "--unified" {
            config.format = OutputFormat::Unified;
        } else if arg == "-c" || arg == "--context" {
            config.format = OutputFormat::Context;
        } else if arg == "-U" || arg.starts_with("-U") {
            config.format = OutputFormat::Unified;
            let num_str = if arg.len() > 2 {
                &arg[2..]
            } else if arg == "-U" && files.len() < 2 {
                continue;
            } else {
                "3"
            };
            if let Ok(n) = num_str.parse() {
                config.context_lines = n;
            }
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            for flag in arg[1..].chars() {
                match flag {
                    'w' => config.ignore_blank = true,
                    'i' => config.ignore_case = true,
                    'q' => config.brief = true,
                    'u' => config.format = OutputFormat::Unified,
                    'c' => config.format = OutputFormat::Context,
                    'h' => {
                        print_help();
                        std::process::exit(0);
                    }
                    _ => {
                        eprintln!("diff: invalid option -- '{}'", flag);
                        eprintln!("Try 'diff --help' for more information.");
                        std::process::exit(1);
                    }
                }
            }
        } else if !arg.starts_with('-') {
            files.push(arg.clone());
        }
    }

    if !config.color {
        config.color = is_terminal();
    }

    Ok((config, files))
}

fn print_help() {
    println!("diff - compare files line by line");
    println!();
    println!("Usage: diff [OPTION]... FILE1 FILE2");
    println!();
    println!("Options:");
    println!("  -w, --ignore-all-space   ignore all white space");
    println!("  -i, --ignore-case        ignore case differences");
    println!("  -q, --brief              report only whether files differ");
    println!("  -u, -U, --unified        show unified diff with context");
    println!("  -c, --context            show context diff");
    println!("  --color                  colorize the output");
    println!("  --no-color               disable color output");
    println!("  -h, --help               display this help and exit");
}

fn diff_files(file1: &str, file2: &str, config: &DiffConfig) -> io::Result<()> {
    let path1 = Path::new(file1);
    let path2 = Path::new(file2);

    if !path1.exists() {
        eprintln!("diff: {}: No such file or directory", file1);
        std::process::exit(1);
    }
    if !path2.exists() {
        eprintln!("diff: {}: No such file or directory", file2);
        std::process::exit(1);
    }

    let f1 = File::open(path1)?;
    let f2 = File::open(path2)?;

    let lines1: Vec<String> = BufReader::new(f1)
        .lines()
        .collect::<Result<Vec<_>, _>>()?;
    let lines2: Vec<String> = BufReader::new(f2)
        .lines()
        .collect::<Result<Vec<_>, _>>()?;

    let process_line = |line: &str| -> String {
        let mut result = line.to_string();
        if config.ignore_blank {
            result = result.split_whitespace().collect::<Vec<_>>().join(" ");
        }
        if config.ignore_case {
            result = result.to_lowercase();
        }
        result
    };

    let processed1: Vec<String> = lines1.iter().map(|l| process_line(l)).collect();
    let processed2: Vec<String> = lines2.iter().map(|l| process_line(l)).collect();

    let lcs = longest_common_subsequence(&processed1, &processed2);

    if config.brief {
        if lines1.len() == lines2.len() && processed1 == processed2 {
        } else {
            println!("Files {} and {} differ", file1, file2);
        }
        return Ok(());
    }

    match config.format {
        OutputFormat::Unified => {
            diff_unified(&lines1, &lines2, &lcs, config)?;
        }
        OutputFormat::Context => {
            diff_context(&lines1, &lines2, &lcs, config)?;
        }
        OutputFormat::Normal => {
            diff_normal(&lines1, &lines2, &lcs, config)?;
        }
    }

    Ok(())
}

fn diff_normal(
    lines1: &[String],
    lines2: &[String],
    lcs: &[(usize, usize)],
    config: &DiffConfig,
) -> io::Result<()> {
    let colors = Colors::new(config.color);
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    let mut i = 0;
    let mut j = 0;

    for &(idx1, idx2) in lcs {
        while i < idx1 {
            writeln!(stdout_lock, "{}< {}{}", colors.red(), lines1[i], colors.reset())?;
            i += 1;
        }
        while j < idx2 {
            writeln!(stdout_lock, "{}> {}{}", colors.green(), lines2[j], colors.reset())?;
            j += 1;
        }
        if i < lines1.len() && j < lines2.len() {
            writeln!(stdout_lock, "  {}", lines1[i])?;
            i += 1;
            j += 1;
        }
    }

    while i < lines1.len() {
        writeln!(stdout_lock, "{}< {}{}", colors.red(), lines1[i], colors.reset())?;
        i += 1;
    }
    while j < lines2.len() {
        writeln!(stdout_lock, "{}> {}{}", colors.green(), lines2[j], colors.reset())?;
        j += 1;
    }

    stdout_lock.flush()?;
    Ok(())
}

fn diff_unified(
    lines1: &[String],
    lines2: &[String],
    lcs: &[(usize, usize)],
    config: &DiffConfig,
) -> io::Result<()> {
    let colors = Colors::new(config.color);
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    let hunks = build_hunks(lines1, lines2, lcs, config.context_lines);

    if hunks.is_empty() {
        return Ok(());
    }

    writeln!(stdout_lock, "--- file1")?;
    writeln!(stdout_lock, "+++ file2")?;

    for hunk in hunks {
        writeln!(stdout_lock, "{}@@ -{},{} +{},{} @@{}",
            colors.cyan(),
            hunk.old_start + 1,
            hunk.old_count,
            hunk.new_start + 1,
            hunk.new_count,
            colors.reset()
        )?;

        for line in hunk.lines {
            match line.0 {
                ' ' => writeln!(stdout_lock, " {}", line.1)?,
                '-' => writeln!(stdout_lock, "{}- {}{}", colors.red(), line.1, colors.reset())?,
                '+' => writeln!(stdout_lock, "{}+ {}{}", colors.green(), line.1, colors.reset())?,
                _ => {}
            }
        }
    }

    stdout_lock.flush()?;
    Ok(())
}

fn diff_context(
    lines1: &[String],
    lines2: &[String],
    lcs: &[(usize, usize)],
    config: &DiffConfig,
) -> io::Result<()> {
    let colors = Colors::new(config.color);
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    let hunks = build_hunks(lines1, lines2, lcs, config.context_lines);

    if hunks.is_empty() {
        return Ok(());
    }

    writeln!(stdout_lock, "*** file1")?;
    writeln!(stdout_lock, "--- file2")?;

    for hunk in hunks {
        writeln!(stdout_lock, "{}***************{}", colors.cyan(), colors.reset())?;
        writeln!(stdout_lock, "{}*** {},{} *****{}",
            colors.cyan(),
            hunk.old_start + 1,
            hunk.old_count,
            colors.reset()
        )?;

        for line in &hunk.lines {
            match line.0 {
                ' ' => writeln!(stdout_lock, "  {}", line.1)?,
                '-' => writeln!(stdout_lock, "{}- {}{}", colors.red(), line.1, colors.reset())?,
                '+' => {}
                _ => {}
            }
        }

        writeln!(stdout_lock, "{}--- {},{} ----{}",
            colors.cyan(),
            hunk.new_start + 1,
            hunk.new_count,
            colors.reset()
        )?;

        for line in &hunk.lines {
            match line.0 {
                ' ' => writeln!(stdout_lock, "  {}", line.1)?,
                '+' => writeln!(stdout_lock, "{}+ {}{}", colors.green(), line.1, colors.reset())?,
                '-' => {}
                _ => {}
            }
        }
    }

    stdout_lock.flush()?;
    Ok(())
}

struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    lines: Vec<(char, String)>,
}

fn build_hunks(
    lines1: &[String],
    lines2: &[String],
    lcs: &[(usize, usize)],
    context: usize,
) -> Vec<Hunk> {
    if lines1.is_empty() && lines2.is_empty() {
        return Vec::new();
    }

    let mut edits: Vec<(char, usize, usize)> = Vec::new();
    let mut i = 0;
    let mut j = 0;

    for &(idx1, idx2) in lcs {
        while i < idx1 {
            edits.push(('-', i, j));
            i += 1;
        }
        while j < idx2 {
            edits.push(('+', i, j));
            j += 1;
        }
        if i < lines1.len() && j < lines2.len() {
            edits.push((' ', i, j));
            i += 1;
            j += 1;
        }
    }

    while i < lines1.len() {
        edits.push(('-', i, j));
        i += 1;
    }
    while j < lines2.len() {
        edits.push(('+', i, j));
        j += 1;
    }

    if edits.is_empty() {
        return Vec::new();
    }

    let mut hunks: Vec<Hunk> = Vec::new();
    let mut hunk_start: Option<usize> = None;

    for (idx, (typ, _, _)) in edits.iter().enumerate() {
        if *typ != ' ' {
            if hunk_start.is_none() {
                hunk_start = Some(idx.saturating_sub(context));
            }
        } else if let Some(start) = hunk_start {
            if idx - start > context * 2 {
                let end = idx.min(edits.len());
                hunks.push(build_hunk_from_edits(&edits, start, end, lines1, lines2));
                hunk_start = None;
            }
        }
    }

    if let Some(start) = hunk_start {
        hunks.push(build_hunk_from_edits(&edits, start, edits.len(), lines1, lines2));
    } else if edits.iter().any(|(t, _, _)| *t != ' ') {
        hunks.push(build_hunk_from_edits(&edits, 0, edits.len(), lines1, lines2));
    }

    hunks
}

fn build_hunk_from_edits(
    edits: &[(char, usize, usize)],
    start: usize,
    end: usize,
    lines1: &[String],
    lines2: &[String],
) -> Hunk {
    let mut old_start = usize::MAX;
    let mut new_start = usize::MAX;
    let mut old_count = 0;
    let mut new_count = 0;
    let mut lines = Vec::new();

    for i in start..end {
        let (typ, i1, i2) = edits[i];
        match typ {
            ' ' => {
                if old_start == usize::MAX {
                    old_start = i1;
                }
                if new_start == usize::MAX {
                    new_start = i2;
                }
                old_count += 1;
                new_count += 1;
                lines.push((' ', lines1[i1].clone()));
            }
            '-' => {
                if old_start == usize::MAX {
                    old_start = i1;
                }
                old_count += 1;
                lines.push(('-', lines1[i1].clone()));
            }
            '+' => {
                if new_start == usize::MAX {
                    new_start = i2;
                }
                new_count += 1;
                lines.push(('+', lines2[i2].clone()));
            }
            _ => {}
        }
    }

    if old_start == usize::MAX {
        old_start = 0;
    }
    if new_start == usize::MAX {
        new_start = 0;
    }

    Hunk {
        old_start,
        old_count,
        new_start,
        new_count,
        lines,
    }
}

fn longest_common_subsequence(a: &[String], b: &[String]) -> Vec<(usize, usize)> {
    let m = a.len();
    let n = b.len();

    if m == 0 || n == 0 {
        return Vec::new();
    }

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;

    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }

    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcs_empty() {
        let a: Vec<String> = vec![];
        let b: Vec<String> = vec!["test".to_string()];
        let result = longest_common_subsequence(&a, &b);
        assert!(result.is_empty());
    }

    #[test]
    fn test_lcs_identical() {
        let a = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let b = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = longest_common_subsequence(&a, &b);
        assert_eq!(result, vec![(0, 0), (1, 1), (2, 2)]);
    }

    #[test]
    fn test_lcs_partial() {
        let a = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let b = vec!["a".to_string(), "c".to_string()];
        let result = longest_common_subsequence(&a, &b);
        assert_eq!(result, vec![(0, 0), (2, 1)]);
    }
}
