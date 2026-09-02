# pathgen-core

The dependency-light core of [`pathgen`](https://crates.io/crates/pathgen): semantic
app path roles (config, logs, runtime, cache, state, data, certs, secrets),
dev/test/prod separation, safe joins that reject absolute children and `..`,
root remapping for chroot and staging installs, audit receipts, artifact
endpoint descriptors and a registry of tagged operations.

No I/O beyond the `fs_ops` bootstrap helpers, no serialization unless the
`serde` feature is on. Most users want the `pathgen` facade crate instead.
