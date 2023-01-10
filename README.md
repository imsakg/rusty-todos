# rusty-todos

A Rust todo project with two independent workflows:

- **Local CLI**: validated CRUD, categories, priorities, due dates, reminders,
  filtering, sorting, pagination, statistics, JSON backups and CSV exchange.
- **Original web demo**: Warp + PostgreSQL + the TypeScript TodoMVC frontend.

The CLI needs no database or network service and does not synchronize with the
legacy web demo. Dates in the reconstructed feature history are user-assigned;
see [development notes](docs/development-plan.md#history-provenance).

## Install and start

Use a current stable Rust toolchain. From the repository root:

```sh
cargo install --path backend --bin rusty-todos --locked
rusty-todos --data "$HOME/.local/share/rusty-todos/todos.json" add "Read the Rust book"
```

Use `--data PATH` on any command, or set `RUSTY_TODOS_FILE`. The default is
`rusty-todos.json` in the current working directory. Choose a stable absolute path
for daily use; every command prints machine-readable JSON unless exporting CSV.

```sh
export RUSTY_TODOS_FILE="$HOME/.local/share/rusty-todos/todos.json"
rusty-todos add "Review the release"
rusty-todos show 1
rusty-todos edit 1 "Review and publish the release"
rusty-todos category assign 1 work
rusty-todos set 1 --priority high --due 2024-06-10
rusty-todos done 1
rusty-todos reopen 1
rusty-todos delete 1
```

Examples assume a fresh store and therefore task ID 1. Additions never reuse a
deleted ID. Titles are trimmed, nonblank and limited to 500 Unicode characters;
categories are case-sensitive, trimmed and limited to 64 printable characters.

## Find the next task

```sh
rusty-todos list --status open --category work --priority high
rusty-todos list --search release --sort title --offset 0 --limit 20
rusty-todos list --sort priority --desc
rusty-todos list --due-before 2024-06-30 --sort due
rusty-todos list --overdue --as-of 2024-06-11
rusty-todos stats --as-of 2024-06-11
```

Filters combine with AND. Search is a Unicode-lowercased title substring. Due-before
is inclusive; overdue means an **open** task due strictly before the chosen date.
Without `--as-of`, overdue and statistics use today's UTC date. No due date means
neither scheduled nor overdue. Sorts break ties by ascending ID, even with `--desc`;
missing due dates stay last. Pagination applies after filtering and sorting.

## Categories and scheduling

```sh
rusty-todos category list
rusty-todos category rename work office
rusty-todos category clear 1
rusty-todos category remove office
rusty-todos set 1 --clear-due
rusty-todos reminder set 1 2024-06-10T21:00:00+03:00
rusty-todos reminder list --now 2024-06-10T18:00:00Z
rusty-todos reminder dismiss 1
rusty-todos reminder clear 1
```

Removing a category keeps its tasks. Renaming into an existing category merges its
membership. Priorities are `low`, `normal` (default), `high`, `urgent`. Due dates use
real `YYYY-MM-DD` calendar dates. Reminder timestamps require an explicit RFC 3339
offset and are normalized to UTC. Reminders are **polled**, not OS notifications:
there is no daemon, background scheduler, sound or email. Completed tasks suppress
reminders; reopening restores an undismissed reminder. Polling does not acknowledge
it; dismissing does, and rescheduling resets acknowledgment.

## Backups and cleanup

```sh
rusty-todos export --output backup.json
rusty-todos import backup.json --mode remap
rusty-todos export --format csv --output tasks.csv
rusty-todos import tasks.csv --format csv
rusty-todos clear-done --yes
```

Export refuses to overwrite a destination. JSON is the complete backup format;
CSV is exchange data and cannot preserve the allocator watermark of deleted IDs.
Import defaults to merge (ID conflicts fail), with `remap` allocating new IDs and
`replace --yes` explicitly replacing the whole store. Export a JSON backup before
replacement or bulk cleanup. See [data formats](docs/data-formats.md).

## Persistence safety

Writes validate the complete state, sync a private temporary file and replace the
store atomically. The CLI holds an OS-backed exclusive lock across load/edit/save.
Contention returns an error; retry the command. The `.lock` sidecar intentionally
remains after exit: **do not delete it while commands are running**. A crashed
process releases its lock automatically. Corrupt stores are never silently reset.
Data and lock paths must be regular files, not symlinks or directories. Store data
in a trusted local directory; advisory locks do not protect against other programs
editing the file, and network filesystems may not honor local locking guarantees.

Exit status: `0` success, `1` operation/storage/validation error, `2` invalid command
syntax. Errors go to stderr, data to stdout. A successful disk write followed by
an output error is still a committed change; inspect the store before retrying.

## Development

```sh
cargo test --manifest-path backend/Cargo.toml --locked
cargo fmt --manifest-path backend/Cargo.toml -- --check
cargo clippy --manifest-path backend/Cargo.toml --all-targets --locked -- -D warnings
cargo check --manifest-path backend/Cargo.toml --features server --bin rusty_todos --locked
```

Default tests are database-free and isolate temporary stores. The original
PostgreSQL tests are opt-in via `postgres-tests`; their initialization resets the
demo schema, so **never run them against valuable data**. See [testing](docs/testing.md).

## Original web demo

The original authentication token is just a user ID, not production authentication.
The demo initializer recreates its schema at startup: use a disposable database.

```sh
# Terminal 1: disposable PostgreSQL, published only on loopback
# Stop any existing process using this port first.
docker run --rm -p 127.0.0.1:5432:5432 -e POSTGRES_PASSWORD=postgres --name rusty-todos-pg postgres:14

# Terminal 2: frontend
cd frontend
npm ci
npm run build

# Terminal 3: backend (database scripts are relative to backend/)
cd backend
cargo run --features server --bin rusty_todos -- ../frontend/web-folder
```

The demo listens on `http://127.0.0.1:8080`. The local CLI's additional metadata is
not exposed by the legacy HTTP API or browser UI.
