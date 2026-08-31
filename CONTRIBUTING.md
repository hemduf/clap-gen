# Contributing to clap-gen

## Development workflow

1. Link each branch and pull request to one focused issue.
2. Start with a failing test that expresses the behavior or regression.
3. Implement the smallest change, then refactor while tests stay green.
4. Run the Rust, CMake, CTest, repository-contract, and relevant validator checks locally.
5. Document a code-review pass and resolve blocking findings with regression tests.
6. Use auto-merge only after required checks and independent approval succeed.

Do not force-merge, bypass branch protections, weaken a test to make an implementation pass, or
combine unrelated cleanup with feature work.

## Architecture boundaries

- KDL 2.0 is the only authoring format.
- The canonical IR separates metadata parsing from code generation.
- Processor-facing C++ uses native CLAP structures and scalar types.
- Do not add `ProcessBlock`, `ParamEvent`, ID, render-mode, or other CLAP ABI mirror types.
- Public `clap_id` values are immutable once released.
- Generated outputs belong to the build tree; metadata and DSP remain user-owned.

Read [the realtime coding rules](docs/realtime-coding-rules.md) before modifying runtime code.
