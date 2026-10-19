# pathgen

- [`pathgen-core`](./pathgen-core) — https://crates.io/crates/pathgen-core
- [`pathgen-macros`](./pathgen-macros) — https://crates.io/crates/pathgen-macros
- [`pathgen-io`](./pathgen-io) — https://crates.io/crates/pathgen-io
- [`pathgen`](./pathgen) — https://crates.io/crates/pathgen

## Releasing

Bump `version` in Cargo.toml, add a dated `## <version> (YYYY-MM-DD)` entry to CHANGELOG.md, and push to
`main`. CI (.github/workflows/ci.yml) runs the tests and clippy, then publishes to crates.io each crate whose
version is not there yet, in dependency order.
