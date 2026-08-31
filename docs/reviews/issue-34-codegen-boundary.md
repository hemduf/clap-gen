# Issue #34 codegen boundary correction

This change fixes the canonical IR boundary required by issue #7. It was developed TDD-first: contracts for typed IR access, transitive dependency closure, parser-span provenance, and pinned official extension validation were committed before implementation.

Canonical IR v1 serialized bytes remain unchanged.
