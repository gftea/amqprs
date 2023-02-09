#!/bin/bash
# readme="\
# See ./local_wsl2.log for benchmarks result from test run on WSL2 of my local PC.\
# It shows amqprs has better performance than lapin.\
# \
# But, benchmark result from default GitHub-hosted runner machine shows lapin has \
# better performance than amqprs.\
# \
# There are various factors to investigate \
#     1. CPU and Memory \
#     2. underlying system libraries \
#     3. Network IO \
#     4. Environment setup\
# \
# "

# echo $readme
# set -x
export RUSTFLAGS="$RUSTFLAGS -A dead_code -A unused_variables"
CARGO_OPTS="-p benchmarks"

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

# build "bench" profile first, might allow cooldown of system before test begins
cargo bench --no-run
profile_exe=$(cargo bench --no-run 2>&1 | egrep "Executable.+basic_pub.rs" | sed -E 's/.+basic_pub.+\((.+)\)/\1/')
echo $profile_exe
sleep 3

# run separately, otherwise there is runtime conflict/error
sleep 3
taskset -c 1 cargo bench ${CARGO_OPTS} --bench basic_pub -- --verbose amqprs
sleep 3
taskset -c 1 cargo bench ${CARGO_OPTS} --bench basic_pub -- --verbose lapin

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
