# #56 — Final core runtime review

This review is the independent completion gate for parent #8. It covers the complete generated CLAP
runtime accumulated by #45–#55, not only the #56 correction.

## Baseline

- CLAP SDK: pinned official `free-audio/clap` commit
  `a47f6badb49d948fd009998f28309cdab78979c9` (CLAP 1.2.10).
- Qualification baseline before this review: `main` at #55 merge
  `05102909b6d433cbe6f748cb5766f5d85a2f028e`.
- #55 main qualification run: GitHub Actions run `33989377692` (run #420). It exercises Rust,
  Linux/macOS/Windows Debug and Release C++, clap-validator, and main-branch ASan/UBSan jobs.

## Native CLAP boundary

The generated public processor contract uses standard C++ scalars and native CLAP types directly:
`clap_process_t`, `clap_process_status`, `clap_id`, and native extension structs. No public
`ProcessBlock`, event, ID, render-mode, or other CLAP ABI mirror type is emitted.

`clap_entry`, the plugin factory and their callbacks have exact native function-pointer types and
external linkage. #54 compiles these callbacks directly against the pinned official headers with
warnings as errors on supported toolchains and rejects unsafe callback casts.

Plugin descriptors and generated public IDs live in immutable static storage. Descriptor ordering,
feature strings and generated IDs are deterministic.

## Ownership and lifecycle

`PluginInstance<Processor>` owns the processor object and native `clap_plugin_t` table. The host
pointer, descriptor pointers, host extension pointers and callback-scoped process/event/buffer
pointers remain borrowed. Callback-scoped pointers are not retained after callback return.

The generated state machine is explicit:

`Created -> Initialized -> Active -> Processing`

with `InitFailed` as a terminal initialization-failure state. Invalid transitions fail closed.
Partial initialization, repeated teardown, processor exceptions and multiple simultaneous instances
have dedicated regression coverage. Processor exceptions are contained at every generated C ABI
boundary; `process()` maps failures to `CLAP_PROCESS_ERROR`.

## Extension dispatch

Only metadata-declared extension tables are generated. Unsupported or unknown extension IDs return
`nullptr`. Dispatch walks bounded generated static data and performs no allocation. Returned tables
and descriptor storage have static lifetime.

Review finding #56-F1: the historical #51 smoke called `get_extension()` before `plugin->init()`.
CLAP 1.2.10 explicitly forbids that host call order. A regression contract was added first, then the
smoke was corrected to initialize the plugin before querying extensions. This was a test-conformance
defect; no production runtime change was required.

## Realtime and threading

Generated audio-thread callbacks are bounded and contain no generated heap allocation, lock,
condition variable, blocking I/O, synchronous logging, filesystem access, retry loop or unbounded
container traversal. #53 includes an allocation-counted host-like runtime smoke and source-shape
regressions for the generated realtime callback region.

Debug builds optionally cache the native `CLAP_EXT_THREAD_CHECK` host service during `init()` and
fail closed on invalid main/audio-thread calls. The thread-check include, pointer, lookup and runtime
branches are compiled out under `NDEBUG`, keeping the Release realtime path minimal. Host thread
checker calls are exception-contained.

## Determinism and ABI qualification

#54 enforces byte-identical generation for identical canonical IR, fixed output ordering, no source
checkout-root leakage, no timestamps/random identifiers in generated C++, and direct native CLAP ABI
compilation without unsafe callback adaptation.

#55 builds a real generated CLAP module through the production entry/factory/lifecycle runtime and
runs a SHA256-pinned official clap-validator release in every Linux/macOS/Windows Debug/Release C++
matrix cell. Main-branch sanitizer jobs build generated runtime fixtures with ASan/UBSan enabled.
Local reproduction commands are documented in `docs/validation.md`.

## Review result

After correction of #56-F1, no BLOCKER, HIGH or MEDIUM correctness, ABI, lifecycle, ownership,
realtime, determinism, validator or maintainability finding remains in the reviewed #8 boundary.
The private `create_plugin_instance()` binding seam remains intentionally internal: concrete
consumer/CMake integration is the responsibility of dependent issue #9 and does not introduce a
public CLAP mirror ABI.

Final merge of #56 remains gated on a green focused PR matrix and the completed green #55
main-branch platform/sanitizer qualification. Parent #8 may be closed only after those gates pass.
