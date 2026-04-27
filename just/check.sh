#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

load_env_file_if_present

cd "${PROJECT_ROOT}"
cargo check
