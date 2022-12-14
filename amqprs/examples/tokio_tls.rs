use std::convert::TryFrom;
use std::fs::File;
use std::io::BufReader;
use std::io::{self, Read};
use std::net::ToSocketAddrs;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::rustls::{self, Certificate, OwnedTrustAnchor, PrivateKey};
use tokio_rustls::{webpki, TlsConnector};
use tokio_native_tls::native_tls;

async fn test_rustls() -> io::Result<()> {
    let current_dir = std::env::current_dir().unwrap();

    let ca_cert = current_dir.join(Path::new(
        "rabbitmq_conf/client/ca_certificate.pem",
    ));

    let client_cert = current_dir.join(Path::new(
        "rabbitmq_conf/client/client_AMQPRS_TEST_certificate.pem",
    ));

    let client_key = current_dir.join(Path::new(
        "rabbitmq_conf/client/client_AMQPRS_TEST_key.pem",
    ));
    
    let addr = ("localhost", 5671)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    let domain = "AMQPRS_TEST";

    let mut root_cert_store = rustls::RootCertStore::empty();
    let mut pem = BufReader::new(File::open(ca_cert)?);
    let certs = rustls_pemfile::certs(&mut pem)?;
    let trust_anchors = certs.iter().map(|cert| {
        let ta = webpki::TrustAnchor::try_from_cert_der(&cert[..]).unwrap();
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    });
    root_cert_store.add_server_trust_anchors(trust_anchors);

    let mut pem = BufReader::new(File::open(client_cert)?);
    let certs = rustls_pemfile::certs(&mut pem)?;
    let certs = certs.into_iter().map(|cert| Certificate(cert));

    let mut pem = BufReader::new(File::open(client_key)?);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut pem)?;
    let mut keys = keys.into_iter().map(|key| PrivateKey(key));

    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_cert_store)
        .with_single_cert(certs.collect(), keys.next().unwrap())
        .unwrap();
    let connector = TlsConnector::from(Arc::new(config));

    let stream = TcpStream::connect(&addr).await?;

    let domain = rustls::ServerName::try_from(domain)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;

    let stream = connector.connect(domain, stream).await?;
    let (mut reader, mut writer) = split(stream);

    writer.write_all(&[65, 77, 81, 80, 0, 0, 9, 1]).await?;

    let mut res = vec![];
    reader.read_to_end(&mut res).await.unwrap();

    println!("{}", String::from_utf8_lossy(&res));

    Ok(())
}

async fn test_nativetls() -> io::Result<()> {
    let current_dir = std::env::current_dir().unwrap();

    let ca_cert = current_dir.join(Path::new(
        "rabbitmq_conf/client/ca_certificate.pem",
    ));

    let client_cert = current_dir.join(Path::new(
        "rabbitmq_conf/client/client_AMQPRS_TEST_certificate.pem",
    ));

    let client_key = current_dir.join(Path::new(
        "rabbitmq_conf/client/client_AMQPRS_TEST_key.pem",
    ));
    
    let addr = ("localhost", 5671)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    let domain = "AMQPRS_TEST";

    let mut file = File::open(ca_cert).unwrap();
    let mut ca_cert = vec![];
    file.read_to_end(&mut ca_cert).unwrap();

    let mut file = File::open(client_cert).unwrap();
    let mut client_cert = vec![];
    file.read_to_end(&mut client_cert).unwrap();

    let mut file = File::open(client_key).unwrap();
    let mut client_key = vec![];
    file.read_to_end(&mut client_key).unwrap();
    let identity = native_tls::Identity::from_pkcs8(&client_cert, &client_key).unwrap();

    let connector = native_tls::TlsConnector::builder()
        .add_root_certificate(native_tls::Certificate::from_pem(&ca_cert).unwrap())
        .identity(identity)
        .build()
        .unwrap();
    let connector = tokio_native_tls::TlsConnector::from(connector);
    let stream = TcpStream::connect(&addr).await.unwrap();
    let stream = connector.connect(domain, stream).await.unwrap();

    let (mut reader, mut writer) = split(stream);

    writer.write_all(&[65, 77, 81, 80, 0, 0, 9, 1]).await?;

    let mut res = vec![];
    reader.read_to_end(&mut res).await.unwrap();

    println!("{}", String::from_utf8_lossy(&res));

    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("test tokio rustls");
    test_rustls().await?;

    println!("test tokio nativetls");
    test_nativetls().await
}
