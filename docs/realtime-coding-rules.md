# Realtime coding rules

Code reachable from CLAP audio-thread callbacks must obey all of these rules:

- no heap allocation or deallocation;
- no mutex, condition variable, filesystem access, network access, logging, or blocking I/O;
- no exceptions crossing the CLAP C ABI;
- no unbounded loop, recursion, container growth, or unpredictable retry;
- no ownership of host-provided process, event, transport, or buffer pointers after callback return;
- no host callback unless the CLAP extension explicitly permits it on the current thread;
- bounded queues and memory must be allocated before processing starts;
- parameter and note events retain their sample offsets and CLAP targeting semantics.

Every realtime change requires tests for allocation, bounds, lifecycle, event timing, and thread
ownership where applicable. Performance optimizations must retain an oracle or equivalence test.
