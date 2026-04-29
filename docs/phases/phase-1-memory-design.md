# Phase 1 Memory Ownership Design

## Purpose

Phase 1 memory work turns Winnie OS from a kernel that can merely observe the
bootloader memory map into a kernel that can own physical frames and create
post-boot mappings deliberately.

This design stays narrow:

- preserve the existing GRUB + Multiboot2 and higher-half boot contract
- keep bootstrap page tables as the audited bring-up mechanism
- build a runtime memory layer above them rather than rewriting bootstrap code
- stage the allocator so frame ownership comes first and `free()` lands later in
  the same phase once reservation and mapping rules are trustworthy

Phase 1 memory does not introduce a general heap, userspace address spaces, or
copy-on-write behavior. Its job is to establish trustworthy ownership of frames
and kernel mappings.

## Current Baseline

The repository currently provides:

- an owned `BootInfo` structure with a parsed Multiboot2 memory map
- a higher-half kernel with bootstrap paging established in
  [`src/arch/x86_64/boot.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/boot.rs)
- fixed higher-half mappings for:
  - the kernel image
  - the main kernel stack
  - the PF IST stack
  - the DF IST stack
- deterministic serial-first verification and destructive fault tests

It does not yet provide:

- a kernel-owned physical frame allocator
- a runtime page-table manipulation layer
- explicit MMIO mapping helpers
- a kernel heap
- userspace page tables

## Selected Decisions

Phase 1 memory is based on the following decisions:

- begin with a monotonic 4 KiB physical frame allocator over `BootInfo`
- explicitly reserve frames already consumed by bootstrap sections and known
  kernel-owned backing pages before making allocation available
- keep frame ownership architecture-neutral where possible
- keep page-table walking and entry encoding x86_64-specific
- add post-boot mapping helpers only for kernel needs:
  - kernel text/data visibility checks if needed
  - stack and guard-page management
  - MMIO mappings such as LAPIC
- delay `free()` until the allocator can safely distinguish reusable runtime
  ownership from bootstrap-reserved pages

## Goals

Phase 1 memory should deliver:

- 4 KiB `PhysicalFrame` ownership primitives
- an allocate-only frame allocator as the first working slice
- explicit reservation of non-allocatable memory regions and bootstrap-owned
  frames
- x86_64 page-table walking and mapping helpers for kernel runtime use
- explicit helper APIs for kernel stacks, guard pages, and MMIO mappings
- allocator `free()` support before the end of Phase 1
- serial-visible verification markers for allocation and mapping behavior

## Non-Goals

Phase 1 memory should not:

- replace the bootstrap path in `boot.rs`
- add a general-purpose heap allocator
- implement userspace address spaces
- implement copy-on-write, swapping, or demand paging
- hide x86_64 paging details behind broad abstractions
- solve SMP memory ordering beyond what single-core bring-up needs

## Architectural Constraints

The design must preserve these existing invariants:

- the kernel remains in the higher half
- low bootstrap sections remain GRUB-loadable
- the kernel stack remains distinct from PF and DF IST stacks
- `#PF` stays on IST2
- `#DF` stays on IST1
- destructive fault paths remain minimal and auditable

The design also adopts these Phase 1 memory constraints:

- no allocation is assumed before the frame allocator itself exists
- page-table manipulation must stay easy to audit
- all reservations must be explicit and explainable from existing kernel state
- serial logging remains the primary verification channel

## Proposed Architecture

Phase 1 memory is intentionally split into three bounded units.

### 1. Physical Address And Frame Model

The kernel should gain small owned types for:

- physical addresses
- physical frames
- frame ranges where useful

These types should stay simple and strongly typed enough to prevent accidental
mixing of byte addresses and frame numbers. They should not attempt to encode
future NUMA or huge-page policies.

The important boundary is:

- above: architecture-neutral frame ownership concepts
- below: x86_64 page-table entry encoding and MMU details

### 2. Physical Frame Allocation

The first allocator slice should iterate the owned `BootInfo` memory map and
hand out 4 KiB frames from `Usable` regions only.

The allocator should:

- skip all non-usable regions
- align region starts and ends to 4 KiB boundaries
- reserve frames consumed by:
  - the loaded kernel image
  - bootstrap sections
  - bootstrap page tables
  - the main kernel stack backing pages
  - PF and DF IST backing pages
  - any additional static early-runtime backing storage that lives in RAM

The first implementation can be monotonic:

- advance through usable frame ranges
- allocate one frame at a time
- never recycle yet

That gives the kernel immediate frame ownership without taking on free-list or
bitmap complexity before the reservation model is trustworthy.

`free()` is still a Phase 1 requirement. It should land after the allocator and
mapper are proven and once the kernel can clearly distinguish:

- permanently reserved frames
- currently allocated runtime-owned frames
- reusable freed frames

### 3. Runtime Page-Table Mapping

Bootstrap paging should remain in assembly as the bring-up mechanism. Phase 1
adds a Rust-side x86_64 paging layer that can:

- walk the current page-table hierarchy
- create missing paging levels using allocated frames
- map and unmap 4 KiB kernel pages after boot
- expose explicit helpers for:
  - stack pages and guard pages
  - MMIO ranges
  - later kernel-owned mapping extensions

This layer should operate on the current loaded address space only. It does not
need to model multiple address spaces yet.

## Detailed Design

## Reservation Strategy

The most important correctness question in Phase 1 memory is not “how do we hand
out the next frame?” but “which frames must never be handed out?”

The kernel already knows enough to reserve several ranges explicitly from linker
symbols and bootstrap layout:

- kernel physical start and end
- low bootstrap sections
- page-table storage in `.boot.bss`
- TSS storage
- stack backing pages

The allocator should not infer these indirectly from “not usable” vs “usable.”
They should be modeled as explicit reserved ranges layered on top of the
bootloader map.

This keeps the safety story auditable:

1. `BootInfo` says which regions are usable in principle.
2. the kernel overlays its own “already occupied” ranges.
3. the allocator hands out only frames that survive both filters.

## Mapping Policy

Phase 1 memory should stay conservative about mapping policy.

Required runtime mapping support:

- create 4 KiB mappings only
- preserve the higher-half kernel layout
- preserve unmapped guard pages intentionally
- support MMIO mapping for APIC work in the interrupt half of Phase 1

Deferred mapping policy:

- huge pages
- userspace mappings
- copy-on-write
- address-space cloning

## `free()` Strategy

`free()` should not be faked or left aspirational. It should be designed into
Phase 1 from the start, but implemented after the kernel can safely reason about
ownership transitions.

The expected progression is:

1. monotonic allocation over filtered usable ranges
2. runtime page-table mapping with explicit reservations
3. a small reusable structure for freed frames
4. `free()` that:
   - rejects permanently reserved frames
   - returns only runtime-owned frames
   - makes them available for later allocation

The first reusable structure can stay simple:

- fixed-capacity stack or queue of freed frames, or
- a compact bitmap/range structure if the implementation pressure justifies it

The design should prefer the smallest structure that enables real reuse without
pulling in heap or broad allocator policy.

## Verification Strategy

Phase 1 memory should use the existing script lanes:

- `make smoke`
  - verify normal boot still succeeds after allocator integration
- `make test`
  - verify non-destructive harness checks for allocator behavior where they fit
- targeted serial-visible boot markers
  - verify reservation counts
  - verify a small sequence of successful frame allocations
  - verify mapping and unmapping of one controlled kernel page
- destructive fault runs
  - re-run at least page-fault and double-fault scenarios after mapping work to
    confirm PF/DF invariants were not broken

## Risks And Mitigations

### Risk: Reallocating Bootstrap-Owned Frames

If the allocator trusts only the bootloader’s “usable” classification, it may
hand out frames already occupied by the kernel or bootstrap data.

Mitigation:

- require explicit kernel-owned reservation overlays from known symbols and
  backing pages before any allocation is exposed

### Risk: Over-Abstracting Paging

A broad architecture-neutral paging interface too early would hide the details
that are currently most fragile and most likely to need inspection.

Mitigation:

- keep frame ownership generic
- keep page-table structure and entry semantics x86_64-specific

### Risk: Adding `free()` Too Early

If `free()` lands before reservation and runtime ownership are trustworthy, it
will make corruption harder to debug.

Mitigation:

- stage allocation first
- land `free()` in the same phase, but only after ownership boundaries are
  proven through mapping work

## Exit Criteria Satisfaction

This design satisfies the Phase 1 memory portion of the roadmap by producing:

- a kernel-owned physical frame allocator
- runtime page-table manipulation instead of bootstrap-only mappings
- explicit mapping helpers for kernel runtime needs
- a clear path to later heap and userspace work without implementing them yet
