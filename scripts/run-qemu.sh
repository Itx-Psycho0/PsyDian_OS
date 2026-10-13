#!/usr/bin/env bash
# Usage:  scripts/run-qemu.sh          -> normal run (serial output in this terminal)
#         scripts/run-qemu.sh debug    -> also log CPU exceptions/resets to qemu.log and DON'T reboot on a crash
set -euo pipefail
cd "$(dirname "$0")/.."

IMG=$(find target -name psydian-bios.img -printf '%T@ %p\n' | sort -n | tail -1 | cut -d' ' -f2-)
[ -n "$IMG" ] || { echo "No psydian-bios.img found - run 'cargo build' first"; exit 1; }
echo "Booting $IMG"

ARGS=(-drive format=raw,file="$IMG" -serial stdio)
if [ "${1:-}" = "debug" ]; then
  # -no-reboot: a triple fault makes QEMU exit instead of silently restarting forever
  ARGS+=(-no-reboot -no-shutdown -d int,cpu_reset,guest_errors -D qemu.log)
  echo "Debug mode: CPU events are logged to qemu.log"
fi
exec qemu-system-x86_64 "${ARGS[@]}"