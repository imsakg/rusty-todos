# Data formats and recovery

## JSON: complete local backups

A store and a JSON export use the same UTF-8 object shape:

```json
{
  "schema_version": 1,
  "next_id": 3,
  "tasks": [
    {
      "id": 1,
      "title": "Review release",
      "status": "open",
      "category": "work",
      "priority": "high",
      "due": "2024-06-10",
      "reminder": "2024-06-10T18:00:00Z",
      "reminder_dismissed": false
    }
  ]
}
```

`next_id` preserves the allocation watermark, including deleted tasks. Every ID
must be nonzero, unique and below `next_id`. IDs use unsigned 64-bit integers;
tools using IEEE-754 JSON numbers must not round large IDs. A missing version is
interpreted as version 1 for early local CLI snapshots. Future versions and unknown
fields fail rather than silently discarding data. Early snapshots may omit the
optional metadata: category/due/reminder become null, priority normal and reminder
acknowledgment false. Title and status remain required. Status values are `open`
and `done`, not the legacy HTTP demo's `close` spelling.

Titles must already be trimmed when importing. Categories must already be trimmed
and printable. Due dates are date-only values; reminders are RFC 3339 instants
with offsets, normalized to UTC. No account, owner or server connection is stored.

## CSV: interoperable, but not a complete backup

The exact, ordered header is:

```csv
id,title,status,category,priority,due,reminder,reminder_dismissed
```

All eight columns are required, including in an empty file. Null optional fields
use empty cells; booleans use `true`/`false`. Commas, double quotes, Unicode and
multiline titles use standard CSV quoting. Duplicate IDs, bad enums, malformed
rows, unrecognized/reordered headers and invalid dates fail the whole import.

CSV omits `schema_version` and the allocator watermark. It reconstructs `next_id`
from the largest surviving ID plus one. After deleting the highest IDs, a fresh
CSV restore **can reuse those deleted IDs**. Use JSON whenever ID history matters.

CSV is raw data exchange, not a spreadsheet-sanitization format. A title beginning
with `=`, `+`, `-` or `@` may be interpreted as a formula by a spreadsheet. Import
CSV columns as text and do not enable formulas from untrusted task data. JSON is
the safer choice when sharing data with an unknown consumer.

## Import policy

- `merge` (default): preserve imported IDs; any conflict aborts without changes.
- `remap`: allocate fresh local IDs for every imported task, keeping metadata.
- `replace --yes`: discard the current in-memory snapshot and adopt the imported
  snapshot. Export a JSON backup first. Replacing is intentionally allowed to
  restore an older allocator watermark.

Parsing and validation finish before the candidate becomes the active store.
The CLI writes only after all requested work succeeds. A bad row does not leave a
partially imported store. Importing the same file again in remap mode deliberately
creates duplicates; it is not a synchronization protocol.

## Filesystem behavior

Store writes use a same-directory temporary file, file sync, atomic rename and a
Unix parent-directory sync. Store files are private on Unix. A sidecar lock is
held across the transaction and released by descriptor close. Preserve the
sidecar inode while commands run; never remove it to "unlock" a busy process.
Use a trusted local directory. Locks are advisory and the library caller must
hold `StoreLock` explicitly if combining `Store::load` and `Store::save`.

A corrupted file returns an error. Move a **copy** aside for investigation and
restore a known-good JSON export to a different path first. Do not hand-edit a
store while the CLI is running. Never put the JSON file under the old database's
schema-initialization path; the CLI and the web demo are separate data stores.

## Input limits

Stores, imports and exports are limited to 8 MiB of UTF-8 data. File reads are
bounded even if a file grows after its initial metadata check. Oversized writes
fail before replacing the store. Titles permit text newlines, carriage returns
and tabs, but reject other control characters (including terminal ESC/BEL).
A dismissed reminder must retain its timestamp. Due dates use exactly four-digit
years in `YYYY-MM-DD`; expanded signed years are not accepted.

Export files are also staged in private temporary files and published with a
no-clobber operation. On Unix their permissions are `0600`, regardless of umask.
Only a complete export is published; existing destinations, including symlinks,
are never replaced. Parent directories may be created. The directory itself must
be trusted; permissions and atomic publication are not encryption.
