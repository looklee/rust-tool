pub mod ai;
pub mod cli;
pub mod colors;
pub mod config;
pub mod errors;
pub mod evolve;
pub mod logging;
pub mod termux;
pub mod utils;

pub use ai::{AiClient, AiConfig, AiProvider, Message, ProjectContext, extract_code_block};
pub use cli::CliOptions;
pub use colors::Colors;
pub use config::{AiConfig as ConfigAiConfig, ToolConfig, UiConfig, LoggingConfig};
pub use errors::{ToolError, ToolResult, exit_with_error};
pub use evolve::{EvolutionEngine, ToolInfo, EvolutionReport, EvolutionPriority, EvolutionAction};
pub use logging::{LogLevel, init_logging, init_logging_with_file};
pub use termux::TermuxInfo;
pub use utils::*;