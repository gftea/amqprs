#!/bin/bash

CARGO_OPTS="-F traces"
TARGET="basic_pub_bencher"

# build "bench" profile first, might allow cooldown of system before test begins
BUILD_CMD="cargo bench $CARGO_OPTS --no-run"
BENCH_EXE=$(${BUILD_CMD} 2>&1 | egrep "Executable.+${TARGET}.rs" | sed -E 's/.+\((.+)\)/\1/')
echo $BENCH_EXE

# run separately, otherwise there is runtime conflict/error
ARGS="--bench --verbose --plotting-backend gnuplot"

sleep 3
$BENCH_EXE $ARGS amqprs
sleep 3
$BENCH_EXE $ARGS lapin

