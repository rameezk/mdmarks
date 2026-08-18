# Space-to-browser resolution at open time

A Bookmark stores an abstract **Space** label, not a concrete browser. Config maps each Space to a `{browser, profile}` pair, and `mdmarks open` resolves that mapping at launch time. This keeps Bookmarks portable (they name a browsing world, not a machine's browser install) and lets one config change re-point every Bookmark in a Space. Launch mechanics were established by research (#2), cited to primary sources.

## Resolution mechanics (macOS)

- **Profile is stored as the human display name**; it is resolved to the browser's internal profile at open time.
- **Chrome**: `open -na "Google Chrome" --args --profile-directory="<dir>" "<url>"`. `--profile-directory` takes the internal dir name (`Default`, `Profile N`), not the display name - map display -> dir via `~/Library/Application Support/Google/Chrome/Local State` (`profile.info_cache[<dir>].name`). `-n` is required or an already-running Chrome ignores the switch.
- **Firefox**: `open -na Firefox --args -P "<name>" --no-remote "<url>"`. `--no-remote` is required or Firefox remotes the url to a running instance and ignores `-P`. A profile cannot be opened twice (profile lock) - handle that failure gracefully.
- **No profile set**: launch the browser plainly (`open -a "<browser>" "<url>"`), or the LaunchServices default when no Space resolves.

## Consequences

- macOS only for v1; non-macOS browser launching is out of scope.
- `open` always launches the stored url verbatim (tracking params and all); normalization (ADR-0003) never touches what is launched.
