# Alfred quick-add: clipboard url, query-as-title, rows-as-spaces

The Alfred workflow adds a Bookmark through a single Script Filter (keyword `bma`): the url comes from the clipboard, the typed query *is* the optional title override, and the rows are the configured Spaces. A future reader will expect typing to filter the rows - it does not - so this ADR records why the flow is shaped this way.

## Context

Adding a Bookmark needs three inputs: a url, an optional title, and a Space. Alfred has no multi-field form; every node reads one input line. The CLI already does the heavy lifting - `add <url> [--title] [--space]` exists and `add` fetches the page title itself, falling back to the url. So the workflow is a thin shell-out, and the only real design question is how to collect three inputs through Alfred's single-input model without a multi-step chain that forces a title prompt nobody usually wants.

## Decisions

### The url comes from the clipboard

`bma` takes no argument; the Script Filter reads the clipboard (`pbpaste`). A non-http(s) clipboard yields a single non-actionable row ("Clipboard isn't a URL") rather than handing garbage to `add`. This keeps the flow browser-agnostic and zero-config - no per-browser AppleScript to read a frontmost tab.

### The typed query is the title override, not a Space filter

Rows are the configured Spaces (a leading "(default)" row adds with no `--space`, inheriting `default_space`). Whatever the user types is passed as `--title`; an empty query lets the CLI infer the title. The query therefore does **not** filter the rows - the Space list stays whole and the user arrow-selects.

### Feedback is `add`'s stdout, forwarded verbatim

`add` emits one clean line per outcome (`Saved ✓ …`, `Already saved: …`) plus exit codes; the Run Script forwards stdout straight to a notification. No structured output, no parser dependency.

## Alternatives rejected

- **Query filters Spaces; title-override moves to a modifier (⌥-Enter).** Preserves filtering but demotes title-override to a hidden, less discoverable path. With a handful of config-defined Spaces there is nothing to filter, so the cost lands on the feature people actually use.
- **Two-stage chain (pick Space → title prompt → add).** Forces a title step on every add when the inferred title is almost always fine, and spreads one action across three nodes.
- **Read the frontmost browser tab's url.** Zero-copy, but needs per-browser AppleScript and breaks across the browser variants in use (Helium/Chrome forks). Clipboard is universal and needs no maintenance.
- **`add --format json` parsed in the workflow.** Consistent with `search --format alfred`, but the notification would need `jq` on PATH, and there is exactly one consumer needing one human string. Deferred until a second consumer exists.

## Consequences

- The workflow needs to enumerate Spaces, which the CLI did not expose - hence a new `spaces` command (`--format alfred` for the picker, plain lines for humans) reading config.
- Quick-add is deliberately url + title + Space only. Tags and the note body are non-goals for the fast path; set them later in Obsidian or a future edit path.
- Live preview cannot show the *fetched* title (an HTTP call per keystroke is too slow), so an empty query previews "fetched from page"; the real title appears only in the post-add notification.

**Revisit trigger**: reopen if the configured Space count grows large enough that arrowing is painful (reconsider query-filters-Spaces), or if a second consumer of `add`'s result appears (reconsider a machine-readable `--format`).
