#!/bin/bash

set -x
export RUSTFLAGS="$RUSTFLAGS -A dead_code -A unused_variables"
CARGO_OPTS="-p benchmarks --quiet"
set +x

# run separately, otherwise there is runtime conflict
sleep 3 # time for idle
cargo bench ${CARGO_OPTS} -- amqprs
sleep 3 # time for idle
cargo bench ${CARGO_OPTS} -- lapin

# run again, but in reverse order
sleep 3 # time for idle
cargo bench ${CARGO_OPTS} -- lapin
sleep 3 # time for idle
cargo bench ${CARGO_OPTS} -- amqprs