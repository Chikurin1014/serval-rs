//! Log messages from the device, as the `log` crate has them: each of a level,
//! shown as `env_logger` does (with no time or target, kept apart).

use std::fmt;
use std::str::FromStr;

/// How much a log message matters, as `log::Level`: the more it does, the
/// less it is (`Error < Trace`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Something went wrong.
    Error = 1,
    /// Something may go wrong.
    Warn,
    /// What happens, as it goes.
    Info,
    /// Details, for finding a fault.
    Debug,
    /// The finest details.
    Trace,
}

impl LogLevel {
    /// All of them, from the one that matters most.
    pub const ALL: [Self; 5] = [
        LogLevel::Error,
        LogLevel::Warn,
        LogLevel::Info,
        LogLevel::Debug,
        LogLevel::Trace,
    ];

    /// As `log::Level::as_str`: e.g. `ERROR`.
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}

/// A text that names no level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseLogLevelError(String);

impl fmt::Display for ParseLogLevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No log level '{}'", self.0)
    }
}

impl std::error::Error for ParseLogLevelError {}

impl FromStr for LogLevel {
    type Err = ParseLogLevelError;

    /// As `log::Level`'s: its name in any case (e.g. `warn`), around spaces;
    /// `warning` too, as many devices write it.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let name = text.trim();
        if name.eq_ignore_ascii_case("warning") {
            return Ok(LogLevel::Warn);
        }
        LogLevel::ALL
            .into_iter()
            .find(|level| level.as_str().eq_ignore_ascii_case(name))
            .ok_or_else(|| ParseLogLevelError(text.to_string()))
    }
}

/// A log message and its level, as `log::Record`'s (its time kept apart, as
/// for all data).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    level: LogLevel,
    message: String,
}

impl Log {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }

    pub fn level(&self) -> LogLevel {
        self.level
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Log {
    /// As `env_logger` with no time or target: `[ERROR] message`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.level, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::{Log, LogLevel};

    #[test]
    fn levels_matter_less_down_to_trace() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Debug < LogLevel::Trace);
        assert_eq!(LogLevel::ALL.iter().max(), Some(&LogLevel::Trace));
        assert_eq!(LogLevel::Error as u8, 1);
    }

    #[test]
    fn levels_parse_in_any_case() {
        assert_eq!("warn".parse(), Ok(LogLevel::Warn));
        assert_eq!(" Info ".parse(), Ok(LogLevel::Info));
        assert_eq!("TRACE".parse(), Ok(LogLevel::Trace));
        assert_eq!("Warning".parse(), Ok(LogLevel::Warn));
        assert!("fatal".parse::<LogLevel>().is_err());
        assert_eq!(
            "fatal".parse::<LogLevel>().unwrap_err().to_string(),
            "No log level 'fatal'"
        );
    }

    #[test]
    fn logs_show_as_env_logger_does() {
        assert_eq!(LogLevel::Warn.to_string(), "WARN");
        assert_eq!(format!("{:<5}|", LogLevel::Info), "INFO |");
        let log = Log::new(LogLevel::Error, "sensor not found");
        assert_eq!(log.to_string(), "[ERROR] sensor not found");
        assert_eq!(
            (log.level(), log.message()),
            (LogLevel::Error, "sensor not found")
        );
    }
}
