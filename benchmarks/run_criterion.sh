#!/bin/bash

set -x

# check environments
# uname -a
# lsb_release -a
# rustc -V
# cargo -V
# cc -v # gcc linker for libc
# lscpu

# # check dependencies
# cargo tree -i tokio -e all
# cargo tree -i amqprs -e all
# cargo tree -i lapin -e all

CARGO_OPTS="-F traces"
TARGET="basic_pub_criterion"

# build "bench" profile first, might allow cooldown of system before test begins
BUILD_CMD="cargo bench $CARGO_OPTS --no-run"
BENCH_EXE=$(${BUILD_CMD} 2>&1 | egrep "Executable.+${TARGET}.rs" | sed -E 's/.+\((.+)\)/\1/')
echo $BENCH_EXE

# run separately, otherwise there is runtime conflict/error
ARGS="--bench --verbose"
if [[ $GITHUB_ACTIONS == true ]]; then
    ARGS="$ARGS --plotting-backend plotters"
else
    ARGS="$ARGS --plotting-backend gnuplot"
fi

sleep 3
$BENCH_EXE $ARGS amqprs
sleep 3
$BENCH_EXE $ARGS lapin
