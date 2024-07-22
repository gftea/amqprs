use amqprs::connection::OpenConnectionArguments;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// construct a subscriber that prints formatted traces to stdout
pub fn setup_logging() {
    // global subscriber with log level according to RUST_LOG
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();
}

#[cfg(not(feature = "tls"))]
pub fn build_conn_args() -> OpenConnectionArguments {
    OpenConnectionArguments::new("localhost", 5672, "user", "bitnami")
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
    OpenConnectionArguments::new("localhost", 5671, "user", "bitnami")
        .tls_adaptor(
            amqprs::tls::TlsAdaptor::with_client_auth(
                Some(root_ca_cert.as_path()),
                client_cert.to_path_buf(),
                client_private_key.to_path_buf(),
                domain.to_owned(),
            )
            .unwrap(),
        )
        .finish()
}
