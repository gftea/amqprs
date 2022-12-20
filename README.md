[![integration-test](https://github.com/gftea/amqprs/actions/workflows/rust.yml/badge.svg)](https://github.com/gftea/amqprs/actions/workflows/rust.yml)
[![codecov](https://github.com/gftea/amqprs/actions/workflows/codecov.yml/badge.svg)](https://github.com/gftea/amqprs/actions/workflows/codecov.yml)

# amqprs
## [Documentation](https://docs.rs/amqprs/latest/amqprs/)

Yet another RabbitMQ client implementation in rust with different design goals.

## Design philosophy

1. API first: easy to use and understand. Keep the API similar as python client library so that it is easier for users to move from there.
2. Minimum external dependencies: as few external crates as possible.
3. lock free: no mutex/lock in client library itself.


## Design Architecture
<img src="amqprs/amqp-chosen_design.drawio.png" />


## [Readme](amqprs/README.md)

## [Example - Publish and Subscribe](amqprs/examples/basic_pub_sub.rs) 
## [Example - SSL/TLS](amqprs/examples/tls.rs) 

## Run test locally

__Testing depends on RabbitMQ docker container.__

```bash
docker-compose up -d
cargo test --all-features 
```