use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    
    #[error("AI client error: {0}")]
    AiClient(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl ToolError {
    pub fn parse(msg: impl Into<String>) -> Self {
        ToolError::Parse(msg.into())
    }
    
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        ToolError::InvalidArgument(msg.into())
    }
    
    pub fn config(msg: impl Into<String>) -> Self {
        ToolError::Config(msg.into())
    }
    
    pub fn not_implemented(msg: impl Into<String>) -> Self {
        ToolError::NotImplemented(msg.into())
    }
    
    pub fn tool_not_found(name: impl Into<String>) -> Self {
        ToolError::ToolNotFound(name.into())
    }
    
    pub fn file_not_found(path: impl Into<String>) -> Self {
        ToolError::FileNotFound(path.into())
    }
    
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        ToolError::PermissionDenied(msg.into())
    }
    
    pub fn network(msg: impl Into<String>) -> Self {
        ToolError::Network(msg.into())
    }
    
    pub fn serialization(msg: impl Into<String>) -> Self {
        ToolError::Serialization(msg.into())
    }
}

pub type ToolResult<T> = Result<T, ToolError>;

pub fn exit_with_error(msg: &str, code: i32) -> ! {
    eprintln!("Error: {}", msg);
    std::process::exit(code);
}