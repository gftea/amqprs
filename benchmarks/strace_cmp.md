# Benchmarking client_amqprs::amqprs_basic_pub: Complete (Analysis Disabled)
`strace -c target/release/deps/basic_pub-3b2b0ff1b8c7dee5 --bench --profile-time 10 amqprs`

|% time |    seconds | usecs/call |    calls |   errors |syscall
|------ |----------- |----------- |--------- |--------- |----------------
| 69.84 |   0.803361 |         57 |    13972 |      603 |futex
| 29.62 |   0.340743 |         46 |     7255 |          |write
|  0.18 |   0.002044 |         51 |       40 |          |clone
|  0.08 |   0.000968 |         30 |       32 |          |sendto
|  0.06 |   0.000646 |         26 |       24 |          |epoll_ctl
|  0.05 |   0.000528 |         33 |       16 |          |fcntl
|  0.04 |   0.000480 |         20 |       24 |          |recvfrom
|  0.03 |   0.000398 |         49 |        8 |        8 |connect
|  0.02 |   0.000252 |         28 |        9 |          |sched_getaffinity
|  0.02 |   0.000228 |         28 |        8 |          |eventfd2
|  0.02 |   0.000201 |         25 |        8 |          |epoll_create1
|  0.02 |   0.000187 |         23 |        8 |          |socket
|  0.01 |   0.000124 |         15 |        8 |          |getsockopt
|  0.01 |   0.000107 |          9 |       11 |          |brk
|  0.00 |   0.000039 |         13 |        3 |          |ioctl
|  0.00 |   0.000024 |         12 |        2 |          |munmap
|  0.00 |   0.000018 |          6 |        3 |          |sigaltstack
|  0.00 |   0.000000 |          0 |       15 |          |read
|  0.00 |   0.000000 |          0 |       10 |          |close
|  0.00 |   0.000000 |          0 |        7 |          |fstat
|  0.00 |   0.000000 |          0 |        1 |          |poll
|  0.00 |   0.000000 |          0 |        1 |          |lseek
|  0.00 |   0.000000 |          0 |       32 |          |mmap
|  0.00 |   0.000000 |          0 |       13 |          |mprotect
|  0.00 |   0.000000 |          0 |        7 |          |rt_sigaction
|  0.00 |   0.000000 |          0 |        1 |          |rt_sigprocmask
|  0.00 |   0.000000 |          0 |        8 |          |pread64
|  0.00 |   0.000000 |          0 |        1 |        1 |access
|  0.00 |   0.000000 |          0 |        1 |          |socketpair
|  0.00 |   0.000000 |          0 |        1 |          |execve
|  0.00 |   0.000000 |          0 |        2 |        1 |arch_prctl
|  0.00 |   0.000000 |          0 |        1 |          |set_tid_address
|  0.00 |   0.000000 |          0 |       10 |          |openat
|  0.00 |   0.000000 |          0 |        1 |          |set_robust_list
|  0.00 |   0.000000 |          0 |        2 |          |prlimit64
|  0.00 |   0.000000 |          0 |        1 |          |getrandom
|  0.00 |   0.000000 |          0 |        2 |        1 |statx
|------ |----------- |----------- |--------- |--------- |----------------
|100.00 |   1.150348 |            |    21548 |      614 |total

# Benchmarking client_lapin::lapin_basic_pub: Complete (Analysis Disabled)
`strace -c target/release/deps/basic_pub-3b2b0ff1b8c7dee5 --bench --profile-time 10 lapin`

|% time |    seconds | usecs/call |    calls |   errors |syscall
|------ |----------- |----------- |--------- |--------- |----------------
| 99.83 |   2.516843 |         38 |    65505 |     2582 |futex
|  0.07 |   0.001791 |         39 |       45 |          |clone
|  0.03 |   0.000667 |         24 |       27 |          |epoll_ctl
|  0.02 |   0.000463 |         25 |       18 |          |fcntl
|  0.02 |   0.000388 |         24 |       16 |          |write
|  0.01 |   0.000311 |         31 |       10 |          |sched_getaffinity
|  0.01 |   0.000282 |         31 |        9 |          |epoll_create1
|  0.01 |   0.000186 |         20 |        9 |          |eventfd2
|  0.01 |   0.000156 |         10 |       15 |          |brk
|  0.00 |   0.000112 |         22 |        5 |          |sched_yield
|  0.00 |   0.000000 |          0 |       15 |          |read
|  0.00 |   0.000000 |          0 |       10 |          |close
|  0.00 |   0.000000 |          0 |        7 |          |fstat
|  0.00 |   0.000000 |          0 |        1 |          |poll
|  0.00 |   0.000000 |          0 |        1 |          |lseek
|  0.00 |   0.000000 |          0 |       34 |          |mmap
|  0.00 |   0.000000 |          0 |       13 |          |mprotect
|  0.00 |   0.000000 |          0 |        2 |          |munmap
|  0.00 |   0.000000 |          0 |        7 |          |rt_sigaction
|  0.00 |   0.000000 |          0 |        1 |          |rt_sigprocmask
|  0.00 |   0.000000 |          0 |        3 |          |ioctl
|  0.00 |   0.000000 |          0 |        8 |          |pread64
|  0.00 |   0.000000 |          0 |        1 |        1 |access
|  0.00 |   0.000000 |          0 |        1 |          |socketpair
|  0.00 |   0.000000 |          0 |        1 |          |execve
|  0.00 |   0.000000 |          0 |        3 |          |sigaltstack
|  0.00 |   0.000000 |          0 |        2 |        1 |arch_prctl
|  0.00 |   0.000000 |          0 |        1 |          |set_tid_address
|  0.00 |   0.000000 |          0 |       10 |          |openat
|  0.00 |   0.000000 |          0 |        1 |          |set_robust_list
|  0.00 |   0.000000 |          0 |        2 |          |prlimit64
|  0.00 |   0.000000 |          0 |        1 |          |getrandom
|  0.00 |   0.000000 |          0 |        2 |        1 |statx
|------ |----------- |----------- |--------- |--------- |----------------
|100.00 |   2.521199 |            |    65786 |     2585 |total