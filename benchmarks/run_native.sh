#!/bin/bash

export RUSTFLAGS="$RUSTFLAGS -A dead_code -A unused_variables"
CARGO_OPTS="-p benchmarks --quiet"



# build "bench" profile first, might allow cooldown of system before test begins
cargo bench --no-run 

set -x
amqprs_exe=$(cargo bench --no-run 2>&1 | egrep "Executable.+/native_pub_amqprs.rs" | sed -E 's/.+\((.+)\)/\1/')
lapin_exe=$(cargo bench --no-run 2>&1 | egrep "Executable.+/native_pub_lapin.rs" | sed -E 's/.+\((.+)\)/\1/')
set +x

# native exe
$amqprs_exe
$lapin_exe

# run strace profile
strace -c $amqprs_exe
strace -c $lapin_exe

# run perf
sudo perf stat -d $amqprs_exe
sudo perf stat -d $lapin_exe

sudo perf record -o perf-amqprs.data $amqprs_exe
sudo perf report -i perf-amqprs.data

sudo perf record -o perf-lapin.data $lapin_exe
sudo perf report -i perf-lapin.data
