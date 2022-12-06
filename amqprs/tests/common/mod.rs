use tracing::{Level, subscriber::DefaultGuard};

// construct a subscriber that prints formatted traces to stdout
pub fn setup_logging(level: Level) -> DefaultGuard {
    // global subscriber as fallback
    let subscriber = tracing_subscriber::fmt().with_max_level(Level::ERROR).finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    // thread local subscriber
    let subscriber = tracing_subscriber::fmt().with_max_level(level).finish();
    // use that subscriber to process traces emitted after this point    
    tracing::subscriber::set_default(subscriber)
}
