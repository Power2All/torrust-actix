use fern::colors::{Color, ColoredLevelConfig};
use log::info;
use crate::config::Configuration;

pub fn setup_logging(config: &Configuration)
{
    let level = match config.log_level.as_str() {
        "off" => log::LevelFilter::Off,
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => {
            panic!("Unknown log level encountered: '{}'", config.log_level.as_str());
        }
    };

    let colors = ColoredLevelConfig::new()
        .trace(Color::Cyan)
        .debug(Color::Magenta)
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Red);

    if let Err(_err) = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} [{:width$}][{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.9f"),
                colors.color(record.level()),
                record.target(),
                message,
                width = 5
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()
    {
        panic!("Failed to initialize logging.")
    }
    info!("logging initialized.");
}