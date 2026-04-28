use std::io::{self, Write};
use std::time::SystemTime;
use atty;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" | "err" => LogLevel::Error,
            "warn" | "warning" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" | "dbg" => LogLevel::Debug,
            "trace" | "trce" => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

pub struct Logger {
    level: LogLevel,
    color: bool,
    prefix: String,
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            color: atty::is(atty::Stream::Stderr),
            prefix: String::new(),
        }
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    pub fn with_color(mut self, color: bool) -> Self {
        self.color = color;
        self
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    fn should_log(&self, level: LogLevel) -> bool {
        level as u8 <= self.level as u8
    }

    fn get_timestamp(&self) -> String {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}.{:03}", now.as_secs(), now.subsec_millis())
    }

    fn write_log(&self, level: LogLevel, msg: &str) -> io::Result<()> {
        if !self.should_log(level) {
            return Ok(());
        }

        let stderr = io::stderr();
        let mut lock = stderr.lock();

        let timestamp = self.get_timestamp();
        let level_str = level.as_str();

        if self.color {
            let color_code = match level {
                LogLevel::Error => "\x1b[31m",
                LogLevel::Warn => "\x1b[33m",
                LogLevel::Info => "\x1b[32m",
                LogLevel::Debug => "\x1b[36m",
                LogLevel::Trace => "\x1b[90m",
            };
            let reset = "\x1b[0m";
            if self.prefix.is_empty() {
                writeln!(lock, "{} [{}] {} {}{}{}", timestamp, level_str, color_code, level_str, reset, msg)?;
            } else {
                writeln!(lock, "{} [{}] {} {}{}{} {}", timestamp, level_str, color_code, level_str, reset, self.prefix, msg)?;
            }
        } else {
            if self.prefix.is_empty() {
                writeln!(lock, "{} [{}] {}", timestamp, level_str, msg)?;
            } else {
                writeln!(lock, "{} [{}] {} {}", timestamp, level_str, self.prefix, msg)?;
            }
        }

        lock.flush()
    }

    pub fn error(&self, msg: &str) {
        let _ = self.write_log(LogLevel::Error, msg);
    }

    pub fn warn(&self, msg: &str) {
        let _ = self.write_log(LogLevel::Warn, msg);
    }

    pub fn info(&self, msg: &str) {
        let _ = self.write_log(LogLevel::Info, msg);
    }

    pub fn debug(&self, msg: &str) {
        let _ = self.write_log(LogLevel::Debug, msg);
    }

    pub fn trace(&self, msg: &str) {
        let _ = self.write_log(LogLevel::Trace, msg);
    }

    pub fn log(&self, level: LogLevel, msg: &str) {
        let _ = self.write_log(level, msg);
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

pub fn init_logger(level: LogLevel) -> Logger {
    Logger::new(level)
}

pub fn init_logger_with_prefix(prefix: &str, level: LogLevel) -> Logger {
    Logger::new(level).with_prefix(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

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
