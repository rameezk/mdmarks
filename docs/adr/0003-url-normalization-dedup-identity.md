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

## Amendment: under-specified cases (from #7 implementation)

The original ruleset did not cover these; the `add` slice pinned them down rather than deciding silently in code:

- **Port**: preserved in identity. Default ports for the scheme are dropped (the url parser removes `:80`/`:443`), so a non-default port is a genuine identity difference and is kept.
- **Userinfo (`user:pass@`)**: dropped from identity. Credentials are not part of what makes a Bookmark the same page, and are rare in saved links; the stored url still keeps them verbatim.
- **Percent-encoding in the query**: normalized consistently by decoding each param key/value once before comparison, so `%20` vs `+` vs a literal space in a query value do not split identity. Path percent-encoding is left as-is (path case and bytes are significant).

## Amendment: fragment retained in identity (reversal of the original rule)

The original ruleset **dropped the fragment entirely**. That was wrong for fragment-routed apps: single-page apps and consoles (log viewers, cloud portals, dashboards) carry the entire meaningful state in the `#` fragment, so distinct Bookmarks sharing a base url collapsed onto one identity and every one but the first was silently skipped as a duplicate on import. A single export lost double-digit distinct Bookmarks this way - silent data loss, the worst possible outcome for a dedup rule.

The fragment is therefore now **part of identity**:

- **Non-empty fragment**: kept and compared. `page#a` and `page#b` are distinct Bookmarks; `page#a` and `page` are distinct.
- **Empty fragment**: a bare trailing `#` (`page#`) carries no state and is treated identically to no fragment, so `page#` merges with `page`.
- The rest of the ruleset above is unchanged: an anchor-only difference is the only thing the fragment now splits on; scheme, host, `www.`, trailing slash, tracker params, and query order still normalize away.

This trades the old convenience of merging `#section` anchor links of the same document (they now save as two Bookmarks) for never silently dropping a fragment-routed Bookmark. Merging distinct links is unrecoverable; keeping an extra anchor Bookmark is trivially deleted. The reversal is itself hard to reverse in the same way the original was: it re-partitions identity, so applying it changes which past re-adds would have merged.
