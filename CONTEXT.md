# mdmarks

A markdown-driven bookmark manager. Each bookmark is a plain markdown file with frontmatter, readable anywhere and native to Obsidian, driven by a fast Rust CLI and an Alfred workflow.

## Language

**Bookmark**:
A single saved link, stored as one markdown file: frontmatter (structured metadata) plus a free-form body of personal notes.
_Avoid_: Link, mark, entry

**Store**:
The directory holding all bookmark files. Defaults to `~/mdmarks`, overridable via config. The single source of truth.
_Avoid_: Vault, library, database

**Space**:
An abstract label on a bookmark (for example `work` or `personal`) declaring which browsing world it belongs to. Config resolves a space to a concrete browser and optional profile at open time.
_Avoid_: Context, profile, environment

**Default space**:
The space used when a bookmark declares none, set in config.

**Import**:
Ingesting bookmarks from an exported Netscape bookmark HTML file (Chrome, Firefox, Safari), mapping folders to tags and preserving original add dates.
