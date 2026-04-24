use core::mem::size_of;

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

    pub fn new(handler: u64) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector: 0x8,
            ist: 0,
            attributes: 0x8E,
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: (handler >> 32) as u32,
            _reserved: 0,
        }
    }
}

#[repr(C, align(16))]
pub struct Idt([IdtEntry; 256]);

impl Idt {
    pub const fn new() -> Self {
        Self([IdtEntry::MISSING; 256])
    }

    pub fn set(&mut self, vector: usize, handler: u64) {
        self.0[vector] = IdtEntry::new(handler)
    }

    pub fn load(&self) {
        let descriptor = IdtDescriptor {
            limit: (size_of::<Idt>() - 1) as u16,
            base: self as *const _ as u64,
        };

        unsafe { core::arch::asm!("lidt [{}]", in(reg) &descriptor) }
    }
}

#[repr(C, packed)]
struct IdtDescriptor {
    limit: u16,
    base: u64,
}

#[repr(C)]
pub struct InterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/**
 * No Error Codes
 */

pub extern "x86-interrupt" fn divide_error_handler(frame: &InterruptFrame) -> ! {
    panic!("DIVIDE ERROR at {:#x}", frame.rip)
}

pub extern "x86-interrupt" fn invalid_opcode_handler(frame: &InterruptFrame) -> ! {
    panic!("INVALID OPCODE at {:#x}", frame.rip)
}

/**
 * With Error Codes
 */

pub extern "x86-interrupt" fn double_fault_handler(frame: &InterruptFrame, error_code: u64) -> ! {
    panic!("DOUBLE FAULT (code={:#x}) at {:#x}", error_code, frame.rip)
}

pub extern "x86-interrupt" fn general_protection_handler(
    frame: &InterruptFrame,
    error_code: u64,
) -> ! {
    panic!(
        "GENERAL PROTECTION FAULT (code={:#x}) at {:#x}",
        error_code, frame.rip
    )
}

pub extern "x86-interrupt" fn page_fault_handler(frame: &InterruptFrame, error_code: u64) -> ! {
    panic!("PAGE FAULT (code={:#x}) at {:#x}", error_code, frame.rip)
}

static mut IDT: Idt = Idt::new();

pub fn init() {
    unsafe {
        let idt = &raw mut IDT;
        (*idt).set(0, divide_error_handler as *const () as u64);
        (*idt).set(6, invalid_opcode_handler as *const () as u64);
        (*idt).set(8, double_fault_handler as *const () as u64);
        (*idt).set(13, general_protection_handler as *const () as u64);
        (*idt).set(14, page_fault_handler as *const () as u64);
        (*idt).load();
    }
}
