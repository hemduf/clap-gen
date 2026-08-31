# Issue #5 post-merge code review

This branch contains a TDD correction pass for issue #5 after the original implementation was merged.

Regression coverage was added first for:

- stable versioned CLAP extensions from the pinned SDK;
- rejection of unknown draft ABI identifiers;
- input/output port ID overlap allowed by CLAP;
- opposite-direction `in-place-pair` resolution;
- `bypass` and `enum` parameter flag invariants;
- one `main` audio port per direction and index-zero serialization;
- semantic import merging into canonical IR;
- byte-for-byte canonical IR v1 golden serialization.

The first test commits are intentionally red and precede the implementation corrections.
