# Testing and development

All commands below start at the repository root unless explicitly changing into
`backend/`. Install a current stable Rust toolchain with rustfmt and Clippy.
The checked-in Cargo.lock is part of the application, not an ignored build file.

## Required offline gates

```sh
cargo fmt --manifest-path backend/Cargo.toml -- --check
cargo clippy --manifest-path backend/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path backend/Cargo.toml --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path backend/Cargo.toml --lib --no-deps --locked
```

`tests/local.rs` covers domain rules, persistence, metadata, reminders, query
ordering/pages, import rollback, malformed inputs, locking, symlinks and the
published fixture. `tests/cli.rs` runs real subprocesses against isolated temporary
stores, including parallel writers, lock contention, corruption, environment
configuration, backups, cleanup confirmation and Unix permissions. Tests inject
calendar dates/reminder instants instead of relying on the wall clock.

Use a dedicated target directory if another checkout is compiling in a shared
Cargo target directory, for example `--target-dir backend/target -j 2`. Never run
integration tests against your daily todo file. Temporary directories are removed
when their guards drop.

## Fixture smoke without installing

Choose a new writable scratch directory and substitute its absolute path for
`/path/to/scratch`; do not use a real store or overwrite an existing backup.

```sh
cargo run --manifest-path backend/Cargo.toml --locked -- --data /path/to/scratch/demo.json import backend/tests/fixtures/version1.json
cargo run --manifest-path backend/Cargo.toml --locked -- --data /path/to/scratch/demo.json list --status open --sort due
cargo run --manifest-path backend/Cargo.toml --locked -- --data /path/to/scratch/demo.json reminder list --now 2024-06-10T18:00:00Z
cargo run --manifest-path backend/Cargo.toml --locked -- --data /path/to/scratch/demo.json export --output /path/to/scratch/backup.json
```

The fixture contains two tasks, one open and one done. The next allocated ID is 5,
showing that JSON backups retain the watermark of previously deleted IDs.

## Legacy server: compile only by default

```sh
cargo check --manifest-path backend/Cargo.toml --features server --bin rusty_todos --locked
cargo test --manifest-path backend/Cargo.toml --features postgres-tests --bin rusty_todos --locked --no-run
```

These commands require no running PostgreSQL. The repository's GitHub Actions
workflow performs the offline gates on Linux/macOS/Windows and compiles the server
and opt-in database tests on Linux. A local green run is not evidence that those
remote jobs have run; no push or hosted CI execution is required for local work.

## Optional destructive database tests

The original tests call the demo initializer, which recreates its schema using
hard-coded localhost PostgreSQL credentials. Only use a dedicated disposable
PostgreSQL 14 instance on loopback port 5432, as described in the README. Verify
that this is not a personal/development database you want to keep.

```sh
cd backend
cargo test --features postgres-tests --bin rusty_todos --locked -- --test-threads=1
```

Do not parallelize these fixtures or run `--all-features` against a valuable local
database. Compilation of this suite is checked separately from executing it; the
new offline feature suite does not exercise or certify legacy HTTP security.

## Known scope limits

- The CLI is single-user local storage, not a server or synchronization engine.
- Reminders are pollable data; a notification daemon is intentionally absent.
- CSV is exchange data, not full allocator-history backup or formula sanitization.
- Advisory locks and rename durability depend on the filesystem. Use local disks
  and a trusted directory, not a hostile shared directory or an assumed-safe NAS.
- The legacy SQLx 0.6 dependency reports a future-incompatibility warning on modern
  Rust. The server compiles today, but upgrading that separate stack remains work.
