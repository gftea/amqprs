#!/bin/bash

CARGO_OPTS="-p benchmarks --quiet"

# build "bench" profile first, might allow cooldown of system before test begins
cargo bench $CARGO_OPTS --no-run 
amqprs_exe=$(cargo bench --no-run 2>&1 | egrep "Executable.+/native_pub_amqprs.rs" | sed -E 's/.+\((.+)\)/\1/')
lapin_exe=$(cargo bench --no-run 2>&1 | egrep "Executable.+/native_pub_lapin.rs" | sed -E 's/.+\((.+)\)/\1/')
echo $amqprs_exe $lapin_exe

# native exe
sleep 3
$amqprs_exe
sleep 3
$lapin_exe

# run strace's profiling
strace -c $amqprs_exe
strace -c $lapin_exe

# run perf's profiling
sudo perf stat -d $amqprs_exe
sudo perf stat -d $lapin_exe

sudo perf record -o perf-amqprs.data $amqprs_exe
sudo perf report -i perf-amqprs.data

sudo perf record -o perf-lapin.data $lapin_exe
sudo perf report -i perf-lapin.data
