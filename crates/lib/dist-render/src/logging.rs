use lazy_static::lazy_static;
use std::sync::Mutex;

/// A single log entry captured for the in-app console.
#[derive(Clone)]
pub struct LogEntry {
    pub level: log::Level,
    pub target: String,
    pub message: String,
    pub timestamp: String,
}

lazy_static! {
    static ref LOG_BUFFER: Mutex<Vec<LogEntry>> = Mutex::new(Vec::new());
}

/// Maximum entries kept in the ring buffer.
const MAX_LOG_ENTRIES: usize = 2048;

/// Drain all new log entries since the last call (or all if first call).
pub fn drain_log_entries() -> Vec<LogEntry> {
    let mut buf = LOG_BUFFER.lock().unwrap();
    std::mem::take(&mut *buf)
}

pub fn set_up_logging(default_log_level: log::LevelFilter) -> anyhow::Result<()> {
    use fern::colors::{Color, ColoredLevelConfig};

    // configure colors for the whole line
    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        // we actually don't need to specify the color for debug and info, they are white by default
        .info(Color::White)
        .debug(Color::White)
        // depending on the terminals color scheme, this is the same as the background color
        .trace(Color::BrightBlack);

    // configure colors for the name of the level.
    // since almost all of them are the some as the color for the whole line, we
    // just clone `colors_line` and overwrite our changes
    let colors_level = colors_line.info(Color::Green);
    // here we set up our fern Dispatch

    let console_out = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date}][{target}][{level}{color_line}] {message}\x1B[0m",
                color_line = format_args!(
                    "\x1B[{}m",
                    colors_line.get_color(&record.level()).to_fg_str()
                ),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                target = record.target(),
                level = colors_level.color(record.level()),
                message = message,
            ));
        })
        // set the default log level. to filter out verbose log messages from dependencies, set
        // this to Warn and overwrite the log level for your crate.
        .level(default_log_level)
        // change log levels for individual modules. Note: This looks for the record's target
        // field which defaults to the module path but can be overwritten with the `target`
        // parameter:
        // `info!(target="special_target", "This log message is about special_target");`
        // .level_for("dist_render::device", log::LevelFilter::Trace)
        // output to stdout
        .chain(std::io::stdout());

    let file_out = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{date}][{target}][{level}] {message}",
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                target = record.target(),
                level = record.level(),
                message = message,
            ));
        })
        .level(log::LevelFilter::Trace)
        .level_for("async_io", log::LevelFilter::Warn)
        .level_for("polling", log::LevelFilter::Warn)
        .chain(
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open("output.log")
                .unwrap(),
        );

    let gui_out = fern::Dispatch::new()
        .format(move |out, message, record| {
            let entry = LogEntry {
                level: record.level(),
                target: record.target().to_string(),
                message: format!("{}", message),
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            };
            let mut buf = LOG_BUFFER.lock().unwrap();
            buf.push(entry);
            if buf.len() > MAX_LOG_ENTRIES {
                let excess = buf.len() - MAX_LOG_ENTRIES;
                buf.drain(..excess);
            }
            out.finish(format_args!(""));
        })
        .level(default_log_level)
        .level_for("async_io", log::LevelFilter::Warn)
        .level_for("polling", log::LevelFilter::Warn)
        .chain(fern::Output::call(|_| {}));

    fern::Dispatch::new()
        .chain(console_out)
        .chain(file_out)
        .chain(gui_out)
        .apply()
        .map_err(|err| anyhow::anyhow!("{:?}", err))
}
