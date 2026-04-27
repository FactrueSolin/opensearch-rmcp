#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

ensure_macos
ensure_command launchctl

domain="$(launchctl_domain)"

if launchctl print "${domain}/${SERVICE_LABEL}" >/dev/null 2>&1; then
  echo "${SERVICE_LABEL} is loaded."
  launchctl print "${domain}/${SERVICE_LABEL}" | sed -n '1,80p'
else
  echo "${SERVICE_LABEL} is not loaded."
  print_paths
  exit 1
fi
