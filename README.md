[![ci](https://github.com/gftea/amqprs/actions/workflows/regression_test.yml/badge.svg)](https://github.com/gftea/amqprs/actions/workflows/regression_test.yml)
[![codecov](https://codecov.io/gh/gftea/amqprs/branch/main/graph/badge.svg?token=7MF92R6F60)](https://codecov.io/gh/gftea/amqprs)
[![Documentation](https://docs.rs/amqprs/badge.svg)](https://docs.rs/amqprs)
[![crates.io](https://img.shields.io/crates/v/amqprs.svg)](https://crates.io/crates/amqprs)

# amqprs

Yet another RabbitMQ client implementation in rust with different design goals.

## Design philosophy

1. API first: easy to use and understand. Keep the API similar as python client library so that it is easier for users to move from there.
2. Minimum external dependencies: as few external crates as possible.
3. lock free: no mutex/lock in client library itself.

# Design Architecture
<img src="amqprs/amqp-chosen_design.drawio.png" />

# [README](amqprs/README.md)

## [Example - Publish and Subscribe](examples/src/basic_pub_sub.rs)
## [Example - SSL/TLS](examples/src/tls.rs)

## Run Test Locally

__Testing depends on RabbitMQ docker container.__

```bash
# start rabbitmq server
./start_rabbitmq.sh

# run tests
./regression_test.sh

# enable traces in test.
# Note that it only takes effect if "traces" feature is enabled
RUST_LOG=debug ./regression_test.sh

```
