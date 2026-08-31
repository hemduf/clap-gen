# Dependency policy

All build-critical dependencies are pinned and updated intentionally.

| Dependency | Pin | Source of truth |
| --- | --- | --- |
| Rust | `1.98.0` | `rust-toolchain.toml` |
| kdl-rs | `6.7.1` exact | `crates/clapgen-cli/Cargo.toml` and `Cargo.lock` |
| CLAP SDK | `a47f6badb49d948fd009998f28309cdab78979c9` | `cmake/dependencies/Clap.cmake` |

## Update procedure

1. Open a dependency-only issue and record the upstream release notes or commit range.
2. Update the explicit pin and regenerated lockfile in one pull request.
3. Run `cargo update -p <package> --precise <version>` for Rust dependencies.
4. Run the complete local Rust, CMake, repository-contract, compatibility, and validator suites.
5. Review license, minimum-toolchain, KDL-format, and CLAP ABI changes explicitly.
6. Merge only after the normal independent review and required checks.

Floating branches, wildcard versions, and unreviewed lockfile changes are not allowed.
