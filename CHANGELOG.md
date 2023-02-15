# Changelog

## Unreleased

- Add a database-free `rusty-todos` CLI alongside the original web demo.
- Support validated task CRUD, completion/reopening, categories and priorities.
- Add calendar due dates and UTC reminder scheduling, polling and acknowledgment.
- Compose status/category/priority/search/due filters with stable sorting and pages.
- Report status/category/priority statistics and confirm bulk completed cleanup.
- Export complete versioned JSON backups or interoperable CSV; import atomically
  with explicit merge, remap and replacement policies.
- Use private, atomic file writes and OS-backed transaction locks; refuse corrupt,
  oversized, symlinked or inconsistent data instead of silently replacing it.
- Add real CLI integration tests, fixtures, documentation and cross-platform CI.

The reconstructed commits use user-assigned 2023–2024 timestamps. They are not
historical release attestations; see `docs/development-plan.md` for provenance.
