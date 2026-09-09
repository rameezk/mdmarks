#!/usr/bin/env bash
set -euo pipefail

export PATH="${HOME:-}/.nix-profile/bin:/etc/profiles/per-user/${USER:-}/bin:/run/current-system/sw/bin:${PATH:-}"

read -r url <<<"$(pbpaste 2>/dev/null || true)" || true
title="${1-}"

case "$url" in
	http://* | https://*)
		"${MDMARKS_BIN:-mdmarks}" spaces --format alfred | jq \
			--arg url "$url" \
			--arg title "$title" '
				.items |= map(
					.variables = {url: $url, title: $title}
					| .subtitle = (
						"Save " + $url
						+ "  →  " + (if .arg == "" then "default space" else "space: " + .arg end)
						+ "  ·  " + (if $title == "" then "title fetched from page" else "title: " + $title end)
					)
				)
			'
		;;
	*)
		printf '%s\n' '{"items":[{"title":"Clipboard isn'\''t a URL","subtitle":"Copy an http(s) link, then run bma","valid":false}]}'
		;;
esac
