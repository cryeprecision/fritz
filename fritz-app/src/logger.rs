use anyhow::Context;
use log::LevelFilter;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

pub fn init() -> anyhow::Result<()> {
    let config = {
        let mut config = ConfigBuilder::default();
        // add filters to ignore stuff
        config
            .add_filter_ignore_str("hyper::")
            .add_filter_ignore_str("rustls::")
            .add_filter_ignore_str("reqwest::");
        // log time should be in the local timezine
        if config.set_time_offset_to_local().is_err() {
            log::warn!("couldn't set log time offset to local time");
        }
        config.build()
    };

    TermLogger::init(
        LevelFilter::Info,
        config,
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("couldn't init logger")
}
