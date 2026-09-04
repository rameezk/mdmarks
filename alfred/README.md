# mdmarks Alfred workflow

A [Script Filter](https://www.alfredapp.com/help/workflows/inputs/script-filter/) workflow that fuzzy-finds Bookmarks across Spaces and opens each in its resolved browser profile. All query parsing, `space:` scoping, ranking, and output shaping live in the `mdmarks` CLI; the workflow is a thin GUI graph over `mdmarks search --format alfred` and `mdmarks open`.

## Layout

- `workflow/` - the unpacked, diffable bundle:
  - `script_filter.sh` - the Script Filter body, a pass-through to `mdmarks search "$1" --format alfred`.
  - `info.plist` - workflow metadata, environment variables, and the configuration sheet. The node graph is assembled in the Alfred GUI (see below), then re-exported over this file.
  - `icon.png` - workflow icon placeholder.
- `mdmarks.alfredworkflow` - the packaged bundle for one-double-click install. Rebuild it from `workflow/` with:

  ```sh
  ( cd alfred/workflow && zip -r -X ../mdmarks.alfredworkflow . -x '.*' )
  ```

## Install

Double-click `mdmarks.alfredworkflow` to import it into Alfred (Powerpack required), then set the binary and PATH as described under [Environment variables](#environment-variables).

## GUI assembly checklist

Import the bundle, then build the node graph in Alfred's workflow editor. Each object and connection below is required to reproduce the workflow.

### 1. Script Filter (the input)

- Add a **Script Filter** object.
- **Keyword**: `bm`.
- **Argument**: `Argument Optional` - an empty query is valid and returns the full newest-first feed.
- Leave **"Alfred filters results"** _off_. mdmarks emits items in its own ranked order (title > tags > url) and Alfred must not re-rank them.
- **Language / Script**: set to `External Script` and point it at `./script_filter.sh` (bundled alongside `info.plist`). Choose **"with input as {query}"** so the query arrives as `$1`.
- The script reads `${MDMARKS_BIN:-mdmarks}` and runs `mdmarks search "$1" --format alfred`. Do not duplicate any `space:` parsing here - the CLI owns it.

### 2. Enter -> open (the default action)

- Add a **Run Script** action.
- **Language**: `/bin/bash`, **with input as argv**.
- **Script**: `"${MDMARKS_BIN:-mdmarks}" open "$1"`.
- Connect the Script Filter's **default (unmodified Enter)** output to this Run Script. Enter opens the selected Bookmark's `arg` (its verbatim URL) in the Space's resolved browser + profile.

### 3. ⌘↵ -> Copy to Clipboard

- Add a **Copy to Clipboard** output. Set its text to `{query}` and enable **"Automatically paste to frontmost app"** off (copy only).
- Connect the Script Filter's **⌘ (Command) modifier** output to it. Each item already sets `mods.cmd.arg` to the URL and its subtitle to "Copy URL", so holding ⌘ previews the copy and ⌘↵ performs it.

### 4. Right arrow -> Universal Actions

- No wiring needed. Each item carries `action.url`, so pressing **right arrow (→)** hands the URL to Alfred's Universal Actions panel (Copy URL, Open URL, and the rest) for free.

## Environment variables

Alfred runs scripts with a minimal `PATH`, and `mdmarks` is installed via Nix outside that `PATH`, so the workflow has to be told where the binary is. Set these in the workflow's **Environment Variables** pane (workflow editor → `[𝓍]` in the top-right).

> Alfred stores these values **literally** and does not expand `~`, `$HOME`, or `$USER`. Every path below must be a real absolute path (`/Users/<you>/...`), not a variable reference.

- **`MDMARKS_BIN`** (recommended): the absolute path to the binary, e.g. `/Users/<you>/.nix-profile/bin/mdmarks`. An absolute path is the robust setup - it bypasses `PATH` resolution for the binary entirely, so the workflow keeps working regardless of what `PATH` contains. Leave it as the bare `mdmarks` default only if the binary's directory is on the `PATH` below.

- **`PATH`**: `mdmarks open` shells out to `/usr/bin/open`, so the `PATH` only needs the system defaults:

  ```
  /usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin
  ```

  You only need to prepend the Nix profile locations here if you left `MDMARKS_BIN` as the bare `mdmarks` default - and then as literal absolute paths, e.g. `/Users/<you>/.nix-profile/bin:/etc/profiles/per-user/<you>/bin:/run/current-system/sw/bin:` ahead of the defaults (`~/.nix-profile/bin` for single-user Nix, `/etc/profiles/per-user/<you>/bin` for nix-darwin/home-manager, `/run/current-system/sw/bin` for the system profile).

`MDMARKS_BIN` is also surfaced in the **Workflow Configuration** sheet (the user-facing `[×]` panel), so it can be overridden per-install without editing the graph. Both the Script Filter and the Enter Run Script inherit these variables.

## How it maps to the CLI

| Alfred gesture | CLI call |
| --- | --- |
| Type `bm <query>` | `mdmarks search "<query>" --format alfred` |
| Type `bm <space>: <query>` | same call; the CLI scopes to `<space>` when it is a configured Space |
| Enter | `mdmarks open "<url>"` |
| ⌘↵ | copy `<url>` |
| → | Alfred Universal Actions on `<url>` |
