# Search fuzzy-ranking semantics

`search <query>` matches Bookmarks by a case-insensitive fuzzy **subsequence** over the fields `title`, `tags`, and `url` (spec v1 §5, decision #4). The notes `body` is excluded; `space` is the explicit `--space` filter, not a fuzzy target. Each Bookmark scores by its **best matching field** (never a sum across fields) times a field multiplier ordered `title` > `tags` > `url`, with ties broken by `added` descending. Implementing this surfaced two cases §5 left open; this ADR records how they are settled.

## Decisions

### Tags are matched individually, best tag wins

`tags` is a list. The query is scored against each tag separately and the tag field contributes its single best-scoring tag - the tags are **not** joined into one string before matching.

Joining invents contiguity and word-boundary adjacencies across unrelated labels (a query could match by straddling the seam between two tags) and makes a Bookmark's score depend on the order its tags happen to be stored in. Per-tag matching keeps every score meaningful and independent of tag order, so the ranking is deterministic regardless of how the frontmatter list is written.

### A multi-word query is one raw subsequence

The query is used verbatim (lowercased, outer whitespace trimmed) as a single subsequence pattern - internal spaces included. It is **not** split on whitespace into tokens combined with AND semantics.

This mirrors the Alfred Script Filter model, where the query is the raw string the user is incrementally typing (§7), and keeps the Ranker a single pure subsequence scorer with no token-combination policy to specify before there is evidence one is needed.

**Revisit trigger**: reopen with token-AND semantics if space-containing queries prove to miss obvious matches in practice - a literal space rarely survives as a subsequence character in `tags` or `url` values.

## Consequences

- The Ranker stays a pure function of `(query, Bookmark fields)` with no I/O, unit-tested with a table over subsequence quality, field weighting, best-field-wins, and the `added` tie-break.
- Matching is ASCII case-insensitive, consistent with url normalization (ADR-0003); full Unicode case folding is out of scope for v1.
