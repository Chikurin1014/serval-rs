//! Log messages from the device, as the `log` crate has them: each of a level,
//! shown as `env_logger` does (with no time or target, kept apart).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// How much a log message matters, as `log::Level`: the more it does, the
/// less it is (`Error < Trace < None`).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "UPPERCASE")]
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
    /// None given: plain text, as a line with no level. Matters least, so a
    /// filter down to it takes all.
    #[default]
    None,
}

impl LogLevel {
    /// All of them, from the one that matters most.
    pub const ALL: [Self; 6] = [
        LogLevel::Error,
        LogLevel::Warn,
        LogLevel::Info,
        LogLevel::Debug,
        LogLevel::Trace,
        LogLevel::None,
    ];

    /// As `log::Level::as_str`: e.g. `ERROR`.
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
            LogLevel::None => "NONE",
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
    /// `warning` too, as many devices write it. Nothing (or `none`) for None.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let name = text.trim();
        if name.is_empty() {
            return Ok(LogLevel::None);
        }
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

    /// A text with no level, as a line is before its level is read.
    pub fn text(message: impl Into<String>) -> Self {
        Self::new(LogLevel::None, message)
    }

    pub fn level(&self) -> LogLevel {
        self.level
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Log {
    /// As `env_logger` with no time or target: `[ERROR] message`; with no level,
    /// the message alone.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.level {
            LogLevel::None => f.write_str(&self.message),
            level => write!(f, "[{level}] {}", self.message),
        }
    }
}

/// `template` with the levels of a map's inputs in it, by their names: `${level:N}`
/// for the `N`th's (from 0), `${level:max}` for the one that matters most and
/// `${level:min}` least. The rest is left as it is, `$$` too, for the groups of a
/// pattern after.
pub fn expand_levels(template: &str, levels: &[LogLevel]) -> String {
    let mut expanded = String::new();
    let mut rest = template;
    while let Some(dollar) = rest.find('$') {
        expanded.push_str(&rest[..dollar]);
        rest = &rest[dollar..];
        if rest.starts_with("$$") {
            expanded.push_str("$$");
            rest = &rest[2..];
            continue;
        }
        let level = rest
            .strip_prefix("${level:")
            .and_then(|key| Some((key, key.find('}')?)))
            .and_then(|(key, close)| {
                let level = match &key[..close] {
                    "max" => levels.iter().min().copied(),
                    "min" => levels.iter().max().copied(),
                    index => levels.get(index.parse::<usize>().ok()?).copied(),
                }?;
                Some((level, "${level:".len() + close + 1))
            });
        match level {
            Some((level, end)) => {
                expanded.push_str(level.as_str());
                rest = &rest[end..];
            }
            None => {
                expanded.push('$');
                rest = &rest[1..];
            }
        }
    }
    expanded.push_str(rest);
    expanded
}

#[cfg(test)]
mod tests {
    use super::{Log, LogLevel, expand_levels};

    #[test]
    fn levels_matter_less_down_to_none() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Debug < LogLevel::Trace);
        assert!(LogLevel::Trace < LogLevel::None);
        assert_eq!(LogLevel::ALL.iter().min(), Some(&LogLevel::Error));
        assert_eq!(LogLevel::ALL.iter().max(), Some(&LogLevel::None));
        assert_eq!(LogLevel::Error as u8, 1);
    }

    #[test]
    fn levels_parse_in_any_case() {
        assert_eq!("warn".parse(), Ok(LogLevel::Warn));
        assert_eq!(" Info ".parse(), Ok(LogLevel::Info));
        assert_eq!("TRACE".parse(), Ok(LogLevel::Trace));
        assert_eq!("Warning".parse(), Ok(LogLevel::Warn));
        assert_eq!("".parse(), Ok(LogLevel::None));
        assert_eq!("  ".parse(), Ok(LogLevel::None));
        assert_eq!("none".parse(), Ok(LogLevel::None));
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
        assert_eq!(Log::text("plain text").to_string(), "plain text");
        assert_eq!(
            (log.level(), log.message()),
            (LogLevel::Error, "sensor not found")
        );
    }

    #[test]
    fn levels_expand_by_their_names() {
        let levels = [LogLevel::Info, LogLevel::Error, LogLevel::None];
        assert_eq!(expand_levels("${level:0}", &levels), "INFO");
        assert_eq!(expand_levels("[${level:1}] $1", &levels), "[ERROR] $1");
        // That which matters most, and least
        assert_eq!(expand_levels("${level:max}", &levels), "ERROR");
        assert_eq!(expand_levels("${level:min}", &levels), "NONE");
        // Not of an input, or not a level: left alone
        assert_eq!(
            expand_levels("${level:3} ${name} $${level:0}", &levels),
            "${level:3} ${name} $${level:0}"
        );
        assert_eq!(expand_levels("${level:0", &levels), "${level:0");
    }
}
