use std::sync::atomic::{AtomicU8, Ordering};

use log::{Level, LevelFilter};
use snb_core::logger::Logger;

/// Default logger that writes to stdout with level-coloured prefixes.
///
/// Messages below the configured level are dropped.
///
/// ```text
/// [INFO] MyPlugin: plugin loaded
/// [WARN] Bot: plugin not found
/// [ERROR] echo: command failed
/// ```
pub struct StdoutLogger {
    min_level: AtomicU8,
}

impl StdoutLogger {
    pub fn new(level: LevelFilter) -> Self {
        Self {
            min_level: AtomicU8::new(level as u8),
        }
    }

    pub fn set_level(&self, level: LevelFilter) {
        self.min_level.store(level as u8, Ordering::Relaxed);
    }

    fn min_level(&self) -> LevelFilter {
        match self.min_level.load(Ordering::Relaxed) {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            5 => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    }
}

impl Logger for StdoutLogger {
    fn log(&self, level: u8, source: &str, message: &str) {
        let level = match level {
            1 => Level::Error,
            2 => Level::Warn,
            3 => Level::Info,
            4 => Level::Debug,
            5 => Level::Trace,
            _ => return,
        };
        let Some(max_level) = self.min_level().to_level() else {
            return;
        };
        if level > max_level {
            return;
        }
        let prefix = match level {
            Level::Trace => "\x1b[35mTRACE\x1b[0m", // magenta
            Level::Debug => "\x1b[36mDEBUG\x1b[0m", // cyan
            Level::Info => "\x1b[32mINFO \x1b[0m",  // green
            Level::Warn => "\x1b[33mWARN \x1b[0m",  // yellow
            Level::Error => "\x1b[31mERROR\x1b[0m", // red
        };
        println!("[{}] {}: {}", prefix, source, message);
    }
}
