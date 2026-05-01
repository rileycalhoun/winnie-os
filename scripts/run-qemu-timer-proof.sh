#!/bin/bash
set -euo pipefail

BOOT_MARKER="Hello from WinnieOS!"
TIMER_MARKER="TIMER PROOF OK"
SERIAL_LOG="qemu-timer.log"
ISO_PATH="$(mktemp "${TMPDIR:-/tmp}/winnie-timer.XXXXXX.iso")"

cleanup() {
    rm -f "$ISO_PATH"
    if [ -n "${qemu_pid:-}" ] && kill -0 "$qemu_pid" >/dev/null 2>&1; then
        kill "$qemu_pid" >/dev/null 2>&1 || true
        wait "$qemu_pid" >/dev/null 2>&1 || true
    fi
}

trap cleanup EXIT

BUILD_IMAGE_OUTPUT_ISO="$ISO_PATH" bash ./scripts/build-image.sh
rm -f "$SERIAL_LOG"

qemu-system-x86_64 \
    -cdrom "$ISO_PATH" \
    -no-reboot \
    -display none \
    -serial "file:${SERIAL_LOG}" \
    -d int,cpu_reset \
    -D qemu.log &
qemu_pid=$!

boot_seen=0

for _ in $(seq 1 100); do
    if grep -Fq "$BOOT_MARKER" "$SERIAL_LOG" 2>/dev/null; then
        boot_seen=1
    fi

    if grep -Fq "$TIMER_MARKER" "$SERIAL_LOG" 2>/dev/null; then
        printf '%s\n' "TIMER PROOF OK"
        exit 0
    fi

    if ! kill -0 "$qemu_pid" >/dev/null 2>&1; then
        wait "$qemu_pid"
        break
    fi

    sleep 0.1
done

if [ "$boot_seen" -eq 1 ]; then
    printf '%s\n' "Timer proof failed: boot completed but missing serial marker '${TIMER_MARKER}'."
else
    printf '%s\n' "Timer proof failed: boot marker '${BOOT_MARKER}' was never observed."
fi

if [ -f "$SERIAL_LOG" ]; then
    cat "$SERIAL_LOG"
fi

exit 1
