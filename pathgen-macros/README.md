# pathgen-macros

Derive macros for [`pathgen`](https://crates.io/crates/pathgen) artifact
contracts: `#[derive(ArtifactContract)]` and `#[derive(ReceiptContract)]`.
Declaration-only: the macros generate descriptor impls against
`pathgen-core` types and pull in nothing at runtime. Enable through the
facade's `derive` feature.
