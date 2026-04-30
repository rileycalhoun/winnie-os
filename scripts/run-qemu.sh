#!/bin/bash
set -euo pipefail

ISO_PATH="$(mktemp "${TMPDIR:-/tmp}/winnie-run.XXXXXX.iso")"

cleanup() {
    rm -f "$ISO_PATH"
}

trap cleanup EXIT

BUILD_IMAGE_OUTPUT_ISO="$ISO_PATH" bash ./scripts/build-image.sh

qemu-system-x86_64 \
    -cdrom "$ISO_PATH" \
    -no-reboot \
    -serial stdio \
    -d int,cpu_reset \
    -D qemu.log
