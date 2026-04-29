#!/bin/bash
set -euo pipefail

BOOT_MARKER="Hello from WinnieOS!"
SERIAL_LOG="qemu-serial.log"

bash ./scripts/build-image.sh
rm -f "$SERIAL_LOG"

qemu-system-x86_64 \
    -cdrom winnie.iso \
    -no-reboot \
    -display none \
    -serial "file:${SERIAL_LOG}" \
    -d int,cpu_reset \
    -D qemu.log &
qemu_pid=$!

cleanup() {
    if kill -0 "$qemu_pid" >/dev/null 2>&1; then
        kill "$qemu_pid" >/dev/null 2>&1 || true
        wait "$qemu_pid" >/dev/null 2>&1 || true
    fi
}

trap cleanup EXIT

for _ in $(seq 1 50); do
    if grep -Fq "$BOOT_MARKER" "$SERIAL_LOG" 2>/dev/null; then
        exit 0
    fi

    if ! kill -0 "$qemu_pid" >/dev/null 2>&1; then
        wait "$qemu_pid"
        break
    fi

    sleep 0.1
done

printf '%s\n' "Smoke boot failed: missing serial marker '${BOOT_MARKER}'."
if [ -f "$SERIAL_LOG" ]; then
    cat "$SERIAL_LOG"
fi
exit 1
