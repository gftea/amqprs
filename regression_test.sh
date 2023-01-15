#!/bin/bash
#// regression test before release

# all examples
cargo run --release --example 2>&1 | grep -E '^ ' | grep -v basic_consumer | xargs -n1 cargo run --release --all-features --example

# Test all features combinations
cargo test 
features=("tls" "traces" "compliance_assert" "urispec")
for (( i=1; i < 2**${#features[@]}; i++ )); do
    combo=""
    for (( j=0; j < ${#features[@]}; j++ )); do
        if (( (i & 2**j) > 0 )); then
            combo="$combo -F ${features[j]}"
        fi
    done
    # Test combination
    cargo test $combo
done



# clippy, warnings not allowed
cargo clippy --all-features -- -Dwarnings

# docs build
cargo doc -p amqprs --all-features --open

# cargo msrv
cargo msrv

# dry-run publish
cargo publish -p amqprs --all-features --dry-run