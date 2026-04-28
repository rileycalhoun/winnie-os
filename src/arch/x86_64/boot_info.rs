use crate::boot_info::{BootInfo, BootInfoError, MemoryRegion, MemoryRegionKind};

const MULTIBOOT2_MAGIC: u32 = 0x36d7_6289;
const TAG_TYPE_END: u32 = 0;
const TAG_TYPE_MEMORY_MAP: u32 = 6;
const MULTIBOOT_TAG_ALIGN: usize = 8;

/// Reports why Multiboot2 boot-info parsing could not produce an owned `BootInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    BadMagic(u32),
    MissingMemoryMap,
    TooManyRegions,
    TruncatedInfo,
    TruncatedTag,
    InvalidTagSize(u32),
    UnsupportedMemoryMapEntrySize(u32),
}

#[repr(C)]
/// Fixed header at the start of the Multiboot2 information block.
struct MultibootInfoHeader {
    total_size: u32,
    _reserved: u32,
}

#[repr(C)]
/// Common header present at the start of every Multiboot2 tag.
struct MultibootTag {
    typ: u32,
    size: u32,
}

#[repr(C)]
/// Multiboot2 memory-map tag header that prefixes the entry array payload.
struct MemoryMapTag {
    typ: u32,
    size: u32,
    entry_size: u32,
    entry_version: u32,
}

#[repr(C)]
/// One raw Multiboot2 memory-map entry in the Phase 0-supported layout.
struct MemoryMapEntry {
    base_addr: u64,
    length: u64,
    typ: u32,
    _reserved: u32,
}

/// Parses the Multiboot2 boot-information block into the owned kernel representation.
///
/// Phase 0 only extracts the memory map. Other tags are skipped after basic
/// bounds validation so the parser stays narrow and easy to audit.
pub fn parse_multiboot2(
    magic: u32,
    info_addr: usize,
    boot_info: &mut BootInfo,
) -> Result<(), ParseError> {
    if magic != MULTIBOOT2_MAGIC {
        return Err(ParseError::BadMagic(magic));
    }

    if info_addr == 0 {
        return Err(ParseError::TruncatedInfo);
    }

    // Sound because the Multiboot2 loader contract guarantees `info_addr`
    // points at a readable boot information block while the early identity
    // mapping is still active.
    let header = unsafe { &*(info_addr as *const MultibootInfoHeader) };

    let total_size = header.total_size as usize;
    if total_size < core::mem::size_of::<MultibootInfoHeader>() {
        return Err(ParseError::TruncatedInfo);
    }

    let info_end = info_addr
        .checked_add(total_size)
        .ok_or(ParseError::TruncatedTag)?;

    // Reset the owned destination so repeated parser calls, if any, never
    // preserve stale region data from an earlier attempt.
    *boot_info = BootInfo::new();
    let mut found_memory_map = false;

    let mut current = info_addr
        .checked_add(core::mem::size_of::<MultibootInfoHeader>())
        .ok_or(ParseError::TruncatedTag)?;

    if current > info_end {
        return Err(ParseError::TruncatedTag);
    }

    while current < info_end {
        let tag_header_end = current
            .checked_add(core::mem::size_of::<MultibootTag>())
            .ok_or(ParseError::TruncatedTag)?;

        if tag_header_end > info_end {
            return Err(ParseError::TruncatedTag);
        }

        // Sound because `current..tag_header_end` was validated to lie fully
        // within the reported Multiboot2 info block before reading the tag header.
        let tag = unsafe { &*(current as *const MultibootTag) };
        if tag.size < core::mem::size_of::<MultibootTag>() as u32 {
            return Err(ParseError::InvalidTagSize(tag.size));
        }

        let tag_end = current
            .checked_add(tag.size as usize)
            .ok_or(ParseError::TruncatedTag)?;

        if tag_end > info_end {
            return Err(ParseError::TruncatedTag);
        }

        match tag.typ {
            TAG_TYPE_END => break,
            TAG_TYPE_MEMORY_MAP => {
                parse_memory_map_tag(current, tag.size as usize, boot_info)?;
                found_memory_map = true;
            }
            _ => {
                // Phase 0 ignores unrelated Multiboot2 tags after bounds validation.
            }
        }

        current = align_up(tag_end, MULTIBOOT_TAG_ALIGN);
    }

    if !found_memory_map {
        return Err(ParseError::MissingMemoryMap);
    }

    Ok(())
}

/// Parses one Multiboot2 memory-map tag into the owned boot-info structure.
fn parse_memory_map_tag(
    tag_addr: usize,
    tag_size: usize,
    boot_info: &mut BootInfo,
) -> Result<(), ParseError> {
    if tag_size < core::mem::size_of::<MemoryMapTag>() {
        return Err(ParseError::TruncatedTag);
    }

    // Sound because the caller already validated that this tag lies within the
    // reported Multiboot2 info block and is large enough for `MemoryMapTag`.
    let tag = unsafe { &*(tag_addr as *const MemoryMapTag) };
    if tag.entry_size as usize != core::mem::size_of::<MemoryMapEntry>() {
        return Err(ParseError::UnsupportedMemoryMapEntrySize(tag.entry_size));
    }

    // Phase 0 relies on the fixed entry layout above and preserves the version
    // field only as future compatibility context.
    let _entry_version = tag.entry_version;

    let entries_start = tag_addr
        .checked_add(core::mem::size_of::<MemoryMapTag>())
        .ok_or(ParseError::TruncatedTag)?;
    let entries_end = tag_addr
        .checked_add(tag_size)
        .ok_or(ParseError::TruncatedTag)?;

    let mut current = entries_start;
    while current < entries_end {
        let next = current
            .checked_add(tag.entry_size as usize)
            .ok_or(ParseError::TruncatedTag)?;

        if next > entries_end {
            return Err(ParseError::TruncatedTag);
        }

        // Sound because `current..next` was validated to lie fully within the
        // tag payload and `entry_size` matches `MemoryMapEntry`.
        let entry = unsafe { &*(current as *const MemoryMapEntry) };
        let region = MemoryRegion {
            base: entry.base_addr,
            length: entry.length,
            kind: MemoryRegionKind::from_multiboot_type(entry.typ),
        };

        boot_info.push_region(region).map_err(map_boot_info_error)?;
        current = next;
    }

    Ok(())
}

/// Translates fixed-capacity owned boot-info errors into parser errors.
fn map_boot_info_error(error: BootInfoError) -> ParseError {
    match error {
        BootInfoError::TooManyMemoryRegions => ParseError::TooManyRegions,
    }
}

/// Rounds one address up to the next Multiboot2-required alignment boundary.
const fn align_up(value: usize, align: usize) -> usize {
    return (value + (align - 1)) & !(align - 1);
}
