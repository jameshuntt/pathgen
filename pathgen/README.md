# pathgen

`pathgen` is a small Rust path policy kernel.

The goal is simple:

```text
Do not manually write operational paths across the codebase.
Ask for semantic roles, then let policy resolve the concrete path.
```

Instead of this:

```rust
let log = std::path::PathBuf::from("/var/log/archive-api/server.log");
```

Use this:

```rust
use pathgen::{AppPathRole, PathPolicy};

let paths = PathPolicy::prod("archive-api")?;
let log = paths.file(AppPathRole::Logs, "server.log")?;

assert_eq!(log.path.to_string_lossy(), "/var/log/archive-api/server.log");
# Ok::<(), pathgen::PathgenError>(())
```

## What it gives you

- typed standard system paths
- semantic app roles: config, logs, runtime, cache, state, data, certs, secrets
- dev/test/prod separation
- safe joins that reject absolute child paths and `..` traversal
- root remapping for container rootfs/chroot/staging installs
- audit receipts for SOC 2-style evidence
- filesystem bootstrap helpers
- examples for systemd, UDS sockets, logs, receipts, and dry-run manifests

## Features

The default build is `pathgen-core` only: no dependencies. Opt in to more:

- `serde` — `serde` derives on the core types
- `file-json`, `file-toml`, `file-yaml`, `file-xml`, `file-cbor`, `file-txt`,
  `file-jsonl`, `file-markdown`, `file-docx`, `file-pdf`, `file-npy`, `file-npz`
  — file sinks and codecs from `pathgen-io` (`all-file` for every one)
- `sql-sqlite` — an SQLite sink (`all-local` = `all-file` + `sql-sqlite`)
- `derive` — `#[derive(ArtifactContract)]` / `#[derive(ReceiptContract)]`
- `workspace` — opt-in upward walk to a `workspace_locations.toml` root, for
  test fixtures

## Basic usage

```rust
use pathgen::{AppPathRole, PathPolicy};

let paths = PathPolicy::prod("video-archive-api")?;

let config = paths.file(AppPathRole::Config, "app.toml")?;
let log = paths.file(AppPathRole::Logs, "server.log")?;
let socket = paths.socket("api.sock")?;

println!("{}", config.path.display());
println!("{}", log.path.display());
println!("{}", socket.path.display());
# Ok::<(), pathgen::PathgenError>(())
```

## Test/dev/prod isolation

```rust
use pathgen::{AppPathRole, PathPolicy};

let prod = PathPolicy::prod("archive-api")?;
let test = PathPolicy::test("archive-api")?;

assert_eq!(
    prod.dir(AppPathRole::Runtime)?.path.to_string_lossy(),
    "/run/archive-api"
);

assert_eq!(
    test.dir(AppPathRole::Runtime)?.path.to_string_lossy(),
    "/tmp/pathgen-test/archive-api/runtime"
);
# Ok::<(), pathgen::PathgenError>(())
```

## Root remapping

Useful for container rootfs, staging installs, test rootfs trees, or image generation.

```rust
use pathgen::{AppPathRole, PathPolicy};

let paths = PathPolicy::prod("archive-api")?
    .with_root("/srv/rootfs");

let log = paths.file(AppPathRole::Logs, "server.log")?;

assert_eq!(
    log.path.to_string_lossy(),
    "/srv/rootfs/var/log/archive-api/server.log"
);
# Ok::<(), pathgen::PathgenError>(())
```

## Audit receipts

Every resolution can carry policy metadata:

```rust
use pathgen::{AppPathRole, PathPolicy};

let paths = PathPolicy::prod("archive-api")?;
let resolved = paths.file(AppPathRole::Config, "app.toml")?;

println!("{}", resolved.audit_line());
# Ok::<(), pathgen::PathgenError>(())
```

Example output:

```text
policy=pathgen.policy.v1 app=archive-api env=prod role=config path=/etc/archive-api/app.toml sensitivity=confidential mutability=read_only owner=root
```

## Run tests

```bash
cargo test --all-targets
```

## Examples

```bash
cargo run --example basic_usage
cargo run --example systemd_integration
cargo run --example uds_integration
cargo run --example audit_receipts
cargo run --example chroot_rootfs
cargo run --example dry_run_manifest
```
