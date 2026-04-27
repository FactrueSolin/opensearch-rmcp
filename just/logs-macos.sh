#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

lines="${1:-120}"
if ! [[ "${lines}" =~ ^[0-9]+$ ]]; then
  echo "lines must be a non-negative integer" >&2
  exit 1
fi

mkdir -p "${LOG_DIR}"
touch "${STDOUT_LOG}" "${STDERR_LOG}"

echo "==> ${STDOUT_LOG}"
tail -n "${lines}" "${STDOUT_LOG}"
echo
echo "==> ${STDERR_LOG}"
tail -n "${lines}" "${STDERR_LOG}"
