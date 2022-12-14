use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use tokio_native_tls::native_tls::Identity;
use tokio_native_tls::native_tls::{Certificate, TlsConnector};

fn main() {
    let ca_cert = "/home/gftea/repo/amqprs/rabbitmq_conf/tls-gen/basic/result/ca_certificate.pem";
    let client_cert = "/home/gftea/repo/amqprs/rabbitmq_conf/tls-gen/basic/result/client_AMQPRS_TEST_certificate.pem";
    let client_key =
        "/home/gftea/repo/amqprs/rabbitmq_conf/tls-gen/basic/result/client_AMQPRS_TEST_key.pem";

    let mut file = File::open(ca_cert).unwrap();
    let mut ca_cert = vec![];
    file.read_to_end(&mut ca_cert).unwrap();

    let mut file = File::open(client_cert).unwrap();
    let mut client_cert = vec![];
    file.read_to_end(&mut client_cert).unwrap();

    let mut file = File::open(client_key).unwrap();
    let mut client_key = vec![];
    file.read_to_end(&mut client_key).unwrap();
    let identity = Identity::from_pkcs8(&client_cert, &client_key).unwrap();

    let connector = TlsConnector::builder()
        .add_root_certificate(Certificate::from_pem(&ca_cert).unwrap())
        .identity(identity)
        .build()
        .unwrap();

    let stream = TcpStream::connect("localhost:5671").unwrap();
    let mut stream = connector.connect("AMQPRS_TEST", stream).unwrap();

    stream.write_all(&[65, 77, 81, 80, 0, 0, 9, 1]).unwrap();
    let mut res = vec![];
    stream.read_to_end(&mut res).unwrap();
    println!("{}", String::from_utf8_lossy(&res));
}
