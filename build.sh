#!/bin/bash
set -euo pipefail

ARTIFACT_PATH="${1:?expected kernel artifact path}"

bash ./scripts/build-image.sh "$ARTIFACT_PATH"

set +e
qemu-system-x86_64 \
    -cdrom winnie.iso \
    -no-reboot \
    -display none \
    -serial stdio \
    -d int,cpu_reset \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -D qemu.log
status=$?
set -e

# The isa-debug-exit device maps guest exit code 0x10 to host status 33 and
# guest exit code 0x11 to host status 35.
if [ "$status" -eq 33 ]; then
    exit 0
fi

if [ "$status" -eq 35 ]; then
    exit 1
fi

exit "$status"
