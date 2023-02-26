# Benchmarking async system

When benchmarking the async system not a single function, to get a stable results,
we run much longer duration in each iteration to reduce deviation.

## Common Configuration for both amqprs and lapin 

- both use tokio runtime
- both use single-threaded runtime
- benchmarking publishing or consuming messages independently

## Interpret Results

- compare wall-time performance
- when both have similar wall-time performance statistics, compare the impact to system using the `strace` and `perf` counters.

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

The result may vary depends on load of running platform. Tuning the message size list per iteration is an important factor to get stable result. 

In general, it seems to have consistent result on the same platform.

Other benefits to use criterion is that it can generate the plots & graphs which is helpful to observe behavior pattern.

But, when we want to do `strace` and `perf` to compare the performance when both clients perform same amount of tasks, it seems to be more reliable to use `run_native.sh`.

# run_native.sh

This is useful to simply compare wall-clock time performance by doing the same amount of tasks once only, but result may vary depends on load of running platform.

It is good to be used for `strace` or `perf` profiling. 

# Publish Benchmark 

## Wall-time Performance

- ### amqprs

![iteration times](report/amqprs-basic-pub/iteration_times.svg)
![PDF](report/amqprs-basic-pub/pdf.svg)

- ### lapin

![iteration times](report/lapin-basic-pub/iteration_times.svg)
![PDF](report/lapin-basic-pub/pdf.svg)

## System Performance/Impact

### `perf` stats

- #### amqprs 

    Performance counter stats for 'target/release/deps/native_pub_amqprs-c3689813d5510222':

                483.68 msec task-clock                #    0.316 CPUs utilized          
                2675      context-switches          #    5.531 K/sec                  
                    8      cpu-migrations            #   16.540 /sec                   
                39834      page-faults               #   82.357 K/sec                  
                                            
        1.532406315 seconds time elapsed

        0.194842000 seconds user
        0.288655000 seconds sys


- #### lapin 

    Performance counter stats for 'target/release/deps/native_pub_lapin-06aebe3a278e95b7':

            1381.09 msec task-clock                #    0.450 CPUs utilized          
                106891      context-switches          #   77.396 K/sec                  
                    68      cpu-migrations            #   49.236 /sec                   
                8864      page-faults               #    6.418 K/sec                  
                                        
        3.067579338 seconds time elapsed

        0.551009000 seconds user
        0.922524000 seconds sys


### `strace` stats

- #### amqprs 

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

- #### lapin

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


# Consume Benchmark 

## Wall-time Performance

- ### amqprs 
<pre>
    Benchmarking amqprs-basic-consume
    Benchmarking amqprs-basic-consume: Warming up for 3.0000 s

    Warning: Unable to complete 100 samples in 5.0s. You may wish to increase target time to 12.4s, or reduce sample count to 40.
    Benchmarking amqprs-basic-consume: Collecting 100 samples in estimated 12.403 s (100 iterations)
    Benchmarking amqprs-basic-consume: Analyzing
    amqprs-basic-consume    time:   [32.134 ms 32.675 ms 33.225 ms]
    mean   [32.134 ms 33.225 ms] std. dev.      [2.5220 ms 3.0618 ms]
    median [31.033 ms 32.765 ms] med. abs. dev. [1.8160 ms 4.0516 ms]
</pre>

- ### lapin
<pre>
    Benchmarking lapin-basic-consume
    Benchmarking lapin-basic-consume: Warming up for 3.0000 s

    Warning: Unable to complete 100 samples in 5.0s. You may wish to increase target time to 12.5s, or reduce sample count to 30.
    Benchmarking lapin-basic-consume: Collecting 100 samples in estimated 12.511 s (100 iterations)
    Benchmarking lapin-basic-consume: Analyzing
    lapin-basic-consume     time:   [31.967 ms 32.598 ms 33.302 ms]
    Found 7 outliers among 100 measurements (7.00%)
    4 (4.00%) high mild
    3 (3.00%) high severe
    mean   [31.967 ms 33.302 ms] std. dev.      [2.3519 ms 4.4001 ms]
    median [31.367 ms 32.906 ms] med. abs. dev. [1.8412 ms 2.9237 ms]
</pre>

## System Performance/Impact

### `perf` stats

- #### amqprs

    Performance counter stats for 'target/release/deps/native_consume_amqprs-3cd5e69f703ee771':

                32.68 msec task-clock                #    0.153 CPUs utilized          
                319      context-switches          #    9.763 K/sec                  
                    6      cpu-migrations            #  183.621 /sec                   
                3034      page-faults               #   92.851 K/sec                  

        0.213868728 seconds time elapsed

        0.014903000 seconds user
        0.019871000 seconds sys

- #### lapin

    Performance counter stats for 'target/release/deps/native_consume_lapin-f6bf48cc2614624a':

                73.32 msec task-clock                #    0.365 CPUs utilized          
                3748      context-switches          #   51.117 K/sec                  
                    8      cpu-migrations            #  109.107 /sec                   
                4502      page-faults               #   61.400 K/sec                  

        0.200998869 seconds time elapsed

        0.016272000 seconds user
        0.059666000 seconds sys

### `strace` stats

- #### amqprs
<pre>

    % time     seconds  usecs/call     calls    errors syscall
    ------ ----------- ----------- --------- --------- ------------------
    71.36    0.009609          10       910           sendto
    23.36    0.003145          17       183           recvfrom
    2.41    0.000325           3        98           brk
    0.97    0.000131           2        47           epoll_wait
    0.64    0.000086          10         8           munmap
    0.53    0.000072           1        37           write
    0.31    0.000042          42         1           futex
    0.19    0.000026           1        24           mmap
    0.11    0.000015          15         1           mremap
    0.07    0.000010           1         8           close
    0.03    0.000004           1         3           sigaltstack
    0.00    0.000000           0         7           read
    0.00    0.000000           0         1           poll
    0.00    0.000000           0         7           mprotect
    0.00    0.000000           0         6           rt_sigaction
    0.00    0.000000           0         3           rt_sigprocmask
    0.00    0.000000           0         4           pread64
    0.00    0.000000           0         1         1 access
    0.00    0.000000           0         1           socket
    0.00    0.000000           0         1         1 connect
    0.00    0.000000           0         1           socketpair
    0.00    0.000000           0         1           getsockopt
    0.00    0.000000           0         1           execve
    0.00    0.000000           0         2           fcntl
    0.00    0.000000           0         2         1 arch_prctl
    0.00    0.000000           0         1           sched_getaffinity
    0.00    0.000000           0         1           set_tid_address
    0.00    0.000000           0         4           epoll_ctl
    0.00    0.000000           0         5           openat
    0.00    0.000000           0         5           newfstatat
    0.00    0.000000           0         1           set_robust_list
    0.00    0.000000           0         1           eventfd2
    0.00    0.000000           0         1           epoll_create1
    0.00    0.000000           0         2           prlimit64
    0.00    0.000000           0         2           getrandom
    0.00    0.000000           0         1           rseq
    0.00    0.000000           0         1           clone3
    ------ ----------- ----------- --------- --------- ------------------
    100.00    0.013465           9      1383         3 total
</pre>

- #### lapin

<pre>
    % time     seconds  usecs/call     calls    errors syscall
    ------ ----------- ----------- --------- --------- ------------------
    86.08    0.031246          28      1104           epoll_wait
    9.35    0.003395           3       958        13 futex
    2.15    0.000781           7       103           brk
    0.42    0.000151           3        39           sched_yield
    0.33    0.000118          39         3           madvise
    0.25    0.000091          18         5           munmap
    0.22    0.000081           3        22           mmap
    0.17    0.000061           7         8           mprotect
    0.16    0.000058           9         6           openat
    0.15    0.000056           7         8           read
    0.13    0.000048          24         2           clone3
    0.08    0.000028           4         6           rt_sigaction
    0.07    0.000025           3         8           close
    0.04    0.000016           5         3           epoll_ctl
    0.04    0.000015          15         1           socketpair
    0.04    0.000014           2         5           rt_sigprocmask
    0.04    0.000013           2         5           newfstatat
    0.03    0.000012           4         3           write
    0.03    0.000011           3         3           sigaltstack
    0.03    0.000010           2         4           pread64
    0.03    0.000010           5         2           getrandom
    0.02    0.000009           4         2           fcntl
    0.02    0.000009           4         2           prlimit64
    0.02    0.000007           7         1           eventfd2
    0.02    0.000007           7         1           epoll_create1
    0.02    0.000006           6         1           poll
    0.01    0.000005           5         1           sched_getaffinity
    0.01    0.000005           5         1           set_robust_list
    0.01    0.000004           2         2         1 arch_prctl
    0.01    0.000004           4         1           set_tid_address
    0.01    0.000004           4         1           rseq
    0.00    0.000000           0         1         1 access
    0.00    0.000000           0         1           execve
    ------ ----------- ----------- --------- --------- ------------------
    100.00    0.036300          15      2313        15 total
</pre>
