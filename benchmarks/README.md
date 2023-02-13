# Benchmarking async system

When benchmarking the async system not a single function, to get a stable results,
we run much longer duration in each iteration to reduce deviation.

# run_bench.sh

**It is NOT reliable and NOT recommended to be used.**

It uses criterion's `bencher` API for simple benchmarking, but `lapin` does not work well with it.

When benchmarking `lapin`, a lot of error traces like below are observed.
```bash
2023-02-13T12:42:38.587631Z ERROR lapin::io_loop: error doing IO error=IOError(Custom { kind: Other, error: "IO driver has terminated" })
2023-02-13T12:42:38.587671Z ERROR lapin::channels: Connection error error=IO error: IO driver has terminated
```

**DO NOT USE, just keep it as a record.**

# run_criterion.sh

It uses criterion's API, and both `ampqrs` and `lapin` run OK.

The result seems to vary depends on which platform is run, I observed that on my local PC, `amqprs` always outperform `lapin`, but on github hosted runner, result is opposite with significant differences.

But it seems to have consistent result on the same platform.

Other benefits to use criterion is that it can generate the plots & graphs which is helpful to observe behavior pattern.

But, when we want to do `strace` and `perf` to compare the performance when both clients perform same amount of tasks, it seems to be more reliable to use `run_native.sh`.

# run_native.sh

This is useful to simply compare two clients wall-clock time performance when doing the same amount of tasks. It is good to be used for `strace` or `perf` profiling.


# Benchmark Plots from run_criterion.sh

## amqprs

![iteration times](report/amqprs-basic-pub/iteration_times.svg)
![PDF](report/amqprs-basic-pub/pdf.svg)

## lapin

![iteration times](report/lapin-basic-pub/iteration_times.svg)
![PDF](report/lapin-basic-pub/pdf.svg)

# Wall-clock performance from run_native.sh

## perf statistics

### amqprs benchmarks

 Performance counter stats for 'target/release/deps/native_pub_amqprs-c3689813d5510222':

            483.68 msec task-clock                #    0.316 CPUs utilized          
              2675      context-switches          #    5.531 K/sec                  
                 8      cpu-migrations            #   16.540 /sec                   
             39834      page-faults               #   82.357 K/sec                  
                                         
       1.532406315 seconds time elapsed

       0.194842000 seconds user
       0.288655000 seconds sys


### lapin benchmarks

 Performance counter stats for 'target/release/deps/native_pub_lapin-06aebe3a278e95b7':

           1381.09 msec task-clock                #    0.450 CPUs utilized          
            106891      context-switches          #   77.396 K/sec                  
                68      cpu-migrations            #   49.236 /sec                   
              8864      page-faults               #    6.418 K/sec                  
                                       
       3.067579338 seconds time elapsed

       0.551009000 seconds user
       0.922524000 seconds sys


## strace statistics (syscalls)

### amqprs 
    |% time |    seconds | usecs/call |    calls |   errors |syscall
    |------ |----------- |----------- |--------- |--------- |------------------
    | 97.99 |   0.302695 |         10 |    28701 |          |sendto
    |  0.80 |   0.002467 |          6 |      382 |          |brk
    |  0.56 |   0.001731 |          6 |      272 |          |epoll_wait
    |  0.44 |   0.001351 |          5 |      259 |          |write
    |  0.04 |   0.000139 |         17 |        8 |          |munmap
    |  0.03 |   0.000094 |         47 |        2 |          |futex
    |  0.02 |   0.000075 |          5 |       15 |          |recvfrom
    |  0.02 |   0.000056 |         56 |        1 |        1 |connect
    |  0.01 |   0.000038 |          4 |        8 |          |close
    |  0.01 |   0.000038 |          5 |        7 |          |mprotect
    |  0.01 |   0.000034 |          4 |        7 |          |read
    |  0.01 |   0.000026 |          1 |       24 |          |mmap
    |  0.01 |   0.000022 |          4 |        5 |          |openat
    |  0.01 |   0.000019 |         19 |        1 |          |clone3
    |  0.01 |   0.000018 |         18 |        1 |          |rseq
    |  0.01 |   0.000016 |         16 |        1 |          |mremap
    |  0.01 |   0.000016 |          4 |        4 |          |epoll_ctl
    |  0.00 |   0.000012 |         12 |        1 |          |socketpair
    |  0.00 |   0.000011 |          1 |        6 |          |rt_sigaction
    |  0.00 |   0.000007 |          7 |        1 |          |socket
    |  0.00 |   0.000006 |          2 |        3 |          |sigaltstack
    |  0.00 |   0.000006 |          3 |        2 |          |getrandom
    |  0.00 |   0.000005 |          1 |        3 |          |rt_sigprocmask
    |  0.00 |   0.000005 |          5 |        1 |          |epoll_create1
    |  0.00 |   0.000004 |          4 |        1 |          |poll
    |  0.00 |   0.000004 |          2 |        2 |          |fcntl
    |  0.00 |   0.000004 |          0 |        5 |          |newfstatat
    |  0.00 |   0.000004 |          4 |        1 |          |eventfd2
    |  0.00 |   0.000004 |          2 |        2 |          |prlimit64
    |  0.00 |   0.000003 |          3 |        1 |          |getsockopt
    |  0.00 |   0.000003 |          3 |        1 |          |sched_getaffinity
    |  0.00 |   0.000000 |          0 |        4 |          |pread64
    |  0.00 |   0.000000 |          0 |        1 |        1 |access
    |  0.00 |   0.000000 |          0 |        1 |          |execve
    |  0.00 |   0.000000 |          0 |        2 |        1 |arch_prctl
    |  0.00 |   0.000000 |          0 |        1 |          |set_tid_address
    |  0.00 |   0.000000 |          0 |        1 |          |set_robust_list
    |------ |----------- |----------- |--------- |--------- |------------------
    |100.00 |   0.308913 |         10 |    29738 |        3 |total

### lapin

    |% time |    seconds | usecs/call |    calls |   errors |syscall
    |------ |----------- |----------- |--------- |--------- |------------------
    | 55.28 |   0.281828 |         17 |    16036 |          |epoll_wait
    | 39.30 |   0.200329 |          6 |    28706 |        7 |futex
    |  5.34 |   0.027243 |          8 |     3080 |          |brk
    |  0.07 |   0.000356 |          4 |       88 |          |sched_yield
    |  0.00 |   0.000018 |          4 |        4 |          |munmap
    |  0.00 |   0.000011 |          0 |       23 |          |mmap
    |  0.00 |   0.000003 |          1 |        2 |          |write
    |  0.00 |   0.000000 |          0 |        7 |          |read
    |  0.00 |   0.000000 |          0 |        7 |          |close
    |  0.00 |   0.000000 |          0 |        1 |          |poll
    |  0.00 |   0.000000 |          0 |        8 |          |mprotect
    |  0.00 |   0.000000 |          0 |        6 |          |rt_sigaction
    |  0.00 |   0.000000 |          0 |        5 |          |rt_sigprocmask
    |  0.00 |   0.000000 |          0 |        4 |          |pread64
    |  0.00 |   0.000000 |          0 |        1 |        1 |access
    |  0.00 |   0.000000 |          0 |        1 |          |socketpair
    |  0.00 |   0.000000 |          0 |        1 |          |execve
    |  0.00 |   0.000000 |          0 |        2 |          |fcntl
    |  0.00 |   0.000000 |          0 |        3 |          |sigaltstack
    |  0.00 |   0.000000 |          0 |        2 |        1 |arch_prctl
    |  0.00 |   0.000000 |          0 |        1 |          |sched_getaffinity
    |  0.00 |   0.000000 |          0 |        1 |          |set_tid_address
    |  0.00 |   0.000000 |          0 |        3 |          |epoll_ctl
    |  0.00 |   0.000000 |          0 |        5 |          |openat
    |  0.00 |   0.000000 |          0 |        5 |          |newfstatat
    |  0.00 |   0.000000 |          0 |        1 |          |set_robust_list
    |  0.00 |   0.000000 |          0 |        1 |          |eventfd2
    |  0.00 |   0.000000 |          0 |        1 |          |epoll_create1
    |  0.00 |   0.000000 |          0 |        2 |          |prlimit64
    |  0.00 |   0.000000 |          0 |        2 |          |getrandom
    |  0.00 |   0.000000 |          0 |        1 |          |rseq
    |  0.00 |   0.000000 |          0 |        2 |          |clone3
    |------ |----------- |----------- |--------- |--------- |------------------
    |100.00 |   0.509788 |         10 |    48012 |        9 |total
