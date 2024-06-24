use tokio::task::JoinHandle;
use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Returns a tracing subscriber that writes to stdout.
/// It tries to read the filter from the `RUST_LOG` environment variable.
///
/// # Parameters
/// - `name`: The name of the service.
/// - `env_filter`: The default filter used to determine the verbosity of the logs.
/// - `sink`: The sink to write the logs to.
///
/// # Returns
/// The tracing subscriber.
pub fn get_subscriber<T>(name: String, env_filter: String, sink: T) -> impl Subscriber + Send + Sync
where
    T: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Sets the given tracing subscriber as the global subscriber.
/// It also configures the global logger to write logs using the tracing subscriber.
///
/// # Parameters
/// - `subscriber`: The tracing subscriber to use.
///
/// # Panics
/// This function panics if it fails to set the global default subscriber.
///
/// # Notes
/// This function should be called only once.
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger.");
    set_global_default(subscriber).expect("Failed to set subscriber.");
}

/// Spawns a blocking task using [tokio::task::spawn_blocking]
/// and ensures that the current tracing span is in scope.
///
/// # Parameters
///
/// - `f`: The function to run in the blocking task.
///
/// # See Also
///
/// - [tokio::task::spawn_blocking]
/// - [tracing::Span::current] and [tracing::Span::in_scope]
pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}
