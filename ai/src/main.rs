fn main() {
    eprintln!("Warning: 'ai' is deprecated, use 'code' instead!");
    eprintln!();
    eprintln!("Migration:");
    eprintln!("  ai 'prompt'           -> code 'prompt'");
    eprintln!("  ai -i                  -> code -i");
    eprintln!("  ai -p qwen 'prompt'   -> code -p qwen 'prompt'");
    eprintln!();
    eprintln!("For IDE mode:");
    eprintln!("  ai -i                  -> code -i");
    eprintln!("For tool management:");
    eprintln!("  ai -M list             -> code -M list");
    std::process::exit(1);
}
