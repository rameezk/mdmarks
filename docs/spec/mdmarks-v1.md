# mdmarks v1 design spec

Status: **locked** for v1. This is the hand-off artifact for a build. Load-bearing, hard-to-reverse choices are additionally captured as ADRs under `docs/adr/`; domain vocabulary is defined in `CONTEXT.md`.

mdmarks is a markdown-driven bookmark manager. Each **Bookmark** is a plain markdown file, readable anywhere and native to Obsidian, driven by a fast Rust CLI and an Alfred workflow on macOS.

## 1. Storage

- One markdown file per Bookmark. The **Store** is a flat directory of these files, defaulting to `~/mdmarks`, overridable in config.
- The Store is the single source of truth. There is no index or database (see ADR-0001).
- A Bookmark file is frontmatter plus a free-form markdown body of personal notes.
- **Filename** = a slug of the Bookmark's `title`. On collision, append `-2`, `-3`, … The filename is a convenience for humans browsing the Store; it is never Bookmark identity (see §3).

## 2. Frontmatter schema

YAML frontmatter. `url` is the only required field.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `url` | string | yes | Stored **verbatim** as saved, tracking params and all. `open` launches exactly this. |
| `title` | string | no | Auto-fetched on `add`; falls back to the url on fetch failure. |
| `tags` | list of string | no | Free-form; populated by `import` from source folders. |
| `added` | date-time | no | Set on `add`; preserved from `ADD_DATE` on `import`. |
| `description` | string | no | Short summary. |
| `space` | string | no | Abstract **Space** label (see §4). Unset means the default space. |

The body below the frontmatter is free-form notes, ignored by search matching (see §5).

## 3. Bookmark identity and dedup

- Identity is the **normalized url**, not the filename or the stored url.
- Normalization is **comparison-only**: the stored `url` is never mutated; the normalized form exists solely to decide whether a re-add is a duplicate. Full ruleset in ADR-0003.
- On `add`, if the normalized url matches an existing Bookmark, mdmarks **reports the existing Bookmark and creates no duplicate**.
- `import` applies the same dedup, so re-running an import is safe and idempotent.

## 4. The Space model

A **Space** is an abstract label (e.g. `work`, `personal`) declaring which browsing world a Bookmark belongs to. Config resolves a Space to a concrete browser and optional profile at open time (see ADR-0002).

Config lives at `~/.config/mdmarks/config.toml`:

```toml
store = "~/mdmarks"
default_space = "personal"

[spaces.personal]
browser = "Google Chrome"
profile = "Default"

[spaces.work]
browser = "Google Chrome"
profile = "Work"

[spaces.research]
browser = "Firefox"
profile = "Research"
```

Resolution order for `open`:

1. `--space <name>` flag, if given, wins.
2. else the Bookmark's `space` field.
3. else `default_space`.

`profile` is stored as the human **display name**; mdmarks resolves it to the browser's internal profile at open time (see ADR-0002).

## 5. CLI surface

Binary: `mdmarks`. Read commands (`list`, `search`) accept `--json`.

| Command | Behaviour |
|---------|-----------|
| `add <url>` | Fetch the page title (fall back to the url on failure), write a new Bookmark. On a normalized-url dedup hit, report the existing Bookmark and write nothing. |
| `list` | List all Bookmarks, sorted by `added` descending. `--json` for machine output. |
| `search <query>` | Fuzzy search (see below). `--json` for machine output. `--space <name>` filters by Space. |
| `open <selector>` | Launch a Bookmark's url in its resolved Space (§4). `--space <name>` overrides. |
| `rm <selector>` | Delete a Bookmark file. |
| `import <file.html>` | Ingest an exported Netscape bookmark HTML file (§6). |

### search semantics (from #4)

- **Match**: fuzzy subsequence, case-insensitive.
- **Fields**: `title`, `tags`, `url`. `body` is excluded (noisy). `space` is not a fuzzy target - it is the explicit `--space` filter.
- **Ranking**: fuzzy quality score (contiguity, start-of-word, earliness) times a field multiplier `title` > `tags` > `url`. Best matching field per Bookmark wins; scores are not summed across fields.
- **Tie-break**: `added` descending.
- **Empty query**: returns all Bookmarks, sorted `added` descending. This is the Alfred Script Filter's initial unfiltered feed.

### `--json` record contract (from #16)

`list --json` and `search <query> --json` emit a JSON array of Bookmark records - the same result set, in the same order, as each command's human output (`list` by `added` descending, `search` in ranked order). The human and `--json` renderings are two views over one identical result set and never disagree. An empty Store or a no-match query emits `[]` and exits 0.

Each record carries the frontmatter fields under their frontmatter-schema names; every key is always present so the shape is stable for the Alfred workflow (§7) to depend on:

- `url` - string, stored **verbatim**, never mutated.
- `title` - string or `null` when unset.
- `tags` - array of strings, `[]` when unset.
- `added` - RFC 3339 string or `null` when unset.
- `description` - string or `null` when unset.
- `space` - string or `null` when unset.

### Performance requirement (from #3)

Search is scan-on-demand with **no index** (ADR-0001). The scan **must parallelize across Store files** (embarrassingly parallel, std threads, zero deps). This holds a 10k-Bookmark Store at ~67ms warm versus ~126ms serial. Revisit trigger: a Store exceeding ~10k Bookmarks, or warm search p95 above ~80-100ms (recorded in ADR-0001).

## 6. Import

- Input: an exported **Netscape bookmark HTML** file (Chrome, Firefox, Safari). Reading browsers' live internal stores is out of scope.
- Folder path maps to `tags`.
- `ADD_DATE` maps to `added`.
- `space` is left unset (imported Bookmarks fall to `default_space` until edited).
- Dedup by normalized url (§3); re-runnable and idempotent.

## 7. Alfred workflow

- A **Script Filter** feeds `mdmarks search "$query" --json` and renders the results; the empty-query feed lists everything (§5).
- **Enter** opens the selected Bookmark in its resolved Space (§4).
- **Modifier keys** override the Space from the configured list at open time.
- Exact keyword, the modifier-to-Space binding, and `.alfredworkflow` bundling/distribution are deferred (shape decided, config not yet pinned).

## 8. Explicitly deferred / out of scope for v1

Deferred (in scope, not yet specified): Alfred packaging details; `edit` / `tag` commands; cross-machine sync and backup of the Store.

Out of scope: an index/database backend (ADR-0001 revisit trigger gates re-entry); reading browsers' live internal bookmark stores; non-macOS browser launching.
