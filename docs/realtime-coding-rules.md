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

## Generated debug thread validation

Debug generated runtimes query the optional native `CLAP_EXT_THREAD_CHECK` host service during
`clap_plugin_t::init()` and cache only the borrowed host-owned extension pointer. When the service
is available, main-thread lifecycle callbacks and audio-thread processing callbacks fail closed if
the host reports the wrong thread. Host extension lookup and checker calls are contained at the C
ABI boundary so a throwing host callback cannot escape into CLAP.

The thread-check include, cached pointer, host lookup, and checker calls are guarded by
`#ifndef NDEBUG`. Release generated code therefore performs no thread-check host call and carries
no thread-check member or branch on the realtime path.
