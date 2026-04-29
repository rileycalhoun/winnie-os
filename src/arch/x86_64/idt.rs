use core::mem::size_of;

use crate::{
    println,
    test_support::{
        DIVIDE_ERROR_MARKER, DOUBLE_FAULT_MARKER, GENERAL_PROTECTION_MARKER, INVALID_OPCODE_MARKER,
        PAGE_FAULT_MARKER,
    },
};

/// One 16-byte x86_64 interrupt descriptor table entry.
///
/// Winnie OS uses these entries during early higher-half kernel bring-up to map
/// selected exception vectors to fixed handler entrypoints. The current entries
/// all target the kernel code segment and either use no IST override or one of
/// the dedicated fault stacks established by bootstrap code.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct IdtEntry {
    offset_low: u16,  // handler address bits 0-15
    selector: u16,    // code segment selector (0x8 = our 64-bit CS)
    ist: u8,          // interrupt stack table offset (0 = none)
    attributes: u8,   // type + privilege + present
    offset_mid: u16,  // handler address bits 16-31
    offset_high: u32, // handler address bits 32-63
    _reserved: u32,   //
}

impl IdtEntry {
    pub const MISSING: Self = Self {
        offset_low: 0,
        selector: 0,
        ist: 0,
        attributes: 0,
        offset_mid: 0,
        offset_high: 0,
        _reserved: 0,
    };

    /// Builds a present interrupt gate for a 64-bit handler address.
    ///
    /// The current implementation always installs the handler into selector
    /// `0x8`, which is the kernel 64-bit code segment loaded by the bootstrap
    /// path before Rust runs in the higher half. It sets attributes to `0x8E`,
    /// meaning a present interrupt gate at privilege level 0.
    ///
    /// The `ist` argument is masked to the low three bits because x86_64 IDT
    /// entries only encode IST slots 0 through 7. Winnie OS currently relies on
    /// that field to preserve the architectural invariant that `#DF` uses IST1
    /// and `#PF` uses IST2.
    pub fn new(handler: u64, ist: u8) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector: 0x8,
            ist: ist & 0x7,
            attributes: 0x8E,
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: (handler >> 32) as u32,
            _reserved: 0,
        }
    }
}

/// The single 256-entry interrupt descriptor table used by the early kernel.
///
/// This table is populated during single-threaded higher-half bring-up before
/// interrupts are enabled. The current system installs only the exception
/// vectors needed for early fault visibility and keeps all fault paths minimal
/// and terminal through [`crate::hlt_loop`].
#[repr(C, align(16))]
pub struct Idt([IdtEntry; 256]);

impl Idt {
    /// Creates an IDT with every vector marked missing.
    ///
    /// This is used to initialize the single global IDT before selected
    /// exception vectors are populated during early kernel startup.
    pub const fn new() -> Self {
        Self([IdtEntry::MISSING; 256])
    }

    /// Installs a handler for `vector` without selecting an IST override.
    ///
    /// This is used for exceptions that currently run on the active kernel stack
    /// rather than a dedicated IST stack.
    pub fn set(&mut self, vector: usize, handler: u64) {
        self.0[vector] = IdtEntry::new(handler, 0)
    }

    /// Installs a handler for `vector` with a specific IST slot.
    ///
    /// The current initialization uses this to preserve two architectural
    /// invariants established by bootstrap code: `#DF` is routed through IST1
    /// and `#PF` is routed through IST2 so those destructive fault paths do not
    /// depend on the current kernel stack remaining usable.
    pub fn set_with_ist(&mut self, vector: usize, handler: u64, ist: u8) {
        self.0[vector] = IdtEntry::new(handler, ist)
    }

    /// Loads this table into the processor with `lidt`.
    ///
    /// This builds the packed 10-byte IDT descriptor expected by x86_64: a
    /// 16-bit limit covering the table size minus one and a 64-bit base address
    /// pointing at this `Idt`. Loading is safe once the kernel is executing in
    /// the higher half with the IDT memory mapped and stable, and after any
    /// vectors the kernel depends on have been installed.
    pub fn load(&self) {
        let descriptor = IdtDescriptor {
            limit: (size_of::<Idt>() - 1) as u16,
            base: self as *const _ as u64,
        };

        // Sound because `descriptor` points to a valid in-scope IDT descriptor for this call.
        unsafe { core::arch::asm!("lidt [{}]", in(reg) &descriptor) }
    }
}

#[repr(C, packed)]
struct IdtDescriptor {
    limit: u16,
    base: u64,
}

/// The stack frame pushed by the CPU for x86_64 interrupt and exception entry.
///
/// Handler signatures in this file match the architecture's exception calling
/// convention exactly so the CPU-supplied frame layout remains correct.
#[repr(C)]
pub struct InterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/*
 * No Error Codes
 */

/// Handles vector 0, the divide-error exception.
///
/// The CPU does not supply an error code for this exception, so the handler
/// takes only an [`InterruptFrame`]. The implementation is intentionally
/// minimal: it prints a fixed message and terminates in [`crate::hlt_loop`],
/// which keeps early destructive fault handling simple and auditable in the
/// higher-half kernel.
pub extern "x86-interrupt" fn divide_error_handler(_frame: &InterruptFrame) -> ! {
    println!("{}", DIVIDE_ERROR_MARKER);
    crate::hlt_loop()
}

/// Handles vector 6, the invalid-opcode exception.
///
/// The CPU does not supply an error code for this exception. The current path
/// avoids complex recovery and instead emits a fixed message. Under the
/// dedicated invalid-opcode test scenario, reaching this handler is treated as
/// success and exits QEMU with the passing test code. The bootable test harness
/// reports failure here, while normal builds retain the minimal terminal halt
/// path.
pub extern "x86-interrupt" fn invalid_opcode_handler(_frame: &InterruptFrame) -> ! {
    println!("{}", INVALID_OPCODE_MARKER);
    match crate::test_support::selected_boot_scenario() {
        crate::test_support::BootScenario::InvalidOpcode => {
            crate::test_support::exit_qemu(crate::test_support::QemuExitCode::Success)
        }
        crate::test_support::BootScenario::TestHarness => {
            crate::test_support::report_test_case_fail()
        }
        _ => crate::hlt_loop(),
    }
}

/*
 * With Error Codes
 */

/// Handles vector 8, the double-fault exception.
///
/// The CPU architecturally supplies an error code for `#DF`, and this handler's
/// signature reflects that requirement exactly. Winnie OS installs `#DF` with
/// IST1 so the handler runs on the dedicated double-fault stack established by
/// bootstrap code, which avoids relying on a possibly corrupted current stack.
/// Under the dedicated double-fault test scenario, reaching this handler is
/// treated as success and exits QEMU with the passing test code. The bootable
/// test harness reports failure here, while normal builds retain the minimal
/// terminal halt path after printing the fixed marker.
pub extern "x86-interrupt" fn double_fault_handler(_frame: &InterruptFrame, _error_code: u64) -> ! {
    println!("{}", DOUBLE_FAULT_MARKER);
    match crate::test_support::selected_boot_scenario() {
        crate::test_support::BootScenario::DoubleFault => {
            crate::test_support::exit_qemu(crate::test_support::QemuExitCode::Success)
        }
        crate::test_support::BootScenario::TestHarness => {
            crate::test_support::report_test_case_fail()
        }
        _ => crate::hlt_loop(),
    }
}

/// Handles vector 13, the general-protection exception.
///
/// The CPU architecturally supplies an error code for `#GP`, so this handler
/// accepts one even though the current implementation only reports a fixed
/// message. Under the dedicated general-protection test scenario, reaching this
/// handler is treated as success and exits QEMU with the passing test code. The
/// bootable test harness reports failure here, while normal builds retain the
/// minimal terminal halt path rather than attempting recovery from compromised
/// kernel state.
pub extern "x86-interrupt" fn general_protection_handler(
    _frame: &InterruptFrame,
    _error_code: u64,
) -> ! {
    println!("{}", GENERAL_PROTECTION_MARKER);
    match crate::test_support::selected_boot_scenario() {
        crate::test_support::BootScenario::GeneralProtection => {
            crate::test_support::exit_qemu(crate::test_support::QemuExitCode::Success)
        }
        crate::test_support::BootScenario::TestHarness => {
            crate::test_support::report_test_case_fail()
        }
        _ => crate::hlt_loop(),
    }
}

/// Handles vector 14, the page-fault exception.
///
/// The CPU architecturally supplies an error code for `#PF`, and Winnie OS
/// installs this handler with IST2 so page faults do not depend on the current
/// kernel stack remaining usable. This preserves the current fault-handling
/// invariant that `#PF` runs on its dedicated IST stack. Under the dedicated
/// page-fault test scenario, reaching this handler is treated as success and
/// exits QEMU with the passing test code. The bootable test harness reports
/// failure here, while normal builds retain the minimal terminal halt path
/// after printing the fixed marker.
pub extern "x86-interrupt" fn page_fault_handler(_frame: &InterruptFrame, _error_code: u64) -> ! {
    println!("{}", PAGE_FAULT_MARKER);

    match crate::test_support::selected_boot_scenario() {
        crate::test_support::BootScenario::PageFault => {
            crate::test_support::exit_qemu(crate::test_support::QemuExitCode::Success)
        }
        crate::test_support::BootScenario::TestHarness => {
            crate::test_support::report_test_case_fail()
        }
        _ => crate::hlt_loop(),
    }
}

static mut IDT: Idt = Idt::new();

/// Populates and loads the single global IDT used during early kernel startup.
///
/// This function runs after the architecture bootstrap code has transferred
/// control into the higher-half kernel and before interrupts are enabled. It
/// installs the currently supported exception vectors: divide error (`0`),
/// invalid opcode (`6`), double fault (`8`) on IST1, general protection (`13`),
/// and page fault (`14`) on IST2.
///
/// Initialization uses the one global IDT because the kernel is still in a
/// single-threaded bring-up phase with exclusive access to that table. The
/// configured handlers all keep fault paths minimal and terminal through
/// [`crate::hlt_loop`], which matches the current early-kernel design.
pub fn init() {
    // Sound because early init has exclusive access to the single global IDT before interrupts are enabled.
    let idt = &raw mut IDT;
    // Sound because `idt` is the only mutable reference to the global IDT for this initialization sequence.
    unsafe { (*idt).set(0, divide_error_handler as *const () as u64) };
    // Sound because `idt` is the only mutable reference to the global IDT for this initialization sequence.
    unsafe { (*idt).set(6, invalid_opcode_handler as *const () as u64) };
    // Sound because `idt` is the only mutable reference to the global IDT for this initialization sequence.
    unsafe { (*idt).set_with_ist(8, double_fault_handler as *const () as u64, 1) };
    // Sound because `idt` is the only mutable reference to the global IDT for this initialization sequence.
    unsafe { (*idt).set(13, general_protection_handler as *const () as u64) };
    // Sound because `idt` is the only mutable reference to the global IDT for this initialization sequence.
    unsafe { (*idt).set_with_ist(14, page_fault_handler as *const () as u64, 2) };
    // Sound because `idt` still points to the initialized static IDT for the duration of this call.
    unsafe { (*idt).load() };
}

/// Removes the page-fault handler entry so a deliberate `#PF` escalates to
/// `#DF` during the dedicated double-fault test scenario.
pub fn clear_page_fault_handler_for_double_fault_test() {
    // Sound because this runs during single-threaded early bring-up for a
    // dedicated destructive test scenario, with exclusive access to the IDT.
    let idt = &raw mut IDT;

    // Sound because `idt` is the loaded global IDT and vector 14 is
    // intentionally cleared only for the dedicated double-fault test scenario.
    unsafe {
        (*idt).0[14] = IdtEntry::MISSING;
    }
}
