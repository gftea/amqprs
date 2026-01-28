# CLAUDE.md - AI Assistant Guide for amqprs

This document provides essential context for AI assistants working with the amqprs codebase.

## Project Overview

**amqprs** is a high-performance, lock-free AMQP 0-9-1 client implementation for RabbitMQ written in Rust. It is listed on the [official RabbitMQ website](https://www.rabbitmq.com/devtools.html#rust-dev) and known for excellent performance among Rust AMQP clients.

### Design Philosophy
1. **API first**: Easy to use, similar to Python's pika library
2. **Minimum dependencies**: As few external crates as possible
3. **Lock-free**: No mutex/lock in the client library itself

## Repository Structure

```
amqprs/
├── amqprs/                    # Main AMQP client library (v2.1.3)
│   ├── src/
│   │   ├── lib.rs             # Public API exports
│   │   ├── api/               # High-level user-facing API
│   │   │   ├── connection.rs  # Connection lifecycle management
│   │   │   ├── channel/       # Channel operations (basic, queue, exchange, tx)
│   │   │   ├── callbacks.rs   # Connection/Channel callback traits
│   │   │   ├── consumer.rs    # Consumer trait definitions
│   │   │   ├── error.rs       # Error types
│   │   │   ├── security.rs    # SASL credentials
│   │   │   └── tls.rs         # TLS support (optional)
│   │   ├── frame/             # AMQP protocol frame layer
│   │   │   ├── method/        # AMQP method frame definitions
│   │   │   └── ...            # Frame encoding/decoding
│   │   └── net/               # Low-level async networking
│   │       ├── reader_handler.rs
│   │       ├── writer_handler.rs
│   │       └── channel_manager.rs
│   └── tests/                 # Integration tests
├── amqp_serde/                # AMQP 0-9-1 types and serialization (v0.4.3)
│   └── src/
│       ├── types.rs           # FieldValue, FieldTable, etc.
│       ├── ser.rs             # Serialization
│       └── de.rs              # Deserialization
├── examples/                  # Example programs
│   └── src/
│       ├── basic_pub_sub.rs   # Core pub/sub workflow
│       ├── tls.rs             # TLS connections
│       ├── mtls.rs            # Mutual TLS
│       └── ...
├── benchmarks/                # Performance benchmarks vs lapin
├── rabbitmq_conf/             # RabbitMQ Docker configuration
├── docker-compose.yml         # RabbitMQ test environment
├── start_rabbitmq.sh          # Sets up RabbitMQ with TLS certs
└── regression_test.sh         # Full test suite runner
```

## Development Commands

### Prerequisites
- Rust 1.71+ (MSRV)
- Docker (for RabbitMQ)

### Building
```bash
# Build with default features
cargo build

# Build with all features
cargo build --all-features

# Build specific crate
cargo build -p amqprs
```

### Testing
Tests require a running RabbitMQ server in Docker:

```bash
# Start RabbitMQ (generates TLS certs, starts container)
./start_rabbitmq.sh

# Run tests with default features
cargo test

# Run tests with specific feature
cargo test -F traces
cargo test -F compliance_assert
cargo test -F tls
cargo test -F urispec

# Run all feature combinations
cargo test --all-features

# Enable trace logging during tests
RUST_LOG=debug cargo test

# Full regression test (runs all feature combinations)
./regression_test.sh
```

### Linting
```bash
# Clippy with warnings as errors (CI requirement)
cargo clippy --all-features -- -Dwarnings
```

### Documentation
```bash
# Generate and open docs
cargo doc -p amqprs --all-features --open
```

### Running Examples
```bash
# Ensure RabbitMQ is running first
./start_rabbitmq.sh

# Run specific example
cargo run -p examples --example basic_pub_sub

# Run TLS example (requires tls feature)
cargo run -p examples --example tls --features example-tls
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `traces` | Enable tracing instrumentation for debugging |
| `compliance_assert` | Enable AMQP spec compliance checks (panics on violations) |
| `tls` | Enable SSL/TLS support via rustls |
| `urispec` | Enable RabbitMQ URI specification support |

## Architecture Overview

### Lock-Free Design Pattern
The library uses async channels for communication instead of locks:

```
Connection::open()
    ├── Spawns ReaderHandler task (reads frames from socket)
    ├── Spawns WriterHandler task (writes frames to socket)
    └── Returns Connection handle

Channel operations use oneshot channels for request/response patterns.
```

### Key Abstractions
- **Connection**: Manages TCP/TLS socket, spawns reader/writer tasks
- **Channel**: Multiplexed logical connection, handles AMQP methods
- **Consumer**: Async trait for receiving delivered messages
- **Callbacks**: Traits for handling connection/channel events

### Module Responsibilities
- `api/`: User-facing types and high-level operations
- `frame/`: AMQP protocol frame definitions and codec
- `net/`: Low-level socket I/O and task management

## Code Conventions

### Error Handling
- Each module has its own `error.rs` with `Result<T>` type alias
- Use `?` operator for error propagation
- Feature-gated compliance assertions instead of runtime checks

### Async Patterns
- All public APIs are async using tokio
- Use `async-trait` macro for async trait methods
- Avoid blocking operations in async contexts

### Builder Pattern
Request arguments use chainable builder pattern:
```rust
let args = QueueDeclareArguments::default()
    .queue("my_queue".into())
    .durable(true)
    .finish();
```

### Testing Patterns
- Integration tests require RabbitMQ Docker container
- Test utilities in `amqprs/tests/common/mod.rs`
- Each test file focuses on specific functionality

## CI/CD Pipeline

The CI runs on push to main and pull requests:

1. **builds**: Debug build + clippy with all features
2. **check_msrv**: Verify compilation on Rust 1.71 and stable
3. **test_examples**: Run example programs
4. **test_features_combination**: Test each feature flag independently

## Common Tasks

### Adding a New AMQP Method
1. Define frame type in `frame/method/`
2. Add high-level API in `api/channel/`
3. Update tests in `amqprs/tests/`

### Modifying Serialization
1. Update types in `amqp_serde/src/types.rs`
2. Adjust ser/de implementations if needed
3. Ensure compatibility with AMQP 0-9-1 spec

### Adding a New Feature Flag
1. Add to `amqprs/Cargo.toml` under `[features]`
2. Use `#[cfg(feature = "...")]` for conditional compilation
3. Update CI matrix in `.github/workflows/regression_test.yml`
4. Document in README.md

## Important Notes

- **No mutex/lock**: Never introduce locks in the library code
- **MSRV 1.71**: Don't use features requiring newer Rust versions
- **Clippy clean**: All warnings must be fixed before merge
- **Feature matrix**: Test with individual features, not just `--all-features`
- **Docker required**: Integration tests need RabbitMQ container running
