#!/usr/bin/env bash
set -uo pipefail

export PATH="${HOME:-}/.nix-profile/bin:/etc/profiles/per-user/${USER:-}/bin:/run/current-system/sw/bin:${PATH:-}"

args=(add "${url:-}")
[ -n "${1:-}" ] && args+=(--space "$1")
[ -n "${title:-}" ] && args+=(--title "$title")

result="$("${MDMARKS_BIN:-mdmarks}" "${args[@]}" 2>&1)"

if [ -n "${1:-}" ]; then
	body="${url:-}"$'\n'"Space: ${1}"
else
	body="${url:-}"$'\n'"Space: (default)"
fi

osascript - "$result" "$body" <<'OSA' >/dev/null 2>&1
on run {subtitleText, bodyText}
	display notification bodyText with title "mdmarks" subtitle subtitleText
end run
OSA

exit 0
