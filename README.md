# clap-gen

`clap-gen` is a metadata-driven CLAP plugin code generator. KDL 2.0 metadata owns plugin
structure; handwritten C++ owns DSP and application-specific behavior. The processor-facing API
uses native CLAP structures directly and does not introduce ABI mirror wrappers.

## Bootstrap build

Required tools:

- Rust 1.98.0 with `rustfmt` and `clippy`;
- CMake 3.25 or newer;
- Ninja and a C++20 compiler;
- Python 3.10 or newer for repository contract tests.

```sh
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check --deny warnings bans licenses sources

cmake --preset dev
cmake --build --preset dev
ctest --preset dev

python3 -m unittest -v tests/bootstrap/test_repository.py
```

The `release`, `ci-linux`, `ci-macos`, and `ci-windows` presets provide equivalent release
configurations. GitHub Actions wiring is tracked separately in issue #3.

## CLI bootstrap

```sh
cargo run -p clapgen-cli -- --version
cargo run -p clapgen-cli -- doctor
```

Dependency update rules are documented in [DEPENDENCIES.md](DEPENDENCIES.md). Generated-source
ownership and realtime constraints are part of the repository policy and apply to every PR.

## Releases

The root `Cargo.toml` owns the project version. Release tags use `v<semver>` and are published by
GitHub Actions with cross-platform CLI binaries and checksums. See [docs/releases.md](docs/releases.md)
for the versioning policy and release procedure.
