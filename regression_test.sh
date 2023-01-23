#!/bin/bash
#// regression test before release
set -x

function check_result () {
    ret=$?
    if [[ $ret != 0 ]];then
        echo "early exit due to error!"
        exit $ret
    fi
}

# all examples
./examples/run_examples.sh
check_result

# features combination
cargo test
check_result

cargo test -F traces
check_result

cargo test -F compliance_assert
check_result

cargo test -F tls
check_result

cargo test -F urispec
check_result

cargo test --all-features
check_result

# clippy, warnings not allowed
cargo clippy --all-features -- -Dwarnings
check_result

# docs build
cargo doc -p amqprs --all-features --open
check_result

# cargo msrv
cargo msrv
check_result

# dry-run publish
cargo publish -p amqprs --all-features --dry-run
check_result

