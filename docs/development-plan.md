# Local-first todo tools

The existing PostgreSQL/Warp demo stays available. A second binary, `rusty-todos`,
will provide an offline single-user workflow using a separate JSON file. These
stores do not synchronize; the CLI must never initialize or reset PostgreSQL.

## Delivery slices

- Validated CRUD and stable monotonically allocated identifiers.
- Crash-safe persistence and explicit errors instead of silent data replacement.
- Categories, priorities, due dates and pollable reminders.
- Composable filters, deterministic sorting and pagination.
- Versioned JSON backups and CSV exchange with transactional import.
- CLI integration tests, examples and repeatable formatting/lint/test commands.

## Constraints

Keep the legacy HTTP response shapes intact. All local data stays on disk; no
notification daemon or network service is implied. Tests use temporary files and
injected dates, not a shared user store. PostgreSQL tests must remain explicitly
separate because the old fixtures recreate their schema.

## History provenance

This feature history is reconstructed development, not evidence that the features
existed in 2023–2024. New commits have user-assigned historical timestamps. Parent
order expresses implementation dependencies; timestamps deliberately do not.
