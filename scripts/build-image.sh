#!/bin/bash
set -euo pipefail

ARTIFACT_PATH="${1:-target/x86_64-kernel/debug/winnie-os.elf}"
ISO_PATH="${BUILD_IMAGE_OUTPUT_ISO:-winnie.iso}"

if [ "$#" -eq 0 ]; then
    cargo build
fi

cp "$ARTIFACT_PATH" iso/boot/winnie-os.elf
i686-elf-grub-mkrescue -o "$ISO_PATH" iso
