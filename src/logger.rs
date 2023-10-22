use log::{LevelFilter, SetLoggerError};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

pub fn init() -> Result<(), SetLoggerError> {
    TermLogger::init(
        LevelFilter::Info,
        ConfigBuilder::default()
            .add_filter_ignore_str("hyper::")
            .add_filter_ignore_str("rustls::")
            .add_filter_ignore_str("reqwest::")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
}
