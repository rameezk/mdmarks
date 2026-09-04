# Two launch commands: `open` is the machine ABI, `go` is the human path

mdmarks launches a Bookmark's url in a resolved Space two ways: `open <url>` and `go <query>` (spec v1 §5). They look redundant - both end in the same launch - but they serve different callers, and collapsing them into one command degrades both. This ADR records why there are two.

## Context

The Alfred workflow (§7) already selects a Bookmark from `search --json` and needs a command that launches *that exact record*. A human at a terminal has no record in hand - they have a fuzzy notion ("the rust one") and want to find-and-launch in one step. The v1 spec originally wrote both as `open <selector>`, leaving "selector" undefined; implementing it forced the question of what a selector actually is.

## Decisions

### `open` selects by exact, verbatim url only

`open` matches a Bookmark by string equality on the stored `url` - tracking params and all, no normalization, no fuzzing. It is the stable ABI the Alfred workflow calls with a url it pulled from a `search --json` record. Its input is machine-produced, so it needs no ergonomics; it needs to be unambiguous and never launch the wrong thing.

### `go` is the human find-and-launch path

`go <query>` reuses the `search` fuzzy ranking (ADR-0004) to resolve a query to a Bookmark, then launches it through the identical Space-resolution and verbatim-url launch path as `open`. Exactly one match launches; zero or many launch nothing and print candidates to stderr so the user narrows (exit 1 no-match, exit 2 ambiguous). `--first`/`-1` overrides the ambiguity guard for a non-empty query.

## Alternatives rejected

- **Overload `open` with fuzzy selection.** Makes the Alfred ABI non-deterministic - the same argument could launch different Bookmarks as the Store changes - and mixes a machine contract with human ergonomics in one command. Keeping `open` exact-only preserves it as a dependable ABI.
- **Always launch the top-ranked fuzzy match.** Zero friction, but a vague query silently opens the wrong page. The silent-wrong-launch is only ever reached now via explicit `--first` on a real query.
- **Launch the top match when it beats the runner-up by a threshold.** Introduces a magic tuning constant that is hard to reason about and hard to test deterministically - against the grain of the hand-rolled, threshold-free matcher (ADR-0004).

## Consequences

- The spec's undefined `open <selector>` resolves into two well-defined jobs: `open` = exact url (machine), `go` = fuzzy (human). `rm` likewise selects by exact url.
- `go` adds no new matching logic - it is the `search` Ranker plus the `open` launch path plus the ambiguity rule. The Ranker stays a pure function reused by both `search` and `go`.
- `go` deliberately carries no `--json`: it is an action. The machine surface stays `search --json` feeding `open`.

**Revisit trigger**: reopen if a terminal picker (interactive filtering) is wanted - that is a different interaction model than resolve-then-launch and would weigh a TUI dependency against the repo's zero-dependency stance, deferred until there is demand Alfred does not already serve.
