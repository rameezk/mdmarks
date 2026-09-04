# Space-to-browser resolution at open time

A Bookmark stores an abstract **Space** label, not a concrete browser. Config maps each Space to a `{browser, profile}` pair, and `mdmarks open` resolves that mapping at launch time. This keeps Bookmarks portable (they name a browsing world, not a machine's browser install) and lets one config change re-point every Bookmark in a Space. Launch mechanics were established by research (#2), cited to primary sources.

## Resolution mechanics (macOS)

- **Profile is stored as the human display name**; it is resolved to the browser's internal profile at open time.
- **Chrome**: `open -na "Google Chrome" --args --profile-directory="<dir>" "<url>"`. `--profile-directory` takes the internal dir name (`Default`, `Profile N`), not the display name - map display -> dir via `~/Library/Application Support/Google/Chrome/Local State` (`profile.info_cache[<dir>].name`). `-n` is required or an already-running Chrome ignores the switch.
- **Firefox**: `open -na Firefox --args -P "<name>" --no-remote "<url>"`. `--no-remote` is required or Firefox remotes the url to a running instance and ignores `-P`. A profile cannot be opened twice (profile lock) - handle that failure gracefully.
- **No profile set**: launch the browser plainly (`open -a "<browser>" "<url>"`), or the LaunchServices default when no Space resolves.

### Amendment: Chromium-family profiles are not Chrome-only

`--profile-directory` and the `Local State` display-name → internal-dir mapping are shared by every Chromium fork, but each fork keeps its `Local State` under its own application-support directory. A Space therefore carries an optional `chromium_support_dir` naming that directory (relative to `~/Library/Application Support/`); the `Local State` read is `~/Library/Application Support/<chromium_support_dir>/Local State`. `"Google Chrome"` defaults to `Google/Chrome`, so a plain Chrome Space needs no `chromium_support_dir`. A profiled Space whose browser has neither a configured `chromium_support_dir` nor a built-in default is unsupported (this is where Firefox profiles, ADR-#23, resolve differently). A display name matching more than one profile in `Local State` is a clear error, non-zero, nothing launched - never a silent wrong-profile launch. Example - Helium (`net.imput.helium`):

```toml
[spaces.work]
browser = "Helium"
profile = "Work"
chromium_support_dir = "net.imput.helium"
```

## Consequences

- macOS only for v1; non-macOS browser launching is out of scope.
- `open` always launches the stored url verbatim (tracking params and all); normalization (ADR-0003) never touches what is launched.
