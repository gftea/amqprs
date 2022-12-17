#!/bin/bash
#// regression test before release

# all features
cargo test --all-features 

# all examples
cargo run --example 2>&1 | grep -E '^ ' | xargs -n1 cargo run --all-features --example

# features combination
cargo test 
cargo test -F tracing
cargo test -F compliance_assert
cargo test -F tls

# docs build
cargo doc -p amqprs --all-features --open

# dry-run publish
cargo publish -p amqprs --all-features --dry-run