fn main() {
    eprintln!("Warning: 'evolve' is deprecated, use 'code -M' instead!");
    eprintln!();
    eprintln!("Migration:");
    eprintln!("  evolve list           -> code -M list");
    eprintln!("  evolve diagnose       -> code -M evolve");
    eprintln!("  evolve update <tool>  -> code -M update <tool>");
    eprintln!();
    eprintln!("The 'code -M' command provides all evolve features plus:");
    eprintln!("  - Unified tool management");
    eprintln!("  - AI-powered tool analysis");
    eprintln!("  - Better integration with code IDE mode");
    std::process::exit(1);
}
