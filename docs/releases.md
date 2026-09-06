# Releases and versioning

`clap-gen` follows Semantic Versioning and uses the workspace version in the root `Cargo.toml` as the canonical project version.

## Version policy

Before `1.0.0`, the project uses the following compatibility policy:

- PATCH (`0.x.y`): compatible bug fixes and internal changes;
- MINOR (`0.x.0`): new public features or intentionally incompatible changes;
- prereleases: `-alpha.N`, `-beta.N`, and `-rc.N` as needed.

Starting with `1.0.0`:

- PATCH: backward-compatible fixes;
- MINOR: backward-compatible public features;
- MAJOR: incompatible changes to a supported public contract.

Public contracts include the CLI, KDL metadata format and semantics, generated C++ API, public C++ runtime API, and public CMake integration.

## Canonical version

Change only `[workspace.package].version` in the root `Cargo.toml`. The Rust CLI inherits this value through `version.workspace = true`. CMake reads the same value at configure time and injects it into the C++ runtime.

When the Cargo package version changes, regenerate and commit `Cargo.lock` as part of the same release PR:

```sh
cargo check --workspace
```

Validate the version locally with:

```sh
python3 tools/release_version.py
python3 tools/release_version.py --tag v0.2.0
```

## Publishing a release

1. Merge a release PR that updates `Cargo.toml` and `Cargo.lock`.
2. Ensure the release commit is on `main` and CI is green.
3. Create and push an exact `v<version>` tag from that commit.

Example:

```sh
git switch main
git pull --ff-only
git tag -a v0.2.0 -m "clap-gen v0.2.0"
git push origin v0.2.0
```

The `Release` workflow then:

- rejects a tag that does not exactly match `Cargo.toml`;
- rejects a release commit that is not reachable from `main`;
- runs release-policy and Rust tests;
- builds `clapgen` for Linux x86-64, macOS x86-64, macOS arm64, and Windows x86-64;
- packages the binaries with the README and license;
- generates `SHA256SUMS`;
- creates the GitHub Release with generated release notes;
- marks SemVer prerelease tags as GitHub prereleases.

Do not move or reuse a published release tag. Publish a new PATCH version instead.
