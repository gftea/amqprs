use amqprs::connection::OpenConnectionArguments;
use tracing::{subscriber::DefaultGuard, Level};

// construct a subscriber that prints formatted traces to stdout
pub fn setup_logging(level: Level) -> DefaultGuard {
    // global subscriber as fallback
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    // thread local subscriber
    let subscriber = tracing_subscriber::fmt().with_max_level(level).finish();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_default(subscriber)
}

#[cfg(not(feature = "tls"))]
pub fn build_conn_args() -> OpenConnectionArguments {
    OpenConnectionArguments::new("localhost", Some(5672), "user", "bitnami")
}
#[cfg(feature = "tls")]
pub fn build_conn_args() -> OpenConnectionArguments {
    // TLS specific configuration
    let current_dir = std::env::current_dir().unwrap();
    let current_dir = current_dir.join("../rabbitmq_conf/client/");

    let root_ca_cert = current_dir.join("ca_certificate.pem");
    let client_cert = current_dir.join("client_AMQPRS_TEST_certificate.pem");
    let client_private_key = current_dir.join("client_AMQPRS_TEST_key.pem");
    // domain should match the certificate/key files
    let domain = "AMQPRS_TEST";
    OpenConnectionArguments::new("localhost", Some(5671), "user", "bitnami")
        .tls_adaptor(
            amqprs::tls::TlsAdaptor::with_client_auth(
                Some(root_ca_cert.as_path()),
                client_cert.as_path(),
                client_private_key.as_path(),
                domain.to_owned(),
            )
            .unwrap(),
        )
        .finish()
}
