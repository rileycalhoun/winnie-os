ENTRY(_start)

SECTIONS {
    . = 0x100000;

    /* Multiboot2 header must be within the first 32KB of the image */
    .multiboot : {
        KEEP(*(.multiboot))
    }

    .text : ALIGN(4096) {
        *(.text .text.*)
    }

    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4096) {
        *(.data .data.*)
    }

    .bss : ALIGN(4096) {
        *(COMMON)
        *(.bss .bss.*)
    }
}
