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

        .section .boot.bss
        .align 16
        stack_bottom:
            .skip 524288
        stack_top:

        .align 16
        double_fault_stack_bottom:
            .skip 16384
        double_fault_stack_top:

        .align 16
        tss64:
            .skip 104
        tss64_end:

        .align 4096
        p4_table:
            .skip 4096
        p3_table:
            .skip 4096
        p2_table:
            .skip 4096

        .section .boot.text
        .code32
        .global _start
        _start:
            mov esp, OFFSET stack_top
            lgdt [gdt64_descriptor32]

            mov eax, OFFSET p3_table
            or eax, 0x3
            mov [p4_table], eax

            mov eax, OFFSET p2_table
            or eax, 0x3
            mov [p3_table], eax

            mov dword ptr [p2_table], 0x83
            mov dword ptr [p2_table + 8], 0x200083

            mov eax, OFFSET p4_table
            mov cr3, eax

            mov eax, cr0
            and eax, ~(1 << 2)
            or eax, (1 << 1)
            mov cr0, eax

            mov eax, cr4
            or eax, (1 << 5)
            or eax, (1 << 9)
            or eax, (1 << 10)
            mov cr4, eax

            fninit

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

            mov rax, OFFSET double_fault_stack_top
            mov [tss64 + 0x24], rax
            mov ax, 104
            mov word ptr [tss64 + 0x66], ax

            mov rax, OFFSET tss64

            mov word ptr [gdt64 + 24 + 2], ax
            shr rax, 16
            mov byte ptr [gdt64 + 24 + 4], al
            shr rax, 8
            mov byte ptr [gdt64 + 24 + 7], al
            shr rax, 8
            mov dword ptr [gdt64 + 24 + 8], eax

            lgdt [gdt64_descriptor64]

            mov ax, 0x18
            ltr ax
            mov rax, OFFSET kernel_main_high
            jmp .halt
        .halt:
            hlt
            jmp .halt

        .section .boot.rodata
        .align 8
        gdt64:
            .quad 0
            .quad 0x00af9a000000ffff
            .quad 0x00af92000000ffff

            .word 104 - 1
            .word 0
            .byte 0
            .byte 0x89
            .byte 0x00
            .byte 0
            .long 0
            .long 0
        gdt64_end:

        gdt64_descriptor32:
            .word gdt64_end - gdt64 - 1
            .long gdt64

        gdt64_descriptor64:
            .word gdt64_end - gdt64 - 1
            .quad gdt64
        "#
);
