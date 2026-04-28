use std::io;

pub mod colors;
pub mod errors;
pub mod args;
pub mod logger;
pub mod ai;
pub mod evolve;

pub use colors::Colors;
pub use errors::CliError;
pub use args::{parse_flag, parse_opt_value, extract_files_from_args};
pub use logger::{Logger, LogLevel, init_logger, init_logger_with_prefix};
pub use ai::{AiClient, AiConfig, AiProvider, Message, ProjectContext, extract_code_block};
pub use evolve::{EvolutionEngine, ToolInfo, EvolutionReport, EvolutionPriority, EvolutionAction};

pub fn is_terminal() -> bool {
    atty::is(atty::Stream::Stdout)
}

pub fn print_version_and_exit(program: &str, version: &str) {
    println!("{} {}", program, version);
    std::process::exit(0);
}

pub fn print_help_and_exit(help_text: &str) {
    println!("{}", help_text);
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colors_enabled() {
        let colors = Colors::new(true);
        assert_eq!(colors.red(), "\x1b[31m");
        assert_eq!(colors.green(), "\x1b[32m");
        assert_eq!(colors.reset(), "\x1b[0m");
    }

    #[test]
    fn test_colors_disabled() {
        let colors = Colors::new(false);
        assert_eq!(colors.red(), "");
        assert_eq!(colors.green(), "");
        assert_eq!(colors.reset(), "");
    }

    #[test]
    fn test_parse_flag_found() {
        let args = vec!["--verbose", "--debug", "file.txt".to_string()];
        assert!(parse_flag(&args, "--verbose"));
        assert!(parse_flag(&args, "--debug"));
        assert!(!parse_flag(&args, "--silent"));
    }

    #[test]
    fn test_parse_flag_not_found() {
        let args = vec!["file.txt".to_string()];
        assert!(!parse_flag(&args, "--verbose"));
    }

    #[test]
    fn test_parse_opt_value_found() {
        let args = vec!["--output".to_string(), "result.txt".to_string()];
        assert_eq!(parse_opt_value(&args, "--output"), Some("result.txt"));
    }

    #[test]
    fn test_parse_opt_value_not_found() {
        let args = vec!["file.txt".to_string()];
        assert_eq!(parse_opt_value(&args, "--output"), None);
    }

    #[test]
    fn test_extract_files() {
        let args = vec!["--verbose".to_string(), "file1.txt".to_string(), "file2.txt".to_string()];
        let files = extract_files_from_args(&args);
        assert_eq!(files, vec!["file1.txt", "file2.txt"]);
    }

    #[test]
    fn test_is_terminal() {
        let result = is_terminal();
        assert!(result == true || result == false);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("error"), LogLevel::Error);
        assert_eq!(LogLevel::from_str("warn"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("INFO"), LogLevel::Info);
        assert_eq!(LogLevel::from_str("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("trace"), LogLevel::Trace);
        assert_eq!(LogLevel::from_str("unknown"), LogLevel::Info);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
    }

    #[test]
    fn test_logger_default() {
        let logger = Logger::default();
        assert_eq!(logger.level, LogLevel::Info);
    }

    #[test]
    fn test_logger_level_filtering() {
        let logger = Logger::new(LogLevel::Warn);
        assert!(logger.should_log(LogLevel::Error));
        assert!(logger.should_log(LogLevel::Warn));
        assert!(!logger.should_log(LogLevel::Info));
        assert!(!logger.should_log(LogLevel::Debug));
    }
}
