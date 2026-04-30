ENTRY(_start)

KERNEL_VIRT_BASE = 0xffffffff80000000;

SECTIONS {
    . = 0x00100000;
    __boot_phys_start = .;

    .multiboot ALIGN(8) : {
        KEEP(*(.multiboot))
    }

    .boot.text ALIGN(4096) : {
        *(.boot.text .boot.text.*)
    }

    .boot.rodata ALIGN(4096) : {
        *(.boot.rodata .boot.rodata.*)
    }

    .boot.data ALIGN(4096) : {
        *(.boot.data .boot.data.*)
    }

    .boot.bss ALIGN(4096) : {
        *(COMMON)
        *(.boot.bss .boot.bss.*)
    }

    __boot_phys_end = ALIGN(., 4096);
    __boot_end = __boot_phys_end;

    . = __boot_end + KERNEL_VIRT_BASE;

    __kernel_virt_start = .;
    __kernel_phys_start = __kernel_virt_start - KERNEL_VIRT_BASE;

    .text ALIGN(4096) : AT(ADDR(.text) - KERNEL_VIRT_BASE) {
        *(.text .text.*)
    }

    .rodata ALIGN(4096) : AT(ADDR(.rodata) - KERNEL_VIRT_BASE) {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4096) : AT(ADDR(.data) - KERNEL_VIRT_BASE) {
        *(.data .data.*)
    }

    .bss ALIGN(4096) : AT(ADDR(.bss) - KERNEL_VIRT_BASE) {
        *(.bss .bss.*)
    }

    __text_start = ADDR(.text);
    __text_end = ADDR(.text) + SIZEOF(.text);

    __rodata_start = ADDR(.rodata);
    __rodata_end = ADDR(.rodata) + SIZEOF(.rodata);

    __data_start = ADDR(.data);
    __data_end = ADDR(.data) + SIZEOF(.data);

    __bss_start = ADDR(.bss);
    __bss_end = ADDR(.bss) + SIZEOF(.bss);

    __kernel_virt_end = .;
    __kernel_phys_end = __kernel_virt_end - KERNEL_VIRT_BASE;
}
