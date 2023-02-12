#!/bin/bash

set -x

CARGO_OPTS="-F traces"

# build "bench" profile first, might allow cooldown of system before test begins
BUILD_CMD="cargo bench $CARGO_OPTS --no-run"

amqprs_exe=$(${BUILD_CMD} 2>&1 | egrep "Executable.+/native_consume_amqprs.rs" | sed -E 's/.+\((.+)\)/\1/')
lapin_exe=$(${BUILD_CMD}  2>&1 | egrep "Executable.+/native_consume_lapin.rs" | sed -E 's/.+\((.+)\)/\1/')
echo $amqprs_exe $lapin_exe

# run native exe
sleep 3
$amqprs_exe
sleep 3
$lapin_exe

# run criterion
TARGET="basic_consume_criterion"
BENCH_EXE=$(${BUILD_CMD}  2>&1 | egrep "Executable.+${TARGET}.rs" | sed -E 's/.+\((.+)\)/\1/')
echo $BENCH_EXE

# run separately, otherwise there is runtime conflict/error
ARGS="--bench --verbose --plotting-backend gnuplot"

sleep 3
$BENCH_EXE $ARGS amqprs
sleep 3
$BENCH_EXE $ARGS lapin