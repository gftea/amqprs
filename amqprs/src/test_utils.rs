#[cfg(test)]
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

//////////////////////////////////////////////////////////////////
// construct a subscriber that prints formatted traces to stdout
#[cfg(test)]
pub fn setup_logging() {
    // global subscriber with log level according to RUST_LOG
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();
}
