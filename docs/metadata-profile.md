# clap-gen canonical KDL metadata profile

clap-gen author metadata uses KDL 2.0 exclusively. The current metadata schema is `1.0.0` and is published at `schemas/clapgen-1.0.0.kdl`.

Every root manifest starts with:

```kdl
clapgen schema="1.0.0"
```

The parser accepts the structural areas `plugin`, `processor`, `parameters`, `audio-ports`, `note-ports`, `state`, `gui`, `presets`, `extensions`, and `import`. Semantic CLAP validation is deliberately deferred to the canonical IR pipeline in issue #5.

## Canonical formatting

`clapgen fmt <file>` parses as KDL 2.0 and applies the kdl-rs canonical formatter. Comments and slashdash-disabled nodes remain author-owned and are preserved. Formatting is idempotent: formatting an already formatted document does not change its bytes.

Use `clapgen fmt --check <file>` in scripts when a non-canonical document should fail instead of being rewritten.

## Imports and dependency tracking

Imports are relative to the file containing them:

```kdl
import "shared/parameters.kdl"
```

`clapgen deps <file>` prints resolved direct import paths. This is the parser-level dependency surface consumed by later generation/CMake integration; semantic import merging belongs to the IR compiler.

## Unknown nodes and properties

The base profile is closed: unknown nodes and properties are rejected with a path-, line-, and node-aware diagnostic.

Vendor metadata is allowed only after explicitly declaring a namespace:

```kdl
extensions {
    namespace "acme"
}

plugin id="com.example.plugin" name="Plugin" vendor="Example" version="1.0.0" acme.mode="fast"
acme.instrument-model enabled=#true
```

A declared namespace may be used with `.` or `:` qualification. An undeclared prefix is rejected.

## KDL 1 and YAML

YAML is not an accepted authoring format. Files with `.yaml`/`.yml` extensions and YAML document markers are rejected with a KDL 2.0 migration hint.

The canonical profile also rejects legacy KDL 1 bare literals (`true`, `false`, `null`) and points to their KDL 2.0 forms (`#true`, `#false`, `#null`). Inputs that are valid in both language versions are interpreted as KDL 2.0; clap-gen never enables the kdl-rs KDL 1 fallback parser.

## Diagnostics

Structural validation errors include:

- manifest path;
- source line when the offending node can be located;
- node name;
- stable error category/message;
- a remediation hint.

Syntax failures are reported as KDL 2.0 parse errors with the original kdl-rs diagnostic text and a formatting/fix hint.

## Boundary with semantic validation

This profile intentionally validates syntax and structural vocabulary only. It does not assign CLAP IDs, normalize flags, resolve semantic cross-references, enforce CLAP capability dependencies, or distinguish stable versus draft extension ABI versions. Those responsibilities start in the canonical IR and semantic validation pipeline (#5).
