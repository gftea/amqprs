#!/bin/bash
# set -x

excludes=("longlive_basic_consumer")
export RUSTFLAGS="$RUSTFLAGS -A dead_code -A unused_variables"

function is_excluded () {
    name=$1
    for x in ${excludes[@]}; do
        if [[ "$x" = "$name" ]];then
            return 1
        fi
    done
    return 0
}

all_examples=$(cargo run --release --example 2>&1 | grep -E '^ ')

for example in ${all_examples[@]}; do
    is_excluded $example
    res=$?
    if [[ $res = 0 ]];then
        cargo run --release --example $example --all-features
    fi
done
