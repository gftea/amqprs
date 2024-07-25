//! This module provides TLS adaptor.
//!
//! If TLS adaptor is set in [`OpenConnectionArguments`], and given to [`Connection::open`],
//! the TLS network stream will be used instead of regular TCP stream.
//!
//! [`OpenConnectionArguments`]: ../connection/struct.OpenConnectionArguments.html
//! [`Connection::open`]: ../connection/struct.Connection.html#method.open

use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_rustls::{
    rustls::{ClientConfig, RootCertStore},
    TlsConnector,
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
        let client_certs: Vec<CertificateDer> = Self::build_client_certificates(client_cert)?;
        let client_keys: Vec<PrivateKeyDer> = Self::build_client_private_keys(client_private_key)?;
        let config = ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_client_auth_cert(client_certs, client_keys.into_iter().next().unwrap())
            .unwrap();
        let connector = TlsConnector::from(Arc::new(config));

        Ok(Self { connector, domain })
    }

    fn build_root_store(root_ca_cert: Option<&Path>) -> std::io::Result<RootCertStore> {
        let mut root_store = RootCertStore::empty();
        if let Some(root_ca_cert) = root_ca_cert {
            let mut pem = BufReader::new(File::open(root_ca_cert)?);

            let certs = rustls_pemfile::certs(&mut pem)?;

            let trust_anchors = certs.into_iter().map(|cert| {
                let der = rustls_pki_types::CertificateDer::from(cert);
                let anchor = webpki::anchor_from_trusted_cert(&der).unwrap().to_owned();

                rustls_pki_types::TrustAnchor {
                    subject: anchor.subject,
                    subject_public_key_info: anchor.subject_public_key_info,
                    name_constraints: anchor.name_constraints,
                }
            });

            root_store.roots.extend(trust_anchors);
        } else {
            root_store
                .roots
                .extend(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
                    rustls_pki_types::TrustAnchor {
                        subject: ta.subject.into(),
                        subject_public_key_info: ta.spki.into(),
                        name_constraints: ta.name_constraints.map(|f| f.into()),
                    }
                }));
        }
        Ok(root_store)
    }

    fn build_client_certificates<'a>(
        client_cert: &Path,
    ) -> std::io::Result<Vec<CertificateDer<'a>>> {
        let file = File::open(client_cert)?;
        let mut pem = BufReader::new(file);
        let raw_certs = rustls_pemfile::certs(&mut pem)?;

        let certs: Vec<CertificateDer> = raw_certs.into_iter().map(CertificateDer::from).collect();
        Ok(certs)
    }

    fn build_client_private_keys<'a>(
        client_private_key: &Path,
    ) -> std::io::Result<Vec<PrivateKeyDer<'a>>> {
        let mut pem = BufReader::new(File::open(client_private_key)?);
        let keys = Self::read_private_keys_from_pem(&mut pem)?;
        let keys = keys
            .into_iter()
            .map(|c| {
                PrivateKeyDer::try_from(c)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            })
            .collect::<std::io::Result<Vec<PrivateKeyDer>>>()?;

        Ok(keys)
    }
    /// Parses PEM encoded private keys.
    ///
    /// The input should PEM encoded private key in RSA, SEC1 Elliptic Curve or PKCS#8 format.
    fn read_private_keys_from_pem(
        rd: &mut dyn std::io::BufRead,
    ) -> Result<Vec<Vec<u8>>, std::io::Error> {
        let mut keys = Vec::new();

        loop {
            match rustls_pemfile::read_one(rd)? {
                None => return Ok(keys),
                Some(rustls_pemfile::Item::RSAKey(key)) => keys.push(key), //PKCS1
                Some(rustls_pemfile::Item::PKCS8Key(key)) => keys.push(key),
                Some(rustls_pemfile::Item::ECKey(key)) => keys.push(key), //SEC1
                _ => {}
            };
        }
    }
}

/// Unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    fn read_key(pem: &str) {
        // Create a Cursor from the string slice
        let cursor = io::Cursor::new(pem);

        // Create a BufReader from the Cursor
        let mut reader = io::BufReader::new(cursor);

        // Read private key from str-slice
        let result = TlsAdaptor::read_private_keys_from_pem(&mut reader);

        // Ensure success
        assert!(result.is_ok());

        // There should be one key
        let result = result.unwrap();
        assert!(result.len() == 1);

        // The key must not be empty!
        let key = result.first().unwrap();
        assert!(key.len() > 0);
    }

    #[test]
    fn read_rsa_key() {
        // RSA private key in PEM format
        let pem = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAq6r5AxFXp8U15ktFL51U4DQelVXtZnD5klyl63MLTZ2Zx6o2
vK1l1cJz7EyEeZ0evQ9OZ+FyNKnD3C2xtmVzg7e4jBh0U9U/fTHGDs7t6Yc2FV9j
UxxvRa3yD4FpMhPC7nxDJ/mcBDHwJl0hZT8GHfOybEpWx+RAomK7QFihJ+W6AiEk
K5pMMAtAvZgJlb0PPYdM5ibzW/KHyr3FqA+ic1y45zpRZa10gZxW84ppzzH7P12H
uhK8Nu4JwD+6EKDN8hGBl1J5leG9eT8oJH+JbiZfZlUULaq+lsMN1x/M8qkZcq9N
qQjVVBCc5E6byq3JshHSIcvZqSBR5dPnOsqIWQIDAQABAoIBAGerMFdh8/R2DJ09
5EozHvVrf5WadNpU/Cmy1g50Br2ptQIRUuGA3x3hwFrZhAeugfBuxNVD8Yc7e5M6
VsQoUtL8YhCuTijZ7BqG48MofV2oZ/umxfKzhI2MGK4okl19uUybRm7Hk4AvIbyk
bK9JSx0bmEhwPJKL9MCR4Z1RWaBywoN6FgFOFIs7gP9v5dygAksgwl+axaiSMc1p
xlxAsZtPZ1m95hyA7My3PfUs7Y1BcVDAExKb1R34J46O0S3tKn4hLe0Uofsm+m5k
prTdoZ6mV3QzUSsPPuGzWb2uP4OkU5XPhoIGZ2ozLW00YhaUn8/XqfF+AFY5oG2A
zUKaZIECgYEA09Oxw0dOxd8GyYr3qxQdpZre/fm0BlBCu06dyQAhTVGuv4seoqR+
SjlQbkwA2Nfxl4wi5ltC8LZnBQw7prNZdzVGhh9egCIv6Qb2a5b2KNnHev3d4RVu
w9rQJo58J5RRplw65A+n0c64rA14HZWTqlFc0i5e17kAGdu4WMO5o8MCgYEAu1IN
6Zt5FiogRIdUNcTYA56V5PCvcb7nb/8whV63WesfD8X/AL9BnySpbWZ+5bPZEP1H
9iE9R82cJd2Pi74t2gdbFSNUIQUBpu0g8RD+2k9iPfVZQH9KJJmq3Q5kzB1HlBWB
TjJkVR2TKZdo99UEBVeRGGCcxl4izdd2txf+Zc8CgYEArMQunvwtrTPPVU3Og1GF
vWrvoURi42uCqOPLkN7wZyCZn2umxZ4mLScHCSk2N7MQTrkZaKXkOVH1hdUoijMF
c8d5eGTAcW2jC8dRlUPzv42L8DpaUpZDr5pDMSAbdnNsi5jccHECSf5GHgNl4v9b
rsE07myDg3LoE3uOmhsf5y0CgYEAm76+8uZjZBP9JdeCZ5ftJp+M5INplNkN1z0k
e66OBD8T5bdAV2OBUV+yK/u8h5HzhtHsziDOU1GImBicX1UK90Qv2cX2rRmb5jEM
wXhbsygFl/1wC5WZn9RBV8qNsVON4Vr/Rg8EBi5EE+Emfn3E8UzzvIXguk4zWe2+
JK7uYe0CgYEA1WlfJslQLSU5WrhV3FuW52fF8u5vXxwY7U2saxpQO0zCkg26kthJ
M/sWZf1EdFyhhKNW2xCU3eTxMP0f3X5BfGP8xn1gRf2a1EOJxIsPvS9zxIkxnLhJ
kZVVcL5RFkNRODXzR1Tn2Txe3HUkx1a+4vLzBRG4xQp2E+grK8PvOxA=
-----END RSA PRIVATE KEY-----"#;

        // Read
        read_key(pem);
    }

    #[test]
    fn read_pkcs8_key() {
        // PKCS8 private key in PEM format
        let pem = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDDWZyfvGeVRWnz
Ttl0nE7J4M52Y1EvZAjx4F1d7XlPZB2wq2kX3jcGiV9UbkmD7DZ+cVf2V1ZPbmbm
mChXvC9R65YhF5Q6XpzGEQWkXEwM4vhEcWmB3ObXQXGEmZ1y/fmPR2lRIX0Xhn7u
0ZhoVsoKBr1QlMAoHODDRfXLTLWv77ryh7D/SYcB9mNZUE4+OBO7ZnA0cgyMtpKN
1VR9JmG00Dks/EGEDVq7q3r1zbfJd8ctjZW9InOR2W69jBZGknG5BoAEV84XzyI4
fAe+xS7tddM5iOL1z7G4n3uHJDN9an+DSB7P0/MbWVrsAGKewAZdpUgPaFRfd5NN
k1fGeWzHAgMBAAECggEBAIbbRn3ThBO5aMQytudt8SyauFRgu4ZWdrbzKMGcTkg4
QjOeVDcN8y0rruUBs2yBpyoCrAfajQtMiOfD+zMTz7nI7DEpCZ4s2djGGytsTUbT
YO38myNKhKGuQPLKoJmHB2eTO0twm9FzDOZceSkDAzUvdHg4tWepdTpIRPmdxayM
hrM4gtDKYRejGdcB3zpqF11vQ5x9L65RJn4quvPBkF/h3B9Yv6hHqTT3pu2SBnD1
MO18zmO1Ai1oEJXpAXhI2Sde3t5CEQXEG+M/0d9D6FE0KYKPmu8QzRgFvROF11lX
B1BKmQ/GrtIykLO1wb6H5S/27XYlnWfd5z4qTxMB2FUCgYEA/qgM4bOvCCUnMfD8
i5KOaG4lZ8BSaxAYgf1mkmb0Jx+RpSdy2T+dfWXsX5fTbti4jW7fwP4Uj6y0CXkg
G8dGb41gVmkxnBZHyn+4LfdiHb5UV9s21mD2rt8+yZKO7fQLq0GLqAgOW5E8hG3J
3WZqQmx0vH+1S3zW3I6V/uPENKUCgYEAzFPCyE3Qiz4bNdWqSPmDhtZMKD07Q8rZ
wmGxY9h5Kz/9YBCBzr0Q7TkTGB8HeirH1sE3rZ+RlTzK+zwIE8d7c9NR8TGIVd7L
4spF8iO0USjOhhnCRFx9Ptq4YmR2cAxYbP/wFq5/mYQh8h5ocnt0FXKmi2elWKoP
EYP3CnZ0PyMCgYB+e1sS3SR5IkgA5SoXBw8fw7tBqtoXmpjsf6o4P5F/UVP/3jFq
iESWr63z8d8L3wD5Jf3u/XNTxnfdIxyL/Fw2jolV5W5gffK0BaBqUnVgU/5e0fyM
JksM5y6OfEPan9R1P5qCLqdektn1soTHhYf1svPmThmcB+pJoQxocBCSpQKBgQCL
Ye+BcIZ2uWlS9wGJhJmfQY8EThN6+y9ZT/xkO8ioQmPdzpvzP+Tx62n4Vz0bAcJg
/2RsZ7RtY0AmJce9Xw9FGY54I62ZVtuygjoA0kRE/X8gjCytPbQ+y8o+RZb0V5T9
HTNZYLU05zU2rU9I3y3X7QpAD1oP1qW4OvdCnhEeqQKBgQC/XBhBEOyTylZihKfH
sChG+K2gxP7KlRX3a+fsDW44VNCa9d8bbajLxF5pUOxt6NhpAWG1cMlW8pjhpr50
/OJziayF3PIScEdjohoL6Tx0FWY6NejCv+TbzBEdg7Z2MfNk0d2QFvXJS3cvfI3v
xuTt1S1eXyoycQMQbn0UBkgOFg==
-----END PRIVATE KEY-----"#;

        // Read
        read_key(pem);
    }

    #[test]
    fn read_ec_key() {
        // PKCS8 private key in PEM format
        let pem = r#"-----BEGIN EC PRIVATE KEY-----
MHcCAQEEIP8t6gTOOqVp6yZklyWV6R2AVT3E7R8Tk1xzJxw8aU/qoAoGCCqGSM49
AwEHoUQDQgAEJXvHve3eHzqEUPHibPeRLVBlqA2cN1tR7dj3IdKj17lxxfKmT+LP
e+VeXslTPB7gThTnpXpeO0PtYln+yBKLv6G+GA==
-----END EC PRIVATE KEY-----"#;

        // Read
        read_key(pem);
    }

    #[test]
    fn read_invalid_key() {
        let pem = r#"-----BEGIN CERTIFICATE-----
MIIDXTCCAkWgAwIBAgIJALflmDNShp+sMA0GCSqGSIb3DQEBCwUAMEUxCzAJBgNV
BAYTAlVTMQswCQYDVQQIDAJDQTESMBAGA1UEBwwJUGFsbyBBbHRvMQ4wDAYDVQQK
DAVNeUNvMB4XDTIxMDEwMTAwMDAwMFoXDTIyMDEwMTAwMDAwMFowRTELMAkGA1UE
BhMCVVMxCzAJBgNVBAgMAkNBMRIwEAYDVQQHDAlQYWxvIEFsdG8xDjAMBgNVBAoM
BU15Q28wggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDDoqXvYMR2rIcM
+dpL8oyTcKEnGJ8oM2L+gG9B2DxvUyyfOvCb+MB4YEqbblso9d+U6P4bmsW+Fs6X
JQJ8+AKn9GyMSPlByoMkXwGZEtAODMS+JWzVbm6hpEyzg+Kuc1Ej2LZq93z72yC9
kSHzV8Jx3CjZ7tbjXpuZMV8O/tFr9uXJpKiL8zKw/yMGg0N3EHEjtT8E8fT5sCl4
ON+R4HB3/TlwjbNBuNYQ+ZVflZoqpKT8mc5lsW5uPY7ysFffPfogV2Xgu3PaYMuD
uFiAlL17ER+izYYRVHpG3mkhEXN94jOUoqP6tJCEtP+Yr9SGeGV1YBh06QDD2I/p
2f3TYeB7AgMBAAGjUDBOMB0GA1UdDgQWBBSLzwcTk9MV2QyPQtJfH4+wsP0JvDAf
BgNVHSMEGDAWgBSLzwcTk9MV2QyPQtJfH4+wsP0JvDAMBgNVHRMEBTADAQH/MA0G
CSqGSIb3DQEBCwUAA4IBAQBniUIk6X9BlvPLG6L/cAv0vChcHpUBz33B9qPoO8Mk
7wLfrnPPCepdp5VxA5By4ZB0/j+CvzV+XAEG4UgQt2J0P3+j8MIRK27/1E3lHNhF
uP7R7LlFu7zp+O2UBfFZJ8I5HD/u4UgIrzHJreNTU1p6zht2g8POTd18b8AxhA7J
aJMR/6O5XmnFxE5tbZm5vkmqv1JAX33mF2iOLswexHfxZc6T2JQ2wL5a/jG38Qus
AOTNLBRxU+1mW4Kx+V7n48aU6fVwZ2Pxk9Qn5UOr6c1RzRl5hlvcB+X/G8cUS06d
rfQThyKXoXkboRGIzmbUfn7Ba1zRRu3OX0D5FY2iTboS
-----END CERTIFICATE-----"#;

        // Create a Cursor from the string slice
        let cursor = io::Cursor::new(pem);

        // Create a BufReader from the Cursor
        let mut reader = io::BufReader::new(cursor);

        // Read private key from str-slice
        let result = TlsAdaptor::read_private_keys_from_pem(&mut reader);

        // Ensure success
        assert!(result.is_ok());

        // There shouldn't be any key
        let result = result.unwrap();
        assert!(result.len() == 0);
    }
}
