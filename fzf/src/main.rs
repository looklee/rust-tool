use std::io::{self, BufRead, Write, BufWriter};
use std::env;
use std::process;

struct Config {
    ignore_case: bool,
    preview: Option<String>,
    multi: bool,
    height: Option<usize>,
    help: bool,
}

fn print_help() {
    println!("fzf - fuzzy finder (simplified Rust implementation)

USAGE:
    fzf [OPTIONS] [FILE...]
    echo -e 'line1\\nline2' | fzf [OPTIONS]

OPTIONS:
    -i              Ignore case in search pattern
    --preview CMD   Preview command (placeholder, shows command)
    --multi         Allow multiple selections
    --height LINES  Maximum height of the finder
    -h, --help      Print help information

DESCRIPTION:
    fzf reads lines from stdin or files and provides fuzzy search.
    Type a pattern to filter lines. Use arrow keys or Ctrl+N/Ctrl+P
    to navigate. Press Enter to select.

EXAMPLES:
    fzf file.txt
    ls -la | fzf
    ls -la | fzf -i --preview 'cat <PLACEHOLDER>'
    ls -la | fzf --multi --height 20");
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut ignore_case = false;
    let mut preview = None;
    let mut multi = false;
    let mut height = None;
    let mut help = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-i" => ignore_case = true,
            "--preview" => {
                i += 1;
                if i < args.len() {
                    preview = Some(args[i].clone());
                }
            }
            "--multi" => multi = true,
            "--height" => {
                i += 1;
                if i < args.len() {
                    height = args[i].parse().ok();
                }
            }
            "-h" | "--help" => help = true,
            _ => {}
        }
        i += 1;
    }

    Config {
        ignore_case,
        preview,
        multi,
        height,
        help,
    }
}

/// Simple fuzzy matching: check if all characters in pattern appear in text in order
fn fuzzy_match(pattern: &str, text: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    if pattern_chars.is_empty() {
        return true;
    }

    let mut pi = 0;
    let mut ti = 0;

    while ti < text_chars.len() && pi < pattern_chars.len() {
        let pc = pattern_chars[pi].to_lowercase().next().unwrap_or(pattern_chars[pi]);
        let tc = text_chars[ti].to_lowercase().next().unwrap_or(text_chars[ti]);

        if pc == tc {
            pi += 1;
        }
        ti += 1;
    }

    pi == pattern_chars.len()
}

/// Score a fuzzy match (higher is better)
fn fuzzy_score(pattern: &str, text: &str) -> i32 {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    if pattern_chars.is_empty() {
        return 0;
    }

    let mut score: i32 = 0;
    let mut last_match = -1i32;
    let mut pi = 0;

    for (ti, tc) in text_chars.iter().enumerate() {
        if pi >= pattern_chars.len() {
            break;
        }

        let pc = pattern_chars[pi].to_lowercase().next().unwrap_or(pattern_chars[pi]);
        let tlc = tc.to_lowercase().next().unwrap_or(*tc);

        if pc == tlc {
            // Consecutive match bonus
            if last_match >= 0 && ti as i32 == last_match + 1 {
                score += 3;
            } else {
                score += 1;
            }

            // Start of word bonus
            if ti == 0 || text_chars[ti - 1] == ' ' || text_chars[ti - 1] == '/' || text_chars[ti - 1] == '_' || text_chars[ti - 1] == '-' {
                score += 2;
            }

            last_match = ti as i32;
            pi += 1;
        }
    }

    if pi == pattern_chars.len() {
        score
    } else {
        -1
    }
}

fn main() {
    let config = parse_args();

    if config.help {
        print_help();
        process::exit(0);
    }

    // Read input from files or stdin
    let args: Vec<String> = env::args().skip(1).collect();
    let mut lines: Vec<String> = Vec::new();

    let mut file_args: Vec<String> = Vec::new();
    for arg in &args {
        if !arg.starts_with('-') {
            file_args.push(arg.clone());
        }
    }

    if file_args.is_empty() {
        // Read from stdin
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => lines.push(l),
                Err(_) => break,
            }
        }
    } else {
        for file in &file_args {
            match std::fs::File::open(file) {
                Ok(f) => {
                    for line in io::BufReader::new(f).lines() {
                        match line {
                            Ok(l) => lines.push(l),
                            Err(_) => break,
                        }
                    }
                }
                Err(e) => {
                    eprintln!("fzf: {}: {}", file, e);
                }
            }
        }
    }

    if lines.is_empty() {
        return;
    }

    // Simple interactive fuzzy finder using raw terminal I/O
    let mut stdout = BufWriter::new(io::stdout());
    let mut pattern = String::new();
    let mut selected_indices: Vec<usize> = Vec::new();
    let mut cursor_pos = 0;
    let mut selected_line = 0;

    // Enable raw mode
    enable_raw_mode();

    loop {
        // Filter and sort lines
        let mut matched: Vec<(String, i32)> = Vec::new();
        let search_pattern = if config.ignore_case {
            pattern.to_lowercase()
        } else {
            pattern.clone()
        };

        for line in &lines {
            let score = fuzzy_score(&search_pattern, line);
            if score >= 0 {
                matched.push((line.clone(), score));
            }
        }

        // Sort by score descending
        matched.sort_by(|a, b| b.1.cmp(&a.1));

        // Limit height
        let display_height = config.height.unwrap_or(10).min(20);
        let display_lines: Vec<_> = matched.iter().take(display_height).collect();

        // Clear screen and redraw
        write!(stdout, "\x1b[2J\x1b[H"); // Clear screen, home cursor

        // Show pattern
        write!(stdout, "Pattern: {}\x1b[K\n", pattern);

        // Show matched lines
        for (i, (line, _score)) in display_lines.iter().enumerate() {
            if i == selected_line {
                write!(stdout, "\x1b[7m> {}\x1b[0m\x1b[K\n", line);
            } else {
                write!(stdout, "  {}\x1b[K\n", line);
            }
        }

        // Fill remaining lines
        for _ in display_lines.len()..display_height {
            write!(stdout, "\x1b[K\n");
        }

        // Show preview if configured
        if let Some(ref preview_cmd) = config.preview {
            if selected_line < display_lines.len() {
                let selected = &display_lines[selected_line].0;
                let cmd = preview_cmd.replace("{}", selected);
                write!(stdout, "\n\x1b[36mPreview: {}\x1b[0m\x1b[K\n", cmd);
            }
        }

        // Show multi-select count
        if config.multi && !selected_indices.is_empty() {
            write!(stdout, "\x1b[33mSelected: {}\x1b[0m\x1b[K\n", selected_indices.len());
        }

        // Show cursor in pattern
        write!(stdout, "> {}\x1b[{}D", pattern, 1);

        stdout.flush().unwrap();

        // Read key
        let key = read_key();

        match key {
            Key::Enter => {
                if selected_line < display_lines.len() {
                    if config.multi {
                        let idx = lines.iter().position(|l| l == &display_lines[selected_line].0);
                        if let Some(i) = idx {
                            if selected_indices.contains(&i) {
                                selected_indices.retain(|&x| x != i);
                            } else {
                                selected_indices.push(i);
                            }
                        }
                    } else {
                        // Clear screen
                        write!(stdout, "\x1b[2J\x1b[H");
                        stdout.flush().unwrap();
                        disable_raw_mode();
                        println!("{}", display_lines[selected_line].0);
                        return;
                    }
                }
            }
            Key::CtrlC | Key::Esc => {
                write!(stdout, "\x1b[2J\x1b[H");
                stdout.flush().unwrap();
                disable_raw_mode();
                process::exit(130);
            }
            Key::Up => {
                if selected_line > 0 {
                    selected_line -= 1;
                }
            }
            Key::Down => {
                if selected_line < display_lines.len().saturating_sub(1) {
                    selected_line += 1;
                }
            }
            Key::Backspace => {
                if cursor_pos > 0 {
                    pattern.remove(cursor_pos - 1);
                    cursor_pos -= 1;
                }
            }
            Key::Char(c) => {
                pattern.insert(cursor_pos, c);
                cursor_pos += 1;
            }
            Key::Left => {
                if cursor_pos > 0 {
                    cursor_pos -= 1;
                }
            }
            Key::Right => {
                if cursor_pos < pattern.len() {
                    cursor_pos += 1;
                }
            }
            Key::Tab => {
                if config.multi && selected_line < display_lines.len() {
                    let idx = lines.iter().position(|l| l == &display_lines[selected_line].0);
                    if let Some(i) = idx {
                        if selected_indices.contains(&i) {
                            selected_indices.retain(|&x| x != i);
                        } else {
                            selected_indices.push(i);
                        }
                        if selected_line < display_lines.len().saturating_sub(1) {
                            selected_line += 1;
                        }
                    }
                }
            }
        }
    }
}

enum Key {
    Enter,
    CtrlC,
    Esc,
    Up,
    Down,
    Left,
    Right,
    Backspace,
    Tab,
    Char(char),
}

fn enable_raw_mode() {
    unsafe {
        let mut termios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(libc::STDIN_FILENO, &mut termios) == 0 {
            let original = termios;
            termios.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
            termios.c_cc[libc::VMIN] = 1;
            termios.c_cc[libc::VTIME] = 0;
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios);

            // Store original in a static for cleanup
            static mut ORIGINAL_TERMIOS: libc::termios = libc::termios {
                c_iflag: 0, c_oflag: 0, c_cflag: 0, c_lflag: 0,
                c_line: 0, c_cc: [0; 32], c_ispeed: 0, c_ospeed: 0,
            };
            ORIGINAL_TERMIOS = original;
        }
    }
}

fn disable_raw_mode() {
    unsafe {
        static mut ORIGINAL_TERMIOS: libc::termios = libc::termios {
            c_iflag: 0, c_oflag: 0, c_cflag: 0, c_lflag: 0,
            c_line: 0, c_cc: [0; 32], c_ispeed: 0, c_ospeed: 0,
        };
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &ORIGINAL_TERMIOS);
    }
}

fn read_key() -> Key {
    let mut buf = [0u8; 3];
    let n = unsafe {
        libc::read(libc::STDIN_FILENO, buf.as_mut_ptr() as *mut libc::c_void, 3)
    };

    if n <= 0 {
        return Key::Esc;
    }

    match buf[0] {
        3 => Key::CtrlC,
        13 | 10 => Key::Enter,
        27 => {
            if n == 1 {
                Key::Esc
            } else if n >= 3 && buf[1] == 91 {
                match buf[2] {
                    65 => Key::Up,
                    66 => Key::Down,
                    67 => Key::Right,
                    68 => Key::Left,
                    _ => Key::Esc,
                }
            } else {
                Key::Esc
            }
        }
        127 | 8 => Key::Backspace,
        9 => Key::Tab,
        c if c >= 32 && c < 127 => Key::Char(c as char),
        _ => Key::Esc,
    }
}
