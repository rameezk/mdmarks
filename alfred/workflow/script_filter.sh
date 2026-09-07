#!/usr/bin/env bash
set -euo pipefail

export PATH="${HOME:-}/.nix-profile/bin:/etc/profiles/per-user/${USER:-}/bin:/run/current-system/sw/bin:${PATH:-}"

exec "${MDMARKS_BIN:-mdmarks}" search "${1-}" --format alfred
