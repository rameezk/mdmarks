# URL normalization as dedup identity

Bookmark identity for dedup is a **normalized** form of the url, not the stored url or the filename. Normalization is **comparison-only**: the `url` in frontmatter is stored verbatim and never mutated - the normalized form is computed solely to decide whether a re-add (via `add` or `import`) is a duplicate. This is hard to reverse because it partitions Bookmark identity: change the ruleset later and previously-distinct Bookmarks may collide, or previously-merged ones split.

## Ruleset

- **Scheme**: `http` and `https` unified (normalize to `https`).
- **Host**: lowercased; leading `www.` stripped.
- **Trailing slash**: stripped from the path; root `/` kept.
- **Fragment**: `#...` dropped entirely.
- **Query params**: known trackers stripped (`utm_*`, `fbclid`, `gclid`, `mc_eid`, `ref`); remaining params kept and sorted alphabetically (order-insensitive).
- **Path case**: preserved (paths are case-sensitive).

## Consequences

- `open` and display always use the stored verbatim url; normalization is invisible except at dedup time.
- Adding a known url reports the existing Bookmark and writes nothing.
