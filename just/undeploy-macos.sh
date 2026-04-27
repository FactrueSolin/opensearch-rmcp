#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

ensure_macos
ensure_command launchctl

domain="$(launchctl_domain)"

if launchctl print "${domain}/${SERVICE_LABEL}" >/dev/null 2>&1; then
  launchctl bootout "${domain}" "${PLIST_PATH}" >/dev/null 2>&1 || true
fi

rm -f "${PLIST_PATH}"

echo "Undeployed ${SERVICE_LABEL}."
print_paths
