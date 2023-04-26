use log::{LevelFilter, SetLoggerError};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

pub fn init() -> Result<(), SetLoggerError> {
    TermLogger::init(
        LevelFilter::Info,
        ConfigBuilder::default().build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
}
