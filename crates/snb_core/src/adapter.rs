use std::sync::Arc;

use crate::context::BotContext;

/// A plugin component that continuously receives external events, running on
/// a dedicated OS thread spawned by the bot.
///
/// Adapters are written with the [`#[adapter]`](snb_macros::adapter) attribute
/// macro: author an inherent `async fn run(&self, bot: Arc<dyn BotContext>)` and
/// the macro generates this trait impl, wrapping the body in [`run_async`] so the
/// tokio runtime is created inside the plugin's own cdylib (independent from the
/// host's, avoiding issues with dynamically loaded plugins that carry their own
/// copies of tokio's statics).
///
/// ```ignore
/// use snb_macros::adapter;
///
/// struct MyAdapter;
///
/// #[adapter]
/// impl MyAdapter {
///     async fn run(&self, bot: Arc<dyn BotContext>) {
///         bot.emit_event(Event::message("my", "hello"));
///     }
/// }
/// ```
pub trait Adapter: Send + Sync {
    fn run(&self, bot: Arc<dyn BotContext>);
}

/// Run an async closure as an adapter body, creating a dedicated single-threaded
/// tokio runtime on the current OS thread.
///
/// Used by the [`#[adapter]`](snb_macros::adapter) macro to bridge the authored
/// `async fn run` to the synchronous [`Adapter::run`]. Adapters should prefer the
/// macro over calling this directly.
pub fn run_async<F: std::future::Future<Output = ()> + Send>(future: F) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("run_async: failed to create tokio runtime");
    rt.block_on(future);
}
