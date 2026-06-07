/// Logger abstraction used by the bot and plugins.
///
/// After [`crate::context::set_bot`] is called, the standard `log` crate macros
/// (`log::info!`, `log::debug!`, etc.) are automatically routed through this
/// logger via the log bridge.
pub trait Logger: Send + Sync {
    fn log(&self, level: u8, source: &str, message: &str);

    fn debug(&self, source: &str, msg: &str) {
        self.log(log::Level::Debug as u8, source, msg);
    }
    fn info(&self, source: &str, msg: &str) {
        self.log(log::Level::Info as u8, source, msg);
    }
    fn warn(&self, source: &str, msg: &str) {
        self.log(log::Level::Warn as u8, source, msg);
    }
    fn error(&self, source: &str, msg: &str) {
        self.log(log::Level::Error as u8, source, msg);
    }
}
