## Overview

Schema migrations for the konduit indexer. The naming follows the convention
established by [Kupo](https://cardanosolutions.github.io/kupo/#section/Schema-Migrations):

```
db/
  vX.Y.Z/
    NNN.sql
```

Where:

- `vX.Y.Z` is the software version that **introduced** the migration.
- `NNN.sql` is a zero-padded sequence number — multiple SQL files per version
  are allowed when a single migration spans several scripts.

## Application

Migration files are embedded into the indexer binary at compile time via
`include_str!` (the Rust counterpart of Haskell's `Data.FileEmbed.embedFile`).
On startup the migrator reads `PRAGMA user_version` and applies, in order, every
migration whose revision number is greater than the current one. Each
migration runs inside its own transaction and ends with a
`PRAGMA user_version = N;` statement that records the new schema version.

See `../src/store/sqlite/migrations.rs` for the implementation.

## Changelog

<p align="right"><code>v1.0.0</code></p>
<hr/>

### Migration to `version=1`

- Initial schema: `block`, `channel`, `step` tables and supporting indexes
  and triggers for the Konduit channel-state indexer.
