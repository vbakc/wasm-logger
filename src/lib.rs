//! A simple logger for front end wasm web app.
//!
//! Please see [README](https://gitlab.com/limira-rs/wasm-logger/blob/master/README.md) for documentation.
#![deny(missing_docs)]
use log::{Level, Log, Metadata, Record};
use wasm_bindgen::prelude::*;
use web_sys::console;

/// Specify timestamp format
pub enum TimestampFormat {
    /// https://www.rfc-editor.org/rfc/rfc2822
    Rfc2822,

    /// https://www.rfc-editor.org/rfc/rfc3339
    Rfc3339,

    /// Custom format string for chrono::DateTime
    Custom(String),
}

/// Specify what to be logged
pub struct Config {
    level: Level,
    module_prefix: Option<String>,
    timestamp_format: Option<TimestampFormat>,
    message_location: MessageLocation,
}

/// Specify where the message will be logged.
pub enum MessageLocation {
    /// The message will be on the same line as other info (level, path...)
    SameLine,
    /// The message will be on its own line, a new after other info.
    NewLine,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            level: Level::Debug,
            module_prefix: None,
            message_location: MessageLocation::SameLine,
            timestamp_format: None,
        }
    }
}

impl Config {
    /// Specify the maximum level you want to log
    pub fn new(level: Level) -> Self {
        Self {
            level,
            module_prefix: None,
            message_location: MessageLocation::SameLine,
            timestamp_format: None,
        }
    }

    /// Configure timestamp format.
    /// If not configured, timestamp will not be used.
    pub fn timestamp_format(mut self, ts_fmt: TimestampFormat) -> Self {
        self.timestamp_format = Some(ts_fmt);
        self
    }

    /// Configure the `target` of the logger. If specified, the logger
    /// only output for `log`s in module that its path starts with
    /// `module_prefix`. wasm-logger only supports single prefix. Only
    /// the last call to `module_prefix` has effect if you call it multiple times.
    pub fn module_prefix(mut self, module_prefix: &str) -> Self {
        self.module_prefix = Some(module_prefix.to_string());
        self
    }

    /// Put the message on a new line, separated from other information
    /// such as level, file path, line number.
    pub fn message_on_new_line(mut self) -> Self {
        self.message_location = MessageLocation::NewLine;
        self
    }
}

/// The log styles
struct Style {
    lvl_trace: String,
    lvl_debug: String,
    lvl_info: String,
    lvl_warn: String,
    lvl_error: String,
    tgt: String,
    args: String,
}

impl Style {
    fn new() -> Style {
        let base = String::from("color: white; padding: 0 3px; background:");
        Style {
            lvl_trace: format!("{} gray;", base),
            lvl_debug: format!("{} blue;", base),
            lvl_info: format!("{} green;", base),
            lvl_warn: format!("{} orange;", base),
            lvl_error: format!("{} darkred;", base),
            tgt: String::from("font-weight: bold; color: inherit"),
            args: String::from("background: inherit; color: inherit"),
        }
    }
}

/// The logger
struct WasmLogger {
    config: Config,
    style: Style,
}

impl Log for WasmLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        if let Some(ref prefix) = self.config.module_prefix {
            metadata.target().starts_with(prefix)
        } else {
            true
        }
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let style = &self.style;
            let message_separator = match self.config.message_location {
                MessageLocation::NewLine => "\n",
                MessageLocation::SameLine => " ",
            };
            let timestamp = self
                .config
                .timestamp_format
                .as_ref()
                .map(|ts_fmt| {
                    chrono::NaiveDateTime::from_timestamp_millis(js_sys::Date::now() as i64)
                        .map(|ndt| chrono::DateTime::<chrono::Utc>::from_utc(ndt, chrono::Utc))
                        .map(|dt| match ts_fmt {
                            TimestampFormat::Rfc2822 => dt.to_rfc2822(),
                            TimestampFormat::Rfc3339 => dt.to_rfc3339(),
                            TimestampFormat::Custom(fmt) => dt.format(&fmt).to_string(),
                        })
                })
                .flatten()
                .map_or("".to_string(), |s| format!("{s} "));
            let s = format!(
                "%c{}%c {}{}:{}%c{}{}",
                record.level(),
                timestamp,
                record.file().unwrap_or_else(|| record.target()),
                record
                    .line()
                    .map_or_else(|| "[Unknown]".to_string(), |line| line.to_string()),
                message_separator,
                record.args(),
            );
            let s = JsValue::from_str(&s);
            let tgt_style = JsValue::from_str(&style.tgt);
            let args_style = JsValue::from_str(&style.args);

            match record.level() {
                Level::Trace => console::debug_4(
                    &s,
                    &JsValue::from(&style.lvl_trace),
                    &tgt_style,
                    &args_style,
                ),
                Level::Debug => console::log_4(
                    &s,
                    &JsValue::from(&style.lvl_debug),
                    &tgt_style,
                    &args_style,
                ),
                Level::Info => {
                    console::info_4(&s, &JsValue::from(&style.lvl_info), &tgt_style, &args_style)
                }
                Level::Warn => {
                    console::warn_4(&s, &JsValue::from(&style.lvl_warn), &tgt_style, &args_style)
                }
                Level::Error => console::error_4(
                    &s,
                    &JsValue::from(&style.lvl_error),
                    &tgt_style,
                    &args_style,
                ),
            }
        }
    }

    fn flush(&self) {}
}

/// Initialize the logger which the given config. If failed, it will log a message to the the browser console.
///
/// ## Examples
/// ```rust
/// wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
/// ```
/// or
/// ```rust
/// wasm_logger::init(wasm_logger::Config::with_prefix(log::Level::Debug, "some::module"));
/// ```
pub fn init(config: Config) {
    let max_level = config.level;
    let wl = WasmLogger {
        config,
        style: Style::new(),
    };

    match log::set_boxed_logger(Box::new(wl)) {
        Ok(_) => log::set_max_level(max_level.to_level_filter()),
        Err(e) => console::error_1(&JsValue::from(e.to_string())),
    }
}
