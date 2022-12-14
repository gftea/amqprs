//! This module provides TLS adaptor.
//!
//! If TLS adaptor is set in [`OpenConnectionArguments`], and given to [`Connection::open`],
//! the TLS network stream will be used instead of regular TCP stream.
//!
//! [`OpenConnectionArguments`]: ../connection/struct.OpenConnectionArguments.html
//! [`Connection::open`]: ../connection/struct.Connection.html#method.open

use std::{fs::File, io::BufReader, path::Path, sync::Arc};
use tokio_rustls::{
    rustls::{Certificate, ClientConfig, OwnedTrustAnchor, PrivateKey, RootCertStore},
    webpki, TlsConnector,
};

/// The TLS adaptor used to enable TLS network stream.
///
/// Currently, it depends on [`tokio-rustls`] and provides convenient
/// methods to create a TLS adaptor. See details of each method.
///
/// [`tokio-rustls`]: https://docs.rs/tokio-rustls/latest/tokio_rustls
#[derive(Clone)]
pub struct TlsAdaptor {
    pub(crate) connector: TlsConnector,
    pub(crate) domain: String,
}

impl TlsAdaptor {
    /// Create TlsAdaptor from customized connector.
    ///
    /// User can use `tokio-rustls` api to create customized `TlsConnector`,
    /// then pass in to create its own TlsAdaptor.
    pub fn new(connector: TlsConnector, domain: String) -> Self {
        Self { connector, domain }
    }

    /// Build SSL/TLS without client authentication.
    ///
    /// # Errors
    ///
    /// Return errors if any I/O failure.
    pub fn without_client_auth(
        root_ca_cert: Option<&Path>,
        domain: String,
    ) -> std::io::Result<Self> {
        let root_cert_store = Self::build_root_store(root_ca_cert)?;

        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));

        Ok(Self { connector, domain })
    }

    /// Build SSL/TLS with client authentication.
    ///
    /// # Errors
    ///
    /// Return errors if any I/O failure.
    ///
    /// # Panics
    ///
    /// Panics if private key is invalid.
    pub fn with_client_auth(
        root_ca_cert: Option<&Path>,
        client_cert: &Path,
        client_private_key: &Path,
        domain: String,
    ) -> std::io::Result<Self> {
        let root_cert_store = Self::build_root_store(root_ca_cert)?;
        let client_certs = Self::build_client_certificates(client_cert)?;
        let client_keys = Self::build_client_private_keys(client_private_key)?;
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_single_cert(client_certs, client_keys.into_iter().next().unwrap())
            .unwrap();
        let connector = TlsConnector::from(Arc::new(config));

        Ok(Self { connector, domain })
    }

    fn build_root_store(root_ca_cert: Option<&Path>) -> std::io::Result<RootCertStore> {
        let mut root_store = RootCertStore::empty();
        if let Some(root_ca_cert) = root_ca_cert {
            let mut pem = BufReader::new(File::open(root_ca_cert)?);
            let certs = rustls_pemfile::certs(&mut pem)?;
            let trust_anchors = certs.iter().map(|cert| {
                let ta = webpki::TrustAnchor::try_from_cert_der(&cert[..]).unwrap();
                OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            });
            root_store.add_server_trust_anchors(trust_anchors);
        } else {
            root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(
                |ta| {
                    OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                },
            ));
        }
        Ok(root_store)
    }

    fn build_client_certificates(client_cert: &Path) -> std::io::Result<Vec<Certificate>> {
        let mut pem = BufReader::new(File::open(client_cert)?);
        let certs = rustls_pemfile::certs(&mut pem)?;
        let certs = certs.into_iter().map(Certificate);
        Ok(certs.collect())
    }

    fn build_client_private_keys(client_private_key: &Path) -> std::io::Result<Vec<PrivateKey>> {
        let mut pem = BufReader::new(File::open(client_private_key)?);
        let keys = rustls_pemfile::pkcs8_private_keys(&mut pem)?;
        let keys = keys.into_iter().map(PrivateKey);
        Ok(keys.collect())
    }
}
