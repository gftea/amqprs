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
#  benchmark data (my local machine) rustc 1.67.0
############################################################

# + export 'RUSTFLAGS= -A dead_code -A unused_variables'
# + RUSTFLAGS=' -A dead_code -A unused_variables'
# + CARGO_OPTS='-p benchmarks --quiet'
# + set +x
# Linux E-CND1488614 5.10.102.1-microsoft-standard-WSL2 #1 SMP Wed Mar 2 00:30:59 UTC 2022 x86_64 x86_64 x86_64 GNU/Linux
# No LSB modules are available.
# Distributor ID: Ubuntu
# Description:    Ubuntu 20.04.4 LTS
# Release:        20.04
# Codename:       focal

# running 1 test
# test client_amqprs::amqprs_basic_pub ... bench:  13,012,300 ns/iter (+/- 4,888,245)

# test result: ok. 0 passed; 0 failed; 0 ignored; 1 measured


# running 1 test
# test client_lapin::lapin_basic_pub   ... bench:  16,972,347 ns/iter (+/- 3,939,642)

# test result: ok. 0 passed; 0 failed; 0 ignored; 1 measured


# running 1 test
# test client_amqprs::amqprs_basic_pub ... bench:  13,256,812 ns/iter (+/- 3,947,424)

# test result: ok. 0 passed; 0 failed; 0 ignored; 1 measured 
