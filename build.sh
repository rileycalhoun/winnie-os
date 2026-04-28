#!/bin/bash
set -euo pipefail

ARTIFACT_PATH="${1:-target/x86_64-kernel/debug/winnie-os.elf}"
IS_TEST_RUN=0

if [ "$#" -eq 0 ]; then
	cargo build
else
	IS_TEST_RUN=1
fi

cp "$ARTIFACT_PATH" iso/boot/winnie-os.elf

i686-elf-grub-mkrescue -o winnie.iso iso

if [ "$IS_TEST_RUN" -eq 1 ]; then
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

	if [ "$status" -eq 33 ]; then
		exit 0
	fi

	if [ "$status" -eq 35 ]; then
		exit 1
	fi

	exit "$status"
fi

qemu-system-x86_64 \
	-cdrom winnie.iso \
	-no-reboot \
	-serial stdio \
	-d int,cpu_reset \
	-D qemu.log
