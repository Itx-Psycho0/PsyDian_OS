#!/usr/bin/env bash
# Thin wrapper kept for convenience; the launcher lives in src/main.rs.
# Usage:  scripts/run-qemu.sh             -> build and boot in a QEMU window
#         scripts/run-qemu.sh debug       -> also log CPU exceptions/resets to qemu.log
#         scripts/run-qemu.sh headless    -> no window, serial console only
set -euo pipefail
cd "$(dirname "$0")/.."

case "${1:-}" in
  debug)    exec cargo run -- --debug ;;
  headless) exec cargo run -- --headless ;;
  *)        exec cargo run ;;
esac
