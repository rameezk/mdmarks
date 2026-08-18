# Scan-on-demand search, no index

mdmarks searches by scanning every Store file on demand rather than maintaining an index or database. A benchmark (#3) of a naive Rust scan showed search is I/O-syscall bound (per-file `open`/`read`/`stat`), and at realistic personal scale (<=5k Bookmarks) even a serial scan clears a comfortable interactive budget: warm 14ms at 1k, 65ms at 5k, 126ms at 10k. An index would remove only the content read, not the directory-walk metadata cost, so it is not worth the added complexity and staleness for v1. Keeping the Store as the single source of truth with no derived state is simpler and more robust.

## Consequences

- The scan **must parallelize across Store files** (embarrassingly parallel, std threads, zero deps); this holds a 10k Store at ~67ms warm versus ~126ms serial.
- **Revisit trigger**: reopen the index/cache option if a Store exceeds ~10k Bookmarks or warm search p95 latency exceeds ~80-100ms. Until then, the index remains out of scope.
