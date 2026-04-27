# Phase 0 Foundation Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add serial-first verification, structured boot metadata, and a full no-`std` QEMU test harness without destabilizing the current higher-half x86_64 boot path.

**Architecture:** Preserve the current GRUB + Multiboot2 and higher-half bootstrap flow, then add a narrow Rust-side handoff for boot info, a mirrored VGA + serial console frontend, and two verification lanes: a Phil Opp-style integration harness and isolated destructive fault kernels. Keep architecture-neutral seams only for `BootInfo`, console routing, and test result reporting.

**Tech Stack:** Rust nightly `no_std`, custom target JSON, GRUB Multiboot2 boot, QEMU x86_64, serial COM1, `custom_test_frameworks`, repo-local shell scripts, headless serial-log verification.

---

## File Structure Map

### Existing files that will likely change

- [`src/main.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/main.rs)
  - add explicit runtime-path selection and early initialization ordering
- [`src/console/mod.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/console/mod.rs)
  - turn the VGA-only print bridge into a small console frontend
- [`src/drivers/mod.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/drivers/mod.rs)
  - export the new serial backend
- [`src/drivers/vga.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/drivers/vga.rs)
  - adapt to backend role rather than owning the whole console path
- [`src/arch/x86_64/boot.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/boot.rs)
  - preserve and hand off the Multiboot2 information pointer with minimal bootstrap change
- [`src/arch/x86_64/idt.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/idt.rs)
  - ensure serial-visible deterministic fault markers remain minimal
- [`Cargo.toml`](/Users/rileycalhoun/Documents/Projects/operating-system/Cargo.toml)
  - add cargo aliases or metadata as needed for the harness
- [`.cargo/config.toml`](/Users/rileycalhoun/Documents/Projects/operating-system/.cargo/config.toml)
  - replace the single `build.sh` runner assumption with narrower commands
- [`build.sh`](/Users/rileycalhoun/Documents/Projects/operating-system/build.sh)
  - either retire or reduce to a compatibility wrapper
- [`iso/boot/grub/grub.cfg`](/Users/rileycalhoun/Documents/Projects/operating-system/iso/boot/grub/grub.cfg)
  - update only if the test-image workflow requires explicit test menu entries

### New source files to create

- `src/boot_info.rs`
  - architecture-neutral `BootInfo` and memory-region representation
- `src/arch/x86_64/boot_info.rs`
  - Multiboot2 tag walking and parsing into `BootInfo`
- `src/drivers/serial.rs`
  - COM1 init and byte output
- `src/test_support/mod.rs`
  - shared test harness utilities, serial result markers, QEMU exit helper
- `src/test_support/qemu.rs`
  - `isa-debug-exit` support and explicit success/failure codes
- `src/test_support/runner.rs`
  - Phil Opp-style `test_runner` implementation for `#[test_case]`

### New test and entrypoint files to create

- `tests/basic_boot.rs`
  - integration harness smoke boot
- `tests/panic.rs`
  - dedicated panic-path test kernel
- `tests/invalid_opcode.rs`
  - dedicated invalid-opcode test kernel
- `tests/general_protection.rs`
  - dedicated GP-fault test kernel
- `tests/page_fault.rs`
  - dedicated page-fault test kernel
- `tests/double_fault.rs`
  - dedicated double-fault test kernel

### New tooling and docs files to create

- `scripts/build-image.sh`
  - build ELF and create bootable ISO
- `scripts/run-qemu-smoke.sh`
  - headless normal-boot run with serial capture
- `scripts/run-qemu-tests.sh`
  - integration harness lane
- `scripts/run-qemu-fault-tests.sh`
  - destructive fault lane
- `docs/phases/phase-0-design.md`
  - approved design doc
- `docs/phases/phase-0-implementation.md`
  - this plan

### External docs expected to update during implementation

- `~/Documents/winnie-os/architecture/Current Boot Flow.md`
- `~/Documents/winnie-os/architecture/Current Memory Layout.md`
- `~/Documents/winnie-os/architecture/Current Trap And Fault Handling.md`
- a repo-facing verification doc if one is added during implementation

## Task 1: Establish The Phase 0 Console Split

**Files:**

- Create: `src/drivers/serial.rs`
- Modify: `src/console/mod.rs`
- Modify: `src/drivers/mod.rs`
- Modify: `src/drivers/vga.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Define the minimal console boundary**

Document the intended responsibilities before changing code:

- `console` owns formatting entrypoints and sink fan-out
- `drivers::vga` owns VGA byte writes only
- `drivers::serial` owns COM1 init and byte writes only

Expected outcome:

- no backend owns the user-facing `print!`/`println!` contract by itself

- [ ] **Step 2: Add the COM1 serial backend**

Implement:

- fixed COM1 port constants
- explicit `init()`
- `write_byte()` and `write_bytes()`
- fixed fallback handling on formatting failure

Constraints:

- no allocation
- no interrupt-driven behavior
- no locking beyond current single-core assumptions

- [ ] **Step 3: Convert `console` into a mirrored frontend**

Update `print!` and `println!` so normal kernel output mirrors to:

- VGA
- serial

Expected behavior:

- the existing boot banner still appears on VGA
- the same output appears over serial in headless QEMU

- [ ] **Step 4: Initialize serial before other boot-stage logging**

Update `kernel_main_high` startup ordering so:

1. serial backend is initialized first
2. mirrored console is usable immediately after
3. later boot stages can safely log

- [ ] **Step 5: Build and smoke-run the normal kernel**

Run the narrowest relevant verification command once the new scripts exist. Before scripts exist, use the current local build path as a temporary check.

Expected result:

- boot succeeds
- identical startup output is visible on VGA and serial

- [ ] **Step 6: Commit**

Suggested commit:

```bash
git add src/main.rs src/console/mod.rs src/drivers/mod.rs src/drivers/vga.rs src/drivers/serial.rs
git commit -m "feat(console): mirror kernel output to serial and vga"
```

## Task 2: Add An Explicit BootInfo Handoff

**Files:**

- Create: `src/boot_info.rs`
- Create: `src/arch/x86_64/boot_info.rs`
- Modify: `src/main.rs`
- Modify: `src/arch/x86_64/mod.rs`
- Modify: `src/arch/x86_64/boot.rs`

- [ ] **Step 1: Define the stable post-parse representation**

Add a small owned `BootInfo` model with:

- memory-region type
- fixed-capacity region storage
- explicit region-kind enum
- helper iteration APIs that do not expose raw Multiboot2 internals

- [ ] **Step 2: Preserve the Multiboot2 information pointer in bootstrap**

Make the smallest possible change in `boot.rs` to capture the raw pointer and pass it into Rust.

Rules:

- do not change higher-half mapping strategy
- do not restructure stack or TSS setup
- keep new assembly comments focused on the handoff invariant only

- [ ] **Step 3: Parse the memory-map tag in Rust**

Implement a small Multiboot2 parser module that:

- validates basic tag iteration structure
- finds the memory-map tag
- copies regions into `BootInfo`
- ignores unsupported tags unless they affect safety

- [ ] **Step 4: Log the parsed boot memory map over serial**

Emit deterministic stage markers plus structured memory-region lines.

Expected output shape:

- one boot-stage marker indicating parsing started
- one line per parsed region
- one summary marker indicating success or a fixed parse failure message

- [ ] **Step 5: Verify with a headless QEMU boot**

Expected result:

- normal boot still reaches the terminal `hlt` path
- serial output includes parsed memory-region lines

- [ ] **Step 6: Commit**

Suggested commit:

```bash
git add src/main.rs src/boot_info.rs src/arch/x86_64/mod.rs src/arch/x86_64/boot.rs src/arch/x86_64/boot_info.rs
git commit -m "feat(boot): parse multiboot memory map into boot info"
```

## Task 3: Normalize Panic And Fatal Fault Reporting

**Files:**

- Modify: `src/main.rs`
- Modify: `src/arch/x86_64/idt.rs`
- Create: `src/test_support/mod.rs`

- [ ] **Step 1: Define fixed serial-visible fatal markers**

Choose exact output markers for:

- `PANIC`
- `DIVIDE ERROR`
- `INVALID OPCODE`
- `GENERAL PROTECTION FAULT`
- `PAGE FAULT`
- `DOUBLE FAULT`

Requirement:

- markers stay stable once introduced because scripts will depend on them

- [ ] **Step 2: Ensure panic uses the same minimal reporting contract**

Keep the panic path simple:

- print fixed marker
- optionally print minimal safe metadata only if it does not complicate the path
- enter terminal halt or test-specific exit behavior

- [ ] **Step 3: Ensure IDT handlers remain minimal and deterministic**

Verify:

- `#PF` still uses IST2
- `#DF` still uses IST1
- handlers still terminate immediately after fixed reporting

- [ ] **Step 4: Build and verify all non-test boot paths**

Expected result:

- normal boot unchanged except for stronger serial visibility
- deliberate manual fault triggers, if temporarily added during development, produce the expected marker

- [ ] **Step 5: Commit**

Suggested commit:

```bash
git add src/main.rs src/arch/x86_64/idt.rs src/test_support/mod.rs
git commit -m "refactor(traps): standardize fatal serial reporting"
```

## Task 4: Add QEMU Exit Support And Shared Test Utilities

**Files:**

- Create: `src/test_support/qemu.rs`
- Create: `src/test_support/runner.rs`
- Modify: `src/test_support/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add explicit QEMU success and failure exit codes**

Implement a tiny helper around `isa-debug-exit` with:

- one success code
- one failure code
- one helper for unrecoverable harness misuse if needed

- [ ] **Step 2: Add shared serial test-reporting helpers**

Create helpers for:

- test start marker
- per-test pass marker
- per-test failure marker
- suite summary marker

- [ ] **Step 3: Isolate test-only behavior from the normal kernel path**

Structure the code so the normal runtime path does not depend on:

- test harness initialization
- QEMU exit side effects
- test-only panic handling

- [ ] **Step 4: Verify helper behavior with one temporary smoke case**

Expected result:

- a test-configured kernel can emit serial markers and terminate QEMU predictably

- [ ] **Step 5: Commit**

Suggested commit:

```bash
git add src/test_support/mod.rs src/test_support/qemu.rs src/test_support/runner.rs src/main.rs
git commit -m "feat(test): add qemu exit and shared test utilities"
```

## Task 5: Build The No-`std` Integration Test Harness

**Files:**

- Modify: `src/main.rs`
- Modify: `Cargo.toml`
- Modify: `.cargo/config.toml`
- Create: `tests/basic_boot.rs`

- [ ] **Step 1: Enable the Phil Opp-style custom test framework flow**

Add the required crate attributes and harness entrypoint pattern for:

- `custom_test_frameworks`
- `#[test_case]`
- `reexport_test_harness_main`

Requirement:

- keep the normal non-test kernel entrypoint easy to audit

- [ ] **Step 2: Implement the integration test runner**

The runner should:

- initialize serial output
- print suite start
- execute `#[test_case]` functions
- print structured pass/fail lines
- exit QEMU successfully on completion

- [ ] **Step 3: Add the first integration test kernel**

Create `tests/basic_boot.rs` as the minimal smoke harness:

- boot into the test path
- run one trivial assertion
- report success over serial
- exit QEMU with the success code

- [ ] **Step 4: Run the integration harness**

Run the dedicated harness command.

Expected result:

- serial output clearly shows suite start and pass
- QEMU exits with the success code

- [ ] **Step 5: Commit**

Suggested commit:

```bash
git add src/main.rs Cargo.toml .cargo/config.toml tests/basic_boot.rs
git commit -m "feat(test): add no-std kernel integration harness"
```

## Task 6: Add Dedicated Panic And Fault Test Kernels

**Files:**

- Create: `tests/panic.rs`
- Create: `tests/invalid_opcode.rs`
- Create: `tests/general_protection.rs`
- Create: `tests/page_fault.rs`
- Create: `tests/double_fault.rs`
- Modify: `src/arch/x86_64/idt.rs` only if a test-only hook is strictly needed
- Modify: `src/test_support/mod.rs`

- [ ] **Step 1: Define one expected outcome contract per destructive test**

Each destructive test must declare:

- the deliberate trigger
- the exact expected serial marker
- whether success is detected by QEMU exit code, terminal halt plus log match, or both

- [ ] **Step 2: Add the panic-path test kernel**

Implement a dedicated panic test that:

- boots into a test-only path
- intentionally panics
- emits the expected marker
- exits or halts in the expected way for the wrapper script

- [ ] **Step 3: Add invalid-opcode and general-protection tests**

Implement one dedicated kernel each.

Rules:

- keep the trigger local and obvious
- do not reuse a generalized “fault injector”

- [ ] **Step 4: Add page-fault and double-fault tests**

Implement one dedicated kernel each.

Critical verification points:

- page-fault test still reaches the IST2-backed handler
- double-fault test still reaches the IST1-backed handler

- [ ] **Step 5: Run the destructive suite**

Expected result:

- every scenario emits the correct fixed marker
- scripts classify expected failure as pass and unexpected behavior as fail

- [ ] **Step 6: Commit**

Suggested commit:

```bash
git add tests/panic.rs tests/invalid_opcode.rs tests/general_protection.rs tests/page_fault.rs tests/double_fault.rs src/test_support/mod.rs src/arch/x86_64/idt.rs
git commit -m "feat(test): add destructive fault kernel tests"
```

## Task 7: Replace The Monolithic Runner With Repo-Local Commands

**Files:**

- Create: `scripts/build-image.sh`
- Create: `scripts/run-qemu-smoke.sh`
- Create: `scripts/run-qemu-tests.sh`
- Create: `scripts/run-qemu-fault-tests.sh`
- Modify: `build.sh`
- Modify: `.cargo/config.toml`
- Modify: `Cargo.toml`

- [ ] **Step 1: Split image construction from execution**

Create a reusable image-build script that:

- builds the kernel ELF
- copies it into the ISO tree
- builds the ISO

- [ ] **Step 2: Add a normal smoke runner**

The smoke runner should:

- boot headless QEMU
- capture serial output
- fail if the expected boot marker is missing

- [ ] **Step 3: Add separate integration and destructive test runners**

Requirements:

- integration harness runner expects QEMU success exit
- destructive runner iterates scenarios and checks each scenario-specific marker

- [ ] **Step 4: Add cargo aliases or documented command shims**

Expose stable commands such as:

- `cargo run`
- `cargo xtask`-style equivalents if chosen later
- shell scripts under `scripts/`

Pick one documented primary interface and keep the others thin wrappers.

- [ ] **Step 5: Verify all three command paths**

Required checks:

- smoke boot command passes
- integration harness command passes
- destructive suite command passes

- [ ] **Step 6: Commit**

Suggested commit:

```bash
git add scripts/build-image.sh scripts/run-qemu-smoke.sh scripts/run-qemu-tests.sh scripts/run-qemu-fault-tests.sh build.sh .cargo/config.toml Cargo.toml
git commit -m "build: add scripted qemu smoke and test runners"
```

## Task 8: Document The Bring-Up And Verification Workflow

**Files:**

- Modify: `docs/ROADMAP.md` only if Phase 0 wording or status needs tightening
- Create or modify: repo-facing verification documentation if introduced during implementation
- Modify: `docs/phases/phase-0-design.md`
- Modify: `docs/phases/phase-0-implementation.md`
- Update external vault notes:
  - `~/Documents/winnie-os/architecture/Current Boot Flow.md`
  - `~/Documents/winnie-os/architecture/Current Memory Layout.md`
  - `~/Documents/winnie-os/architecture/Current Trap And Fault Handling.md`

- [ ] **Step 1: Document the serial-first verification path**

Explain:

- why serial is authoritative
- how VGA mirroring fits in
- what the stable commands are

- [ ] **Step 2: Document the boot-info handoff**

Cover:

- raw Multiboot2 pointer capture
- Rust-side parsing boundary
- the owned memory-map representation

- [ ] **Step 3: Document the harness lanes**

Cover:

- integration harness expectations
- destructive suite expectations
- stable markers and success conditions

- [ ] **Step 4: Update the Obsidian vault**

Keep the paired architecture notes aligned with the implemented code and verification workflow.

- [ ] **Step 5: Commit**

Suggested commit:

```bash
git add docs/ROADMAP.md docs/phases/phase-0-design.md docs/phases/phase-0-implementation.md
git commit -m "docs: record phase 0 verification and boot design"
```

## Task 9: Final Verification And Handoff

**Files:**

- No new product files required

- [ ] **Step 1: Run the full Phase 0 verification set**

Required command categories:

- normal smoke boot
- integration harness
- destructive fault suite

Expected result:

- all commands pass
- serial logs show the expected stage and result markers

- [ ] **Step 2: Inspect for regressions in architecture-critical paths**

Review these files carefully before signoff:

- `src/arch/x86_64/boot.rs`
- `src/arch/x86_64/idt.rs`
- `linker.ld`

Confirm:

- higher-half assumptions remain intact
- IST slot assignments remain intact
- new logic did not broaden the destructive fault paths

- [ ] **Step 3: Record any residual gaps explicitly**

If any scenario remains partially verified, capture:

- which scenario
- what was verified
- what is still missing

- [ ] **Step 4: Final commit or merge-ready handoff**

Suggested final commit if a separate integration commit is desired:

```bash
git add -A
git commit -m "feat: complete phase 0 foundation hardening"
```

## Verification Commands To Standardize During Implementation

These command names are targets, not assumptions about the current repo state. The implementation should converge on one documented command path per category.

- Smoke boot:
  - `scripts/run-qemu-smoke.sh`
- Integration harness:
  - `scripts/run-qemu-tests.sh`
- Destructive suite:
  - `scripts/run-qemu-fault-tests.sh`

Expected outputs to standardize:

- smoke boot prints the normal boot marker sequence and reaches the terminal halt path
- integration harness prints suite start, per-test pass markers, summary, and exits QEMU successfully
- destructive tests print the exact expected fatal marker per scenario and are classified correctly by the wrapper script

## Execution Notes

- Keep each task small and reviewable.
- Run the simplest relevant QEMU verification after any change to boot, traps, or the test harness.
- Avoid broad refactors while Phase 0 is still landing.
- If any task forces a change to `linker.ld`, re-check the higher-half and bootstrap-section assumptions immediately.
