#!/bin/bash

set -x
export RUSTFLAGS="$RUSTFLAGS -A dead_code -A unused_variables"
CARGO_OPTS="-p benchmarks --quiet"

# check environments
uname -a
lsb_release -a
rustc -V
cargo -V
cc -v # gcc linker for libc
lscpu

# check dependencies
cargo tree -i tokio -e all
cargo tree -i amqprs -e all
cargo tree -i lapin -e all


# run separately, otherwise there is runtime conflict
sleep 3 # time for idle
cargo bench ${CARGO_OPTS} -- lapin

sleep 3 # time for idle
cargo bench ${CARGO_OPTS} -- amqprs

############################################################
#  benchmark results
############################################################
readme="
See './local_wsl2.log' for benchmarks result from test run on WSL2 of my local PC.
It shows 'amqprs' has better performance than 'lapin'.

But, benchmark result from default GitHub-hosted runner machine shows 'lapin' has 
better performance than 'amqprs'.

Have not found out reasonable explanations.
"

echo $readme
