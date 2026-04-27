use core::arch::global_asm;

global_asm!(
    r#"
        .equ PAGE_PRESENT_WRITABLE, 0x3              # present + writable in early boot page tables
        .equ PAGE_SHIFT, 12                          # 4 KiB pages
        .equ PAGE_SIZE, 0x1000
        .equ PAGE_ALIGN_MASK, PAGE_SIZE - 1
        .equ PAGE_TABLE_ENTRY_COUNT, 512             # entries per x86_64 page-table page
        .equ P1_INDEX_MASK_WITHIN_2M, 0x1ff000      # bits 12..20 select a P1 slot inside one 2 MiB window

        .equ P4_LOW_SLOT, 0                         # identity map for early bootstrap code/data
        .equ P4_HIGHER_HALF_SLOT, 511               # canonical top-level slot for higher-half kernel addresses
        .equ P3_HIGHER_HALF_SLOT, 510               # P3 slot covering 0xffffffff80000000..
        .equ P2_KERNEL_WINDOW_SLOT, 0               # first higher-half 2 MiB window holds the kernel image
        .equ P2_STACK_WINDOW_SLOT, 1                # second higher-half 2 MiB window holds stacks and guards
        .equ P1_KERNEL_STACK_PAGE0_SLOT, 1          # first mapped kernel stack page at 0xffffffff80201000
        .equ P1_KERNEL_STACK_PAGE1_SLOT, 2          # second mapped kernel stack page at 0xffffffff80202000
        .equ P1_PF_IST_STACK_SLOT, 5                # page-fault IST page at 0xffffffff80205000
        .equ P1_DF_IST_STACK_SLOT, 7                # double-fault IST page at 0xffffffff80207000

        .equ KERNEL_STACK_TOP, 0xffffffff80203000   # top of the two-page kernel stack; slot 0 below it stays unmapped
        .equ PF_IST_STACK_TOP,  0xffffffff80206000  # top of IST2, used by #PF
        .equ DF_IST_STACK_TOP,  0xffffffff80208000  # top of IST1, used by #DF

        .equ CR0_PE_BIT, (1 << 0)                   # protected mode enable
        .equ CR0_MP_BIT, (1 << 1)                   # monitor coprocessor
        .equ CR0_EM_BIT, (1 << 2)                   # x87 emulation; clear before fninit
        .equ CR0_PG_BIT, (1 << 31)                  # paging enable
        .equ CR4_PAE_BIT, (1 << 5)                  # required for long mode page translation
        .equ CR4_OSFXSR_BIT, (1 << 9)               # OS supports FXSAVE/FXRSTOR
        .equ CR4_OSXMMEXCPT_BIT, (1 << 10)          # OS handles SIMD floating-point exceptions
        .equ EFER_MSR, 0xC0000080                   # IA32_EFER MSR
        .equ EFER_LME_BIT, (1 << 8)                 # long mode enable

        .equ KERNEL_CODE_SELECTOR, 0x8              # GDT entry 1, ring-0 64-bit code segment
        .equ TSS_SELECTOR, 0x18                     # GDT entry 3, available 64-bit TSS descriptor
        .equ TSS64_SIZE, 104                        # bytes in the 64-bit TSS
        .equ TSS_IST1_OFFSET, 0x24                  # IST1 stack pointer field in the TSS (#DF)
        .equ TSS_IST2_OFFSET, 0x2c                  # IST2 stack pointer field in the TSS (#PF)
        .equ TSS_IOPB_OFFSET_FIELD, 0x66            # I/O bitmap base field in the TSS
        .equ TSS_IOPB_DISABLED_OFFSET, TSS64_SIZE   # point the I/O bitmap just past the TSS to disable it

        .equ GDT_TSS_DESCRIPTOR_OFFSET, 24          # byte offset after null/code/data descriptors
        .equ GDT_TSS_BASE_LOW16_OFFSET, 2           # TSS base bits 0..15 inside the descriptor
        .equ GDT_TSS_BASE_MID8_OFFSET, 4            # TSS base bits 16..23
        .equ GDT_TSS_BASE_HIGH8_OFFSET, 7           # TSS base bits 24..31
        .equ GDT_TSS_BASE_UPPER32_OFFSET, 8         # TSS base bits 32..63
        .equ GDT_KERNEL_CODE64_DESCRIPTOR, 0x00af9a000000ffff  # ring-0 execute/read code segment, long-mode, 4 KiB granularity
        .equ GDT_KERNEL_DATA_DESCRIPTOR, 0x00af92000000ffff    # ring-0 writable data segment, 4 KiB granularity
        .equ GDT_TSS64_ACCESS_BYTE, 0x89                       # present available 64-bit TSS descriptor type

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
            .skip TSS64_SIZE
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

        # preserve the Multiboot2 loader handoff outside the small Rust kernel stack
        .align 8
        multiboot_magic_slot:
            .skip 8
        multiboot_info_ptr_slot:
            .skip 8

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

            # preserve the Multiboot2 loader contract across bootstrap so Rust can validate and parse it later
            mov [multiboot_magic_slot], eax
            mov [multiboot_info_ptr_slot], ebx

            mov eax, OFFSET p3_low
            or eax, PAGE_PRESENT_WRITABLE
            mov [p4_table + 8 * P4_LOW_SLOT], eax

            mov eax, OFFSET p2_low
            or eax, PAGE_PRESENT_WRITABLE
            mov [p3_low], eax

            mov eax, OFFSET p1_low
            or eax, PAGE_PRESENT_WRITABLE
            mov [p2_low], eax

            xor ecx, ecx
        1:
            mov eax, ecx
            shl eax, PAGE_SHIFT
            or eax, PAGE_PRESENT_WRITABLE
            mov [p1_low + ecx * 8], eax
            inc ecx
            cmp ecx, PAGE_TABLE_ENTRY_COUNT
            jne 1b

            mov eax, OFFSET p3_high
            or eax, PAGE_PRESENT_WRITABLE
            mov [p4_table + 8 * P4_HIGHER_HALF_SLOT], eax

            mov eax, OFFSET p2_high
            or eax, PAGE_PRESENT_WRITABLE
            mov [p3_high + 8 * P3_HIGHER_HALF_SLOT], eax

            mov eax, OFFSET p1_high_kernel
            or eax, PAGE_PRESENT_WRITABLE
            mov [p2_high + 8 * P2_KERNEL_WINDOW_SLOT], eax

            mov eax, OFFSET p1_high_stack
            or eax, PAGE_PRESENT_WRITABLE
            mov [p2_high + 8 * P2_STACK_WINDOW_SLOT], eax

            mov esi, OFFSET __kernel_phys_start
            mov edi, OFFSET __kernel_phys_end

            mov ecx, OFFSET __kernel_phys_start
            and ecx, P1_INDEX_MASK_WITHIN_2M
            shr ecx, PAGE_SHIFT

            sub edi, esi
            add edi, PAGE_ALIGN_MASK
            shr edi, PAGE_SHIFT

        2:
            cmp edi, 0
            je 3f

            mov eax, esi
            or eax, PAGE_PRESENT_WRITABLE
            mov [p1_high_kernel + ecx * 8], eax

            add esi, PAGE_SIZE
            inc ecx
            dec edi
            jmp 2b

        3:
            mov eax, OFFSET kernel_stack_page0
            or eax, PAGE_PRESENT_WRITABLE
            mov [p1_high_stack + 8 * P1_KERNEL_STACK_PAGE0_SLOT], eax

            mov eax, OFFSET kernel_stack_page1
            or eax, PAGE_PRESENT_WRITABLE
            mov [p1_high_stack + 8 * P1_KERNEL_STACK_PAGE1_SLOT], eax

            mov eax, OFFSET pf_ist_stack_page
            or eax, PAGE_PRESENT_WRITABLE
            mov [p1_high_stack + 8 * P1_PF_IST_STACK_SLOT], eax

            mov eax, OFFSET df_ist_stack_page
            or eax, PAGE_PRESENT_WRITABLE
            mov [p1_high_stack + 8 * P1_DF_IST_STACK_SLOT], eax

            mov eax, OFFSET p4_table
            mov cr3, eax

            mov eax, cr0
            and eax, ~CR0_EM_BIT
            or eax, CR0_MP_BIT
            mov cr0, eax

            mov eax, cr4
            or eax, CR4_PAE_BIT
            or eax, CR4_OSFXSR_BIT
            or eax, CR4_OSXMMEXCPT_BIT
            mov cr4, eax

            fninit

            mov ecx, EFER_MSR
            rdmsr
            or eax, EFER_LME_BIT
            wrmsr

            mov eax, cr0
            or eax, (CR0_PE_BIT | CR0_PG_BIT)
            mov cr0, eax

            push KERNEL_CODE_SELECTOR
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
            mov [tss64 + TSS_IST1_OFFSET], rax

            mov rax, PF_IST_STACK_TOP
            mov [tss64 + TSS_IST2_OFFSET], rax

            mov ax, TSS_IOPB_DISABLED_OFFSET
            mov word ptr [tss64 + TSS_IOPB_OFFSET_FIELD], ax

            mov rax, OFFSET tss64

            mov word ptr [gdt64 + GDT_TSS_DESCRIPTOR_OFFSET + GDT_TSS_BASE_LOW16_OFFSET], ax
            shr rax, 16
            mov byte ptr [gdt64 + GDT_TSS_DESCRIPTOR_OFFSET + GDT_TSS_BASE_MID8_OFFSET], al
            shr rax, 8
            mov byte ptr [gdt64 + GDT_TSS_DESCRIPTOR_OFFSET + GDT_TSS_BASE_HIGH8_OFFSET], al
            shr rax, 8
            mov dword ptr [gdt64 + GDT_TSS_DESCRIPTOR_OFFSET + GDT_TSS_BASE_UPPER32_OFFSET], eax

            lgdt [gdt64_descriptor64]

            mov ax, TSS_SELECTOR
            ltr ax

            mov rsp, KERNEL_STACK_TOP

            # hand Multiboot2 magic and info pointer to Rust using the x86_64 SysV calling convention
            mov edi, dword ptr [multiboot_magic_slot]
            mov esi, dword ptr [multiboot_info_ptr_slot]

            mov rax, OFFSET kernel_main_high
            jmp rax
        .halt:
            hlt
            jmp .halt

        .section .boot.rodata, "a"
        .align 8
        gdt64:
            .quad 0
            .quad GDT_KERNEL_CODE64_DESCRIPTOR
            .quad GDT_KERNEL_DATA_DESCRIPTOR

            .word TSS64_SIZE - 1
            .word 0
            .byte 0
            .byte GDT_TSS64_ACCESS_BYTE
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
