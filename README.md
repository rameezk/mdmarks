# mdmarks

A markdown-driven bookmark manager. Each Bookmark is a plain markdown file you own - readable anywhere and native to Obsidian - driven by a fast Rust CLI.

See `docs/spec/mdmarks-v1.md` for the v1 design, `CONTEXT.md` for domain vocabulary, and `docs/adr/` for load-bearing decisions.

## Development environment

The dev environment is a Nix flake. With [Nix](https://nixos.org/download) (flakes enabled) and [direnv](https://direnv.net):

```sh
direnv allow
```

`direnv` reads `.envrc` (`use flake`) and drops you into a shell with the Rust toolchain (`cargo`, `rustc`, `clippy`, `rustfmt`, `rust-analyzer`). Without direnv, use `nix develop` to enter the same shell manually.

## Common tasks

```sh
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

## Usage

```sh
mdmarks add <url> [--title <string>]
```

`add` fetches the page title (falling back to the url on any failure), then writes a new Bookmark to the Store. Re-adding a link already saved (matched by normalized-url identity) reports the existing Bookmark and writes nothing.

The Store path resolves in order: the `MDMARKS_STORE` environment variable, then `store` in `~/.config/mdmarks/config.toml`, then the `~/mdmarks` default.
