#!/usr/bin/env bash
set -euo pipefail

exec "${MDMARKS_BIN:-mdmarks}" search "${1-}" --format alfred
