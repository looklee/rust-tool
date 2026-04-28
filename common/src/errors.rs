use std::fmt;
use std::io;

#[derive(Debug)]
pub enum CliError {
    Io(io::Error),
    Parse(String),
    InvalidArgument(String),
    Custom(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Io(e) => write!(f, "IO error: {}", e),
            CliError::Parse(s) => write!(f, "Parse error: {}", s),
            CliError::InvalidArgument(s) => write!(f, "Invalid argument: {}", s),
            CliError::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(err: io::Error) -> Self {
        CliError::Io(err)
    }
}

pub fn exit_with_error(msg: &str, code: i32) -> ! {
    eprintln!("Error: {}", msg);
    std::process::exit(code);
}
