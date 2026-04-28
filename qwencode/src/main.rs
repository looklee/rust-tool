fn main() {
    eprintln!("Warning: 'qwencode' is deprecated, use 'code' instead!");
    eprintln!();
    eprintln!("Migration:");
    eprintln!("  qwencode -i            -> code -i");
    eprintln!("  qwencode -p qwen -i   -> code -p qwen -i");
    eprintln!("  qwencode -y -i        -> code -y -i");
    eprintln!();
    eprintln!("The 'code' tool provides all qwencode features plus:");
    eprintln!("  - Unified chat/IDE/manager modes");
    eprintln!("  - More AI providers");
    eprintln!("  - Better code analysis");
    std::process::exit(1);
}
