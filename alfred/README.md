# mdmarks Alfred workflow

A [Script Filter](https://www.alfredapp.com/help/workflows/inputs/script-filter/) workflow that fuzzy-finds Bookmarks across Spaces and opens each in its resolved browser profile, and quick-adds the clipboard URL as a Bookmark. All query parsing, `space:` scoping, ranking, and output shaping live in the `mdmarks` CLI; the workflow is a thin graph over `mdmarks search --format alfred`, `mdmarks spaces --format alfred`, `mdmarks add`, and `mdmarks open`.

The bundle ships fully wired - double-click to import and it works, no manual assembly in the Alfred editor.

## Install

1. Double-click `mdmarks.alfredworkflow` to import it into Alfred (Powerpack required).
2. Make sure the `mdmarks` binary is resolvable (see [Finding the binary](#finding-the-binary)). For a standard Nix install there is nothing to do.
3. Trigger it with the `bm` keyword.

## Usage

| Alfred gesture | Result |
| --- | --- |
| `bm <query>` | fuzzy-find Bookmarks; empty query lists the full newest-first feed |
| `bm <space>: <query>` | scope to `<space>` when it is a configured Space (the CLI parses the prefix) |
| Enter | `mdmarks open <url>` - opens the Bookmark's verbatim URL in the Space's resolved browser + profile |
| Right arrow (→) | hands the URL to Alfred's Universal Actions (Copy URL, and the rest) |
| `bma [title]` | quick-add the clipboard URL as a Bookmark; the typed query is an optional title override |

### Quick-add (`bma`)

Copy an http(s) URL, then run `bma`. The Script Filter reads the clipboard (`pbpaste`) and shows the configured Spaces as rows - a leading `(default)` row adds with no `--space` (inheriting `default_space`). A non-http(s) clipboard yields a single non-actionable row and adds nothing.

Whatever you type is the title override, not a row filter; an empty query lets the CLI fetch the page title (one HTTP round-trip, so a slow page delays the confirmation by a second or two). Enter runs `mdmarks add <url> --space <space> [--title <query>]`; the action posts its own macOS notification showing the result (`Saved ✓ …` / `Already saved: …` / the error), the URL, and the chosen Space. Design rationale: `docs/adr/0006-alfred-quick-add-interaction-model.md`.

## Finding the binary

Alfred runs scripts with a minimal environment, and `mdmarks` is typically installed via Nix outside that `PATH`. Every script (both Script Filters and both actions) handles this itself:

- They prepend the standard Nix profile directories to `PATH` at runtime: `$HOME/.nix-profile/bin` (single-user Nix), `/etc/profiles/per-user/$USER/bin` (nix-darwin / home-manager), and `/run/current-system/sw/bin` (system profile). `$HOME` and `$USER` expand because this happens inside the running shell, not in Alfred's environment-variable pane (which stores values literally and does not expand `~`, `$HOME`, or `$USER`).
- If your `mdmarks` lives somewhere else, set **`MDMARKS_BIN`** to its absolute path in the workflow's **Configuration** sheet (`[×]`). It takes precedence over `PATH` resolution entirely.

## Editing

The node graph is authored directly in `workflow/info.plist` and committed as the source of truth:

- `workflow/` - the unpacked, diffable bundle:
  - `info.plist` - metadata, environment variables, the configuration sheet, **and** the wired node graph (`objects` + `connections`).
  - `script_filter.sh` - the search Script Filter body: prepends the Nix `PATH` and runs `mdmarks search "$1" --format alfred`.
  - `quick_add.sh` - the `bma` Script Filter body: reads the clipboard, and on an http(s) URL shapes `mdmarks spaces --format alfred` into the Space picker (carrying the URL and typed title as feed variables). Requires `jq`, resolved off the same Nix `PATH`.
  - `quick_add_action.sh` - the `bma` Run Script body: runs `mdmarks add <url> [--space] [--title]` and posts the result as a macOS notification via `osascript`. It posts the notification itself (rather than wiring a Post Notification node) because Alfred does not forward a Run Script action's stdout to a downstream notification, and `mdmarks add`'s non-zero exit codes (already-saved, error) would otherwise be swallowed.
  - `icon.png` - workflow icon (placeholder).
- `mdmarks.alfredworkflow` - the packaged bundle. Rebuild it from `workflow/` with:

  ```sh
  ( cd alfred/workflow && zip -r -X ../mdmarks.alfredworkflow . -x '.*' )
  ```

> Do **not** re-export the workflow from the Alfred GUI over `info.plist`. A GUI export rewrites the file and would drop the committed structure and the system-default `PATH`. Edit `info.plist` (and `script_filter.sh`) by hand, then rebuild the zip with the command above. `tests/alfred_workflow.rs` guards against `info.plist` regressing to an empty graph.
