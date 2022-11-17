use tracing::{Level, subscriber::{DefaultGuard, SetGlobalDefaultError}};

pub fn setup_logging(level: Level)  -> Result<(), SetGlobalDefaultError> {
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::fmt().with_max_level(level).finish();

    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber)
}
