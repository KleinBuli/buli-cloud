use chrono::Local;
use std::fmt;

/// Represents the severity level of a log message.
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl fmt::Display for LogLevel {
    /// Converts the log level into its uppercase string representation.
    ///
    /// # Examples
    ///
    /// ```text
    /// INFO
    /// WARN
    /// ERROR
    /// DEBUG
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
        };

        write!(f, "{level}")
    }
}

/// Writes a formatted log message to the console.
///
/// The log message contains:
///
/// - the current local timestamp
/// - the log level
/// - the provided message
///
/// Messages with [`LogLevel::Error`] are written to `stderr`.
/// All other log levels are written to `stdout`.
///
/// # Format
///
/// ```text
/// [YYYY-MM-DD HH:MM:SS] [LEVEL] - Message
/// ```
///
/// # Example
///
/// ```text
/// [2026-09-30 15:48:12] [INFO] - Core initialized
/// ```
///
/// # Arguments
///
/// * `level` - The severity level of the log message.
/// * `text` - The message that should be logged.
pub fn log(level: LogLevel, text: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    let (color, label) = match level {
        LogLevel::Info => ("\x1b[32m", "INFO"),
        LogLevel::Warn => ("\x1b[33m", "WARN"),
        LogLevel::Error => ("\x1b[31m", "ERROR"),
        LogLevel::Debug => ("\x1b[36m", "DEBUG"),
    };

    println!("[{timestamp}] {color}[{label}]\x1b[0m {text}");
}
