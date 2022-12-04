[![integration-test](https://github.com/gftea/amqprs/actions/workflows/rust.yml/badge.svg)](https://github.com/gftea/amqprs/actions/workflows/rust.yml)

# amqprs
## [Documentation](https://docs.rs/amqprs/latest/amqprs/)

Yet another RabbitMQ client implementation in rust with different design goals.

## Design philosophy

1. API first: easy to use, easy to understand. Keep the API similar as python client library so that it is easier for users to move from there.
2. Minimum external dependencies: as less exteranl crates as possible
3. lock free: no mutex/lock in client library itself 

## Design Architecture
<img src="amqprs/amqp-chosen_design.drawio.png" />

## [Example: Consume and Publish](amqprs/examples/basic_pub_sub.rs) 

## [README](amqprs/README.md)