#!/bin/bash

export RUSTFLAGS="$RUSTFLAGS -A dead_code -A unused_variables"
CARGO_OPTS="-p benchmarks --quiet"



# build "bench" profile first, might allow cooldown of system before test begins
cargo bench --no-run
profile_exe=$(cargo bench --no-run 2>&1 | egrep "Executable.+basic_pub_criterion.rs" | sed -E 's/.+basic_pub.+\((.+)\)/\1/')
echo $profile_exe
sleep 3

# run separately, otherwise there is runtime conflict/error
sleep 3
taskset -c 1 cargo bench ${CARGO_OPTS} --bench basic_pub_criterion -- --verbose amqprs
sleep 3
taskset -c 1 cargo bench ${CARGO_OPTS} --bench basic_pub_criterion -- --verbose lapin

# run strace profile
strace -c $profile_exe --bench --profile-time 10 amqprs
strace -c $profile_exe --bench --profile-time 10 lapin

# run perf
sudo perf stat -d $profile_exe --bench --profile-time 1 amqprs
sudo perf stat -d $profile_exe --bench --profile-time 1 lapin

sudo perf record -o perf-amqprs.data $profile_exe --bench --profile-time 1 amqprs
sudo perf report -i perf-amqprs.data

sudo perf record -o perf-lapin.data $profile_exe --bench --profile-time 1 lapin
sudo perf report -i perf-lapin.data