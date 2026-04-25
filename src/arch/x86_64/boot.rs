use core::arch::global_asm;

global_asm!(
    r#"
        .equ KERNEL_STACK_TOP, 0xffffffff80203000
        .equ PF_IST_STACK_TOP,  0xffffffff80206000
        .equ DF_IST_STACK_TOP,  0xffffffff80208000

        .extern __kernel_phys_start
        .extern __kernel_phys_end
        .extern __kernel_virt_start
        .extern __kernel_virt_end

        .section .multiboot, "a"
        .align 8
        .long 0xe85250d6
        .long 0
        .long 24
        .long 0x100000000 - (0xe85250d6 + 0 + 24)
        .short 0
        .short 0
        .long 8

        .section .boot.bss, "aw", @nobits
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
        kernel_stack_page0:
            .skip 4096
        kernel_stack_page1:
            .skip 4096
        pf_ist_stack_page:
            .skip 4096
        df_ist_stack_page:
            .skip 4096

        .align 4096
        p4_table:
            .skip 4096

        .align 4096
        p3_low:
            .skip 4096
        p2_low:
            .skip 4096
        p1_low:
            .skip 4096

        .align 4096
        p3_high:
            .skip 4096
        p2_high:
            .skip 4096
        p1_high_kernel:
            .skip 4096

        .align 4096
        p1_high_stack:
            .skip 4096


        .section .boot.text, "ax"
        .code32
        .global _start
        _start:
            mov esp, OFFSET stack_top
            lgdt [gdt64_descriptor32]

            mov eax, OFFSET p3_low
            or eax, 0x3
            mov [p4_table], eax

            mov eax, OFFSET p2_low
            or eax, 0x3
            mov [p3_low], eax

            mov eax, OFFSET p1_low
            or eax, 0x3
            mov [p2_low], eax

            xor ecx, ecx
        1:
            mov eax, ecx
            shl eax, 12
            or eax, 0x3
            mov [p1_low + ecx * 8], eax
            inc ecx
            cmp ecx, 512
            jne 1b

            mov eax, OFFSET p3_high
            or eax, 0x3
            mov [p4_table + 8 * 511], eax

            mov eax, OFFSET p2_high
            or eax, 0x3
            mov [p3_high + 8 * 510], eax

            mov eax, OFFSET p1_high_kernel
            or eax, 0x3
            mov [p2_high + 8 * 0], eax

            mov eax, OFFSET p1_high_stack
            or eax, 0x3
            mov [p2_high + 8 * 1], eax

            mov esi, OFFSET __kernel_phys_start
            mov edi, OFFSET __kernel_phys_end

            mov ecx, OFFSET __kernel_phys_start
            and ecx, 0x1ff000
            shr ecx, 12

            sub edi, esi
            add edi, 0x0fff
            shr edi, 12

        2:
            cmp edi, 0
            je 3f

            mov eax, esi
            or eax, 0x3
            mov [p1_high_kernel + ecx * 8], eax

            add esi, 0x1000
            inc ecx
            dec edi
            jmp 2b

        3:
            mov eax, OFFSET kernel_stack_page0
            or eax, 0x3
            mov [p1_high_stack + 8 * 1], eax

            mov eax, OFFSET kernel_stack_page1
            or eax, 0x3
            mov [p1_high_stack + 8 * 2], eax

            mov eax, OFFSET pf_ist_stack_page
            or eax, 0x3
            mov [p1_high_stack + 8 * 5], eax

            mov eax, OFFSET df_ist_stack_page
            or eax, 0x3
            mov [p1_high_stack + 8 * 7], eax

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

            mov rax, DF_IST_STACK_TOP
            mov [tss64 + 0x24], rax

            mov rax, PF_IST_STACK_TOP
            mov [tss64 + 0x2c], rax

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

            mov rsp, KERNEL_STACK_TOP

            mov rax, OFFSET kernel_main_high
            jmp rax
        .halt:
            hlt
            jmp .halt

        .section .boot.rodata, "a"
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
