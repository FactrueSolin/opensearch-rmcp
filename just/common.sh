#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

BINARY_NAME="${OPENPERPLEXITY_BIN:-searxng_mcp}"
SERVICE_LABEL="${OPENPERPLEXITY_SERVICE_LABEL:-com.factrue.openperplexity.searxng-mcp}"
LAUNCH_AGENT_DIR="${HOME}/Library/LaunchAgents"
PLIST_PATH="${LAUNCH_AGENT_DIR}/${SERVICE_LABEL}.plist"
LOG_DIR="${PROJECT_ROOT}/data/launchd"
STDOUT_LOG="${LOG_DIR}/${SERVICE_LABEL}.out.log"
STDERR_LOG="${LOG_DIR}/${SERVICE_LABEL}.err.log"

launchctl_domain() {
  printf 'gui/%s' "$(id -u)"
}

ensure_macos() {
  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "This command only supports macOS launchd." >&2
    exit 1
  fi
}

ensure_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "Missing required command: ${command_name}" >&2
    exit 1
  fi
}

ensure_env_file() {
  if [[ ! -f "${PROJECT_ROOT}/.env" ]]; then
    echo "Missing ${PROJECT_ROOT}/.env. Create it from .env.example before deploying." >&2
    exit 1
  fi
}

load_env_file() {
  ensure_env_file
  set -a
  # shellcheck source=/dev/null
  source "${PROJECT_ROOT}/.env"
  set +a
}

load_env_file_if_present() {
  if [[ -f "${PROJECT_ROOT}/.env" ]]; then
    set -a
    # shellcheck source=/dev/null
    source "${PROJECT_ROOT}/.env"
    set +a
  fi
}

xml_escape() {
  sed \
    -e 's/&/\&amp;/g' \
    -e 's/</\&lt;/g' \
    -e 's/>/\&gt;/g' \
    -e 's/"/\&quot;/g' \
    -e "s/'/\&apos;/g"
}

escaped() {
  printf '%s' "$1" | xml_escape
}

print_paths() {
  echo "project: ${PROJECT_ROOT}"
  echo "plist:   ${PLIST_PATH}"
  echo "stdout:  ${STDOUT_LOG}"
  echo "stderr:  ${STDERR_LOG}"
}
