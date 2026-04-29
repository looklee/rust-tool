use tracing_subscriber::{prelude::*, EnvFilter};

use super::errors::{ToolError, ToolResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    
    pub fn to_tracing_filter(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

pub fn init_logging(level: &str) -> ToolResult<()> {
    let log_level = LogLevel::from_str(level);
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("rust_tool={}", log_level.to_tracing_filter())));
    
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
    
    tracing::info!("Logging initialized at level: {}", log_level.as_str());
    
    Ok(())
}

pub fn init_logging_with_file(level: &str, file_path: &str) -> ToolResult<()> {
    let log_level = LogLevel::from_str(level);
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("rust_tool={}", log_level.to_tracing_filter())));
    
    let file_appender = tracing_appender::rolling::hourly(".", "rust-tool.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false).with_writer(non_blocking))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
    
    tracing::info!("Logging initialized at level: {} (with file output)", log_level.as_str());
    
    Ok(())
}