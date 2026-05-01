# Phase 1 Memory Ownership Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Winnie OS kernel-owned physical frame allocation, runtime kernel mapping helpers, and allocator `free()` support without destabilizing the audited bootstrap paging path.

**Architecture:** Add a small architecture-neutral frame model above `BootInfo`, then build a monotonic 4 KiB frame allocator with explicit kernel-owned reservations. Layer an x86_64 runtime mapper on top of the existing address space so the kernel can add and remove controlled kernel mappings after boot. Land `free()` only after ownership and reservation rules are proven.

**Tech Stack:** Rust nightly `no_std`, owned Multiboot2 `BootInfo`, x86_64 4 KiB paging, linker symbols from the current kernel image, serial-first QEMU verification.

---

## File Structure Map

### Existing files that will likely change

- [`src/lib.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/lib.rs)
  - integrate memory initialization and serial-visible verification hooks
- [`src/boot_info.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/boot_info.rs)
  - expose any small helpers needed for usable-region iteration
- [`src/arch/x86_64/boot.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/boot.rs)
  - avoid changes unless a narrow symbol/export detail is strictly required
- [`src/arch/x86_64/mod.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/mod.rs)
  - export new paging helpers if added
- [`src/main.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/main.rs)
  - keep early init order stable if any new boot-stage markers are needed

### New source files to create

- `src/memory/mod.rs`
  - memory-subsystem root and public integration surface
- `src/memory/frame.rs`
  - physical address, frame, and frame-range types
- `src/memory/allocator.rs`
  - frame allocator state, reservations, allocate-only path, and later `free()`
- `src/arch/x86_64/paging.rs`
  - x86_64 runtime page-table walking and mapping helpers

### External docs expected to update during implementation

- `docs/ROADMAP.md` only if Phase 1 wording needs tightening after landing
- `~/Documents/winnie-os/Handbook/Architecture/Current Memory Layout.md`
- `~/Documents/winnie-os/Handbook/Architecture/Current Boot Flow.md`
- `~/Documents/winnie-os/Handbook/Reference/Build And Boot Pipeline.md` if verification commands change

## Task 1: Define The Frame Model

**Files:**

- Create: `src/memory/mod.rs`
- Create: `src/memory/frame.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Add the memory module root**

Expose a narrow public surface for:

- frame types
- allocator init/access entrypoints
- future paging helpers

Expected outcome:

- no paging or allocation logic leaks directly through `lib.rs`

- [x] **Step 2: Add the physical address and frame types**

Implement small owned types for:

- `PhysicalAddress`
- `PhysicalFrame`
- optional `FrameRange` helper if it reduces iteration complexity

Requirements:

- 4 KiB frame alignment helpers
- explicit conversion boundaries between addresses and frames
- no premature huge-page support

- [x] **Step 3: Add one minimal compile-time or harness-visible smoke check**

Write the narrowest test or boot-time assertion that proves:

- frame alignment helpers behave as expected
- usable-region iteration can be expressed in frame terms

- [x] **Step 4: Verify the new types compile without changing boot behavior**

Run:

```bash
make build
make smoke
```

Expected:

- normal boot still succeeds
- no new boot-path regressions

- [x] **Step 5: Commit**

```bash
git add src/lib.rs src/memory/mod.rs src/memory/frame.rs
git commit -m "feat(memory): add physical frame model"
```

## Task 2: Build The Monotonic Frame Allocator

**Files:**

- Create: `src/memory/allocator.rs`
- Modify: `src/memory/mod.rs`
- Modify: `src/boot_info.rs` only if a helper is truly needed
- Modify: `src/lib.rs`

- [x] **Step 1: Write the failing allocator-behavior test**

Add the narrowest check for:

- allocator skips non-usable regions
- allocator returns aligned 4 KiB frames
- allocator progresses monotonically through usable space

The first test can be a small in-kernel harness/unit-style test over synthetic
region input if that is simpler than boot-path observation.

- [x] **Step 2: Verify the test fails for the intended reason**

Run the narrowest relevant command:

```bash
make test
```

Expected:

- failure because allocator behavior is not implemented yet

- [x] **Step 3: Implement reservation-free monotonic allocation**

Add allocator state that:

- walks owned `BootInfo` regions
- filters to `MemoryRegionKind::Usable`
- aligns region bounds to 4 KiB frames
- returns one frame at a time

Do not implement `free()` yet.

- [x] **Step 4: Add serial-visible boot logging for a tiny allocation sample**

During normal boot, log:

- allocator init marker
- a small count or sample of successfully allocated frames

Keep it deterministic and small.

- [x] **Step 5: Verify the allocator works in both harness and boot paths**

Run:

```bash
make smoke
make test
```

Expected:

- normal boot still reaches its terminal path
- the test harness still passes
- allocation markers appear over serial in smoke output

- [x] **Step 6: Commit**

```bash
git add src/boot_info.rs src/lib.rs src/memory/mod.rs src/memory/allocator.rs
git commit -m "feat(memory): add monotonic physical frame allocator"
```

## Task 3: Reserve Kernel-Owned Frames Explicitly

**Files:**

- Modify: `src/memory/allocator.rs`
- Modify: `src/lib.rs`
- Modify: `src/arch/x86_64/boot.rs` only if a symbol export is strictly required

- [x] **Step 1: Define the required reservation inputs**

Collect the ranges that must never be allocated:

- kernel physical image span
- bootstrap sections
- page-table backing frames
- kernel stack backing pages
- PF IST backing page
- DF IST backing page

Prefer existing linker/bootstrap symbols over inferred layout.

- [x] **Step 2: Add the failing reservation test**

Write the narrowest test or assertion showing that one known reserved frame is
still incorrectly returned by the current allocator.

- [x] **Step 3: Verify the reservation test fails**

Run the smallest relevant verification path and confirm failure is due to the
missing reservation overlay rather than unrelated compile issues.

- [x] **Step 4: Implement explicit reserved-range filtering**

Overlay the reserved ranges on top of usable boot regions so allocation skips
kernel-owned frames deterministically.

- [x] **Step 5: Verify with serial-visible allocation sampling**

Run:

```bash
make smoke
```

Expected:

- allocation markers still appear
- sampled frames no longer overlap known reserved ranges

- [x] **Step 6: Commit**

```bash
git add src/lib.rs src/memory/allocator.rs
git commit -m "fix(memory): reserve kernel-owned physical frames"
```

## Task 4: Add Runtime x86_64 Mapping Helpers

**Files:**

- Create: `src/arch/x86_64/paging.rs`
- Modify: `src/arch/x86_64/mod.rs`
- Modify: `src/memory/mod.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Write the failing mapping test or boot-time proof**

Choose the narrowest proof that the runtime mapper must satisfy, for example:

- map one scratch kernel page from an allocated frame
- write through the mapped address
- unmap it cleanly

- [x] **Step 2: Verify the proof fails before implementation**

Run the smallest relevant path and confirm failure is due to absent runtime
mapping support.

- [x] **Step 3: Implement x86_64 page-table walking**

Add helpers that can:

- locate the active P4
- walk/create lower paging levels
- encode/decode 4 KiB page mappings

Keep the implementation x86_64-specific and auditable.

- [x] **Step 4: Add explicit kernel mapping helpers**

Support at minimum:

- mapping one runtime kernel page
- unmapping one runtime kernel page
- preserving intentional guard-page holes
- mapping one MMIO page for later interrupt work

- [x] **Step 5: Verify smoke and destructive fault invariants still hold**

Run:

```bash
make smoke
make faults
```

Expected:

- normal boot still succeeds
- page-fault and double-fault scenarios still behave as designed

- [x] **Step 6: Commit**

```bash
git add src/lib.rs src/memory/mod.rs src/arch/x86_64/mod.rs src/arch/x86_64/paging.rs
git commit -m "feat(memory): add runtime x86_64 paging helpers"
```

## Task 5: Add `free()` And Reuse Support

**Files:**

- Modify: `src/memory/allocator.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Write the failing reuse test**

Add the narrowest test showing:

- allocated runtime-owned frame can be freed
- next allocation can reuse a freed frame
- permanently reserved frames cannot be freed successfully

- [x] **Step 2: Verify the test fails for the expected reason**

Run:

```bash
make test
```

Expected:

- failure because `free()` / reuse is not yet implemented

- [x] **Step 3: Implement a minimal freed-frame structure**

Choose the smallest viable structure that does not require a heap:

- fixed-capacity stack/queue of freed frames, or
- compact bitmap/range structure only if clearly warranted

- [x] **Step 4: Implement `free()` with explicit ownership checks**

`free()` must:

- reject permanently reserved frames
- reject obviously invalid/unmanaged frames
- accept runtime-owned allocated frames
- make them available for later reuse

- [x] **Step 5: Verify reuse and boot stability**

Run:

```bash
make test
make smoke
```

Expected:

- reuse test passes
- normal boot remains stable

- [x] **Step 6: Commit**

```bash
git add src/lib.rs src/memory/allocator.rs
git commit -m "feat(memory): add frame free and reuse support"
```
