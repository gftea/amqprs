#!/bin/bash
#// regression test before release

cargo test 
cargo test -F tracing
cargo test -F compliance_assert
cargo test --all-features 

cargo doc -p amqprs --all-features --open
cargo publish -p amqprs --all-features --dry-run