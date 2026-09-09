#!/usr/bin/env bash
set -uo pipefail

export PATH="${HOME:-}/.nix-profile/bin:/etc/profiles/per-user/${USER:-}/bin:/run/current-system/sw/bin:${PATH:-}"

args=(add "${url:-}")
[ -n "${1:-}" ] && args+=(--space "$1")
[ -n "${title:-}" ] && args+=(--title "$title")

result="$("${MDMARKS_BIN:-mdmarks}" "${args[@]}" 2>&1)"

if [ -n "${1:-}" ]; then
	space="${1}"
else
	space="(default)"
fi

printf '%s\n%s · %s\n' "$result" "${url:-}" "$space"

exit 0
