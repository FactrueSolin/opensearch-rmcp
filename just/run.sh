#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

load_env_file

cd "${PROJECT_ROOT}"
cargo run --bin "${BINARY_NAME}"
