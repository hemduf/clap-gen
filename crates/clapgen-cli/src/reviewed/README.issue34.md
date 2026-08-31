# Issue #34 codegen boundary correction

This branch fixes the canonical IR boundary required by issue #7. It is intentionally developed TDD-first: contracts for typed IR access, transitive dependency closure, parser-span provenance, and pinned official extension validation are committed before implementation.

Canonical IR v1 serialized bytes must remain unchanged.
