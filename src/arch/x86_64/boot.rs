use core::arch::global_asm;

global_asm!(
    r#"
        .section .multiboot, "a"
        .align 8
        .long 0xe85250d6
        .long 0
        .long 24
        .long 0x100000000 - (0xe85250d6 + 0 + 24)
        .short 0
        .short 0
        .long 8

        .section .bss
        .align 16
        stack_bottom:
            .skip 16384
        stack_top:
        .align 4096
        p4_table:
            .skip 4096
        p3_table:
            .skip 4096
        p2_table:
            .skip 4096

        .section .text
        .code32
        .global _start
        _start:
            mov esp, OFFSET stack_top
            lgdt [gdt64_descriptor]

            mov eax, OFFSET p3_table
            or eax, 0x3
            mov [p4_table], eax

            mov eax, OFFSET p2_table
            or eax, 0x3
            mov [p3_table], eax

            mov dword ptr [p2_table], 0x83

            mov eax, OFFSET p4_table
            mov cr3, eax

            mov eax, cr4
            or eax, 0x20
            mov cr4, eax

            mov ecx, 0xC0000080
            rdmsr
            or eax, 0x100
            wrmsr

            mov eax, cr0
            or eax, 0x80000001
            mov cr0, eax

            push 0x8
            mov eax, OFFSET start64
            push eax
            retf

        .code64
        start64:
            mov ax, 0
            mov ds, ax
            mov es, ax
            mov fs, ax
            mov gs, ax
            mov ss, ax
            call kernel_main
        .halt:
            hlt
            jmp .halt

        .section .rodata
        .align 8
        gdt64:
            .quad 0
            .quad 0x00af9a000000ffff
            .quad 0x00af92000000ffff
        gdt64_end:

        gdt64_descriptor:
            .word gdt64_end - gdt64 - 1
            .long gdt64
        "#
);
