# mdmarks

A markdown-driven bookmark manager. Each Bookmark is a plain markdown file you own - readable anywhere and native to Obsidian - driven by a fast Rust CLI.

See `docs/spec/mdmarks-v1.md` for the v1 design, `CONTEXT.md` for domain vocabulary, and `docs/adr/` for load-bearing decisions.

## Installation

mdmarks is distributed as a Nix flake. With [Nix](https://nixos.org/download) (flakes enabled), pick whichever fits how you use Nix.

Try it once without installing:

```sh
nix run github:rameezk/mdmarks -- add <url>
```

Install it into your profile (`mdmarks` on your `PATH`, removable with `nix profile remove`):

```sh
nix profile install github:rameezk/mdmarks
```

Add it to a declarative NixOS or home-manager config. In your flake's inputs:

```nix
inputs.mdmarks.url = "github:rameezk/mdmarks";
```

Then add the package to `home.packages` (home-manager) or `environment.systemPackages` (NixOS), matching `${system}` to your host (e.g. `x86_64-linux`, `aarch64-darwin`):

```nix
inputs.mdmarks.packages.${system}.default
```

These references track the latest commit on the default branch. To pin a specific version, append a commit hash: `github:rameezk/mdmarks/<commit>`.

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

## Building and running with Nix

The flake exposes a reproducible optimized build (release profile, symbols stripped):

```sh
nix build            # builds ./result/bin/mdmarks
nix run . -- add <url>   # builds if needed, then runs the binary
```

`nix build` produces the same binary you would ship; the network-dependent test suite is not run inside the Nix sandbox, so run `cargo test` in the dev shell for tests.

## Usage

```sh
mdmarks add <url> [--title <string>]
```

`add` fetches the page title (falling back to the url on any failure), then writes a new Bookmark to the Store. Re-adding a link already saved (matched by normalized-url identity) reports the existing Bookmark and writes nothing.

The Store path resolves in order: the `MDMARKS_STORE` environment variable, then `store` in `~/.config/mdmarks/config.toml`, then the `~/mdmarks` default.
