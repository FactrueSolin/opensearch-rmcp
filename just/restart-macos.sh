#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

ensure_macos
ensure_command launchctl

domain="$(launchctl_domain)"

if ! launchctl print "${domain}/${SERVICE_LABEL}" >/dev/null 2>&1; then
  echo "${SERVICE_LABEL} is not loaded. Run: just deploy" >&2
  exit 1
fi

launchctl kickstart -k "${domain}/${SERVICE_LABEL}"
echo "Restarted ${SERVICE_LABEL}."
