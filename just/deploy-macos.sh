#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

ensure_macos
ensure_command cargo
ensure_command launchctl
load_env_file

cd "${PROJECT_ROOT}"
cargo build --release --bin "${BINARY_NAME}"

binary_path="${PROJECT_ROOT}/target/release/${BINARY_NAME}"
if [[ ! -x "${binary_path}" ]]; then
  echo "Expected binary not found: ${binary_path}" >&2
  exit 1
fi

mkdir -p "${LAUNCH_AGENT_DIR}" "${LOG_DIR}"

label_xml="$(escaped "${SERVICE_LABEL}")"
binary_xml="$(escaped "${binary_path}")"
project_xml="$(escaped "${PROJECT_ROOT}")"
stdout_xml="$(escaped "${STDOUT_LOG}")"
stderr_xml="$(escaped "${STDERR_LOG}")"
rust_log_xml="$(escaped "${OPENPERPLEXITY_RUST_LOG:-info}")"

cat >"${PLIST_PATH}" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${label_xml}</string>

  <key>ProgramArguments</key>
  <array>
    <string>${binary_xml}</string>
  </array>

  <key>WorkingDirectory</key>
  <string>${project_xml}</string>

  <key>EnvironmentVariables</key>
  <dict>
    <key>RUST_LOG</key>
    <string>${rust_log_xml}</string>
  </dict>

  <key>RunAtLoad</key>
  <true/>

  <key>KeepAlive</key>
  <true/>

  <key>StandardOutPath</key>
  <string>${stdout_xml}</string>

  <key>StandardErrorPath</key>
  <string>${stderr_xml}</string>
</dict>
</plist>
PLIST

domain="$(launchctl_domain)"

if launchctl print "${domain}/${SERVICE_LABEL}" >/dev/null 2>&1; then
  launchctl bootout "${domain}" "${PLIST_PATH}" >/dev/null 2>&1 || true
fi

launchctl bootstrap "${domain}" "${PLIST_PATH}"
launchctl enable "${domain}/${SERVICE_LABEL}"
launchctl kickstart -k "${domain}/${SERVICE_LABEL}"

echo "Deployed ${SERVICE_LABEL} as a macOS LaunchAgent."
print_paths
