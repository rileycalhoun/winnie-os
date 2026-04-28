#!/bin/bash
set -e

cargo build

cp target/x86_64-kernel/debug/winnie-os.elf iso/boot/winnie-os.elf

i686-elf-grub-mkrescue -o winnie.iso iso

qemu-system-x86_64 -cdrom winnie.iso -no-reboot -serial stdio -d int,cpu_reset -device isa-debug-exit,iobase=0xf4,iosize=0x04 -D qemu.log
