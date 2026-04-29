#!/bin/bash
set -euo pipefail

ARTIFACT_PATH="${1:-target/x86_64-kernel/debug/winnie-os.elf}"

if [ "$#" -eq 0 ]; then
    cargo build
fi

cp "$ARTIFACT_PATH" iso/boot/winnie-os.elf
i686-elf-grub-mkrescue -o winnie.iso iso
