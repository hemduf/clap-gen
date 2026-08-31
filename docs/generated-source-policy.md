# Generated source ownership

Metadata and handwritten processor/DSP sources are **user-owned**. `clapgen generate` must never
rewrite or delete them.

All generated C++, depfiles, resource tables, manifests, and wrapper configuration belong in the
**build tree**. They are disposable build products and must not be committed. Generation uses
fixed output names, atomic replacement, and unchanged timestamps when output bytes do not change.

The only source-tree write operations permitted to future initialization or migration commands
must be explicit, reviewable, and limited to metadata or registry files named by the user.
