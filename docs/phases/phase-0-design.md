# Phase 0 Design

## Purpose

Phase 0 turns Winnie OS from a booting kernel into a repeatable bring-up platform.
The work stays intentionally narrow:

- preserve the existing GRUB + Multiboot2 boot contract
- preserve the current higher-half handoff
- preserve the current `#DF` on IST1 and `#PF` on IST2 invariants
- add the minimum structure needed for debug visibility, boot metadata handoff, and deterministic verification

This phase does not attempt to introduce a general allocator, a long-term virtual memory subsystem, a scheduler, or broad architecture abstraction. It creates the tools and interfaces later phases need to evolve the kernel safely.

## Current Baseline

The repository currently provides:

- a GRUB-loaded Multiboot2 x86_64 higher-half kernel
- early bootstrap paging and long-mode entry in [`src/arch/x86_64/boot.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/boot.rs)
- a minimal IDT with fatal exception handlers in [`src/arch/x86_64/idt.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/idt.rs)
- VGA text output through [`src/console/mod.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/console/mod.rs) and [`src/drivers/vga.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/drivers/vga.rs)
- a terminal `hlt` path after startup

It does not yet provide:

- serial output
- structured boot information in Rust
- a memory map handoff for later allocators
- an automated no-`std` test harness
- deterministic QEMU pass/fail signaling
- a documented single-command verification path

## Selected Decisions

The Phase 0 design is based on the following decisions:

- allow narrow enabling refactors when they reduce bring-up risk
- follow Phil Opp's `no_std` test-harness model conceptually, but keep orchestration in repo-local scripts and cargo aliases rather than `bootimage`
- keep x86_64 implementation details explicit while introducing architecture-neutral seams only for console output, boot information, and test reporting
- mirror console output to VGA and serial, with serial treated as the authoritative verification channel
- preserve GRUB + Multiboot2 and add a small explicit in-tree parser
- cover normal boot, panic, invalid opcode, general protection, page fault, and double fault in the first destructive-test suite

## Goals

Phase 0 should deliver:

- serial console output in headless QEMU
- explicit boot-stage logging visible over serial
- a small owned boot-info structure that includes the bootloader memory map
- deterministic minimal panic and fatal-fault reporting
- a working no-`std` test harness for kernel integration tests
- isolated destructive fault tests with repeatable QEMU orchestration
- repo and vault documentation for the new bring-up workflow

## Non-Goals

Phase 0 should not:

- redesign the boot contract
- replace GRUB
- introduce heap-backed logging or boot metadata storage
- add recovery-oriented fault handling
- hide architecture-critical state transitions behind abstractions
- implement the actual physical memory allocator planned for Phase 1

## Architectural Constraints

The design must preserve these existing invariants:

- the kernel runs in the higher half
- low bootstrap sections remain GRUB-loadable
- the kernel stack stays distinct from PF and DF IST stacks
- `#PF` stays on IST2
- `#DF` stays on IST1
- destructive fault paths stay simple enough to survive compromised stack state

The design also adopts these Phase 0 constraints:

- no allocation is assumed
- all early reporting paths must work with fixed storage and direct writes
- boot and trap code remain easy to audit
- later multi-architecture support is only prepared where it meaningfully reduces churn

## Proposed Architecture

Phase 0 adds a narrow early-runtime boundary immediately after the higher-half Rust handoff. The boundary has three responsibilities:

1. Initialize deterministic output channels early enough that later boot stages can be diagnosed in headless QEMU.
2. Parse raw bootloader metadata into a small owned kernel structure that later phases can consume without depending on Multiboot2 parsing internals.
3. Distinguish normal boot, integration-test boot, and destructive-fault boot flows without complicating the normal runtime path.

The architecture is intentionally split into four bounded units.

### 1. Boot Information

x86_64 bootstrap preserves the Multiboot2 information pointer and passes it into Rust as part of an explicit handoff. Rust parses only the tags Phase 0 needs, especially the memory map, and copies that data into a small fixed-capacity kernel structure.

This creates a stable boundary:

- below the boundary: architecture-specific Multiboot2 parsing details
- above the boundary: a small architecture-neutral `BootInfo` representation that later memory-management code can consume

### 2. Console And Logging

The current VGA-only printing path becomes a minimal console frontend with two backends:

- VGA text mode
- x86_64 serial COM1

The frontend remains simple:

- direct writes only
- no allocation
- no scrolling redesign requirement beyond current behavior
- no generalized logger subsystem

Serial is the canonical verification channel. VGA remains mirrored for local visibility.

### 3. Fatal And Test Reporting

Panic, exception handlers, and test harness result reporting should all emit deterministic serial-visible markers. The implementation should share only the minimum reporting plumbing needed for fixed messages and explicit QEMU exit signaling.

Destructive fault paths remain isolated from the normal test harness:

- panic and non-destructive integration tests can use a Phil Opp-style `#[test_case]` harness
- deliberate invalid opcode, general protection, page fault, and double fault tests use dedicated test kernels or entrypoints that intentionally terminate in the target fault path

### 4. Verification Tooling

Repo-local build and run scripts become the authoritative verification interface. Rather than one monolithic `build.sh`, the repository should expose narrower commands for:

- normal kernel build and boot smoke run
- integration test harness run
- destructive fault suite run
- image generation as a reusable primitive

These scripts should use headless QEMU, serial capture, and deterministic success/failure signaling through `isa-debug-exit` or an equivalent QEMU-supported mechanism.

## Detailed Component Design

## Boot Handoff And Initialization Order

The boot path should remain conservative. `_start` in [`src/arch/x86_64/boot.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/boot.rs) keeps responsibility for:

- stack setup in 32-bit bootstrap mode
- early page-table construction
- long-mode entry
- GDT/TSS setup
- loading the higher-half kernel stack
- jumping into Rust

Phase 0 should add only one new boot responsibility here:

- preserve the Multiboot2 information pointer and hand it to Rust without changing the higher-half layout

Once Rust starts, initialization should become explicitly ordered:

1. initialize the serial backend first
2. initialize the shared console frontend so output mirrors to VGA and serial
3. parse and store `BootInfo`
4. emit boot-stage logs for visibility
5. initialize the IDT
6. select the runtime path:
   - normal boot path
   - integration test harness path
   - destructive fault test path

Every stage should emit fixed serial-visible markers so QEMU logs can pinpoint where boot stopped.

## BootInfo Representation

Phase 0 needs a small owned structure, not a broad boot abstraction. The representation should cover:

- loader identity if cheaply available
- kernel physical and virtual span if helpful for logging
- boot memory map as a list of structured regions
- region kind data sufficient for later physical-memory-manager work

The structure should avoid:

- keeping raw borrowed Multiboot2 pointers as the stable interface
- heap-backed collections
- broad tag support that later phases do not need yet

The parser should validate only enough to safely walk Multiboot2 tags and extract the memory map. Unsupported tags should be ignored unless they affect safety.

## Console And Output Model

The console architecture should stay smaller than a real logger. A minimal shape is enough:

- one console frontend that accepts formatted output
- one VGA backend
- one serial backend
- one optional low-level emergency reporting helper for fatal paths if normal formatting is too risky

Required behavior:

- `print!` and `println!` still exist as the main kernel-facing macros
- ordinary output is mirrored to VGA and serial
- serial initialization is explicit and idempotent enough for early boot use
- formatting failures still degrade to a fixed visible fallback message

Deferred behavior:

- levels beyond simple stage or severity prefixes
- runtime-configurable sinks
- buffering, locking, or concurrent ownership beyond the current single-core bring-up assumptions

## Panic And Exception Reporting

The repository guidance is correct for this phase: destructive fault paths should stay brutally simple.

The reporting design should therefore require:

- fixed messages for panic and each fatal exception
- serial visibility for all destructive paths
- optional minimal metadata only when it is robust to print in bad machine state
- terminal halt after reporting unless a test-specific QEMU exit path is explicitly part of the scenario

The design should reject:

- richer panic formatting as the primary destructive path
- generalized recovery
- shared code that hides which vector or trap path is being exercised

Current vector expectations remain:

- divide error
- invalid opcode
- general protection fault
- page fault on IST2
- double fault on IST1

## Test Harness Design

Phase 0 should build a fully functional `no_std` kernel test harness, but not by forcing every test shape into one mechanism.

### Integration Harness Lane

The first lane follows the Phil Opp model:

- nightly `custom_test_frameworks`
- `#[test_case]` discovery
- a test runner that prints structured serial output
- explicit QEMU success/failure exit codes

This lane is appropriate for:

- basic boot smoke assertions
- parser unit-style integration tests that must run in kernel context
- console and reporting behavior checks that do not intentionally destroy machine state

### Destructive Fault Lane

The second lane is for deliberate fatal conditions:

- panic
- invalid opcode
- general protection fault
- page fault
- double fault

These should use one bootable kernel binary plus compile-time-selected boot
scenarios so each case:

- exercises one fault source intentionally
- emits one expected serial marker
- halts or exits in a way the QEMU wrapper can classify

This separation matters because it avoids mixing intentionally destructive
machine-state tests into the ordinary `#[test_case]` runner path while still
preserving the same GRUB and Multiboot2 boot contract as normal boot.

## QEMU And Verification Strategy

Headless QEMU becomes the standard verification path for Phase 0. The strategy should include:

- serial redirection to a captured log
- deterministic guest exit for pass/fail when possible
- explicit expected markers per scenario
- separate command paths for smoke boot, integration tests, and destructive tests

The test harness should use a QEMU device like `isa-debug-exit` so scripts can fail fast without parsing logs alone. Log parsing still matters for diagnostics and for destructive tests whose success criteria include the emitted fatal marker.

Verification commands should be documented and stable enough that contributors can run them without understanding the full harness internals.

## Architecture-Neutral Seams

Phase 0 should introduce architecture-neutral boundaries only where they pay for themselves now:

- `BootInfo` as a stable post-parse representation
- console frontend interfaces that do not hard-code VGA
- test result reporting that does not depend on x86_64-specific call sites

It should remain architecture-specific in:

- Multiboot2 pointer capture
- COM1 serial port details
- IDT and exception vectors
- fault-trigger instructions and QEMU test scenarios tied to x86_64 behavior

This avoids premature abstraction while reducing later churn for the ARM64 phase.

## File Boundary Strategy

The implementation should prefer small focused files.

Expected boundary changes:

- keep [`src/arch/x86_64/boot.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/boot.rs) small and architecture-critical
- add a dedicated x86_64 boot-info parsing module rather than expanding `boot.rs`
- introduce a small architecture-neutral `BootInfo` module
- split console frontend logic from backend drivers
- add a dedicated serial driver under `src/drivers/`
- keep harness dispatch and destructive scenarios separate from the normal runtime path
- replace the one-shot build script with narrower scripts or cargo aliases

## Documentation Deliverables

Phase 0 changes the effective bring-up workflow, so documentation is part of the design, not cleanup.

Repository docs should cover:

- current Phase 0 architecture and scope
- how serial logging works
- how boot info is parsed and exposed
- how to run the smoke boot and both test lanes
- expected serial markers and QEMU outcomes

The paired Obsidian vault at `~/Documents/winnie-os/` should be updated where behavior materially changes, especially for:

- current boot flow
- current memory layout if the documented handoff changes
- current trap and fault handling
- verification workflow or milestone status

## Risks And Mitigations

### Risk: Over-Engineering The Console

If Phase 0 grows a generalized logging subsystem, it will add complexity without solving the current milestone problem.

Mitigation:

- keep one thin frontend
- keep direct writes
- avoid runtime configurability

### Risk: Making Boot Code Harder To Audit

If too much parsing or reporting logic leaks into `boot.rs`, the most fragile path becomes harder to reason about.

Mitigation:

- keep pointer capture in bootstrap minimal
- move parsing into Rust modules with narrow responsibilities

### Risk: Shared Harness Logic Infects Fatal Paths

If destructive tests depend on too much common infrastructure, the tests will become less trustworthy.

Mitigation:

- isolate destructive scenarios
- keep success criteria at the serial/QEMU boundary
- preserve terminal halt behavior where appropriate

### Risk: Toolchain And Runner Churn

Nightly features, linker behavior, and QEMU device flags become part of the repository contract once the harness lands.

Mitigation:

- document the contract explicitly
- keep the runner path repo-local and inspectable
- prefer a small number of stable commands

## Exit Criteria Mapping

The design satisfies the roadmap Phase 0 exit criteria this way:

- `the kernel can emit logs over serial in headless QEMU`
  - satisfied by mirrored console output and scripted headless serial capture
- `destructive fault tests are easy to trigger and inspect`
  - satisfied by isolated destructive kernels or entrypaths plus expected serial markers
- `boot-time memory regions are visible to the kernel in a structured form`
  - satisfied by the owned `BootInfo` memory map representation
- `contributors can verify boot and fault behavior with a single documented command path`
  - satisfied by repo-local scripts/cargo aliases with clear docs

## Implementation Intent

The Phase 0 implementation should proceed incrementally:

1. serial backend and mirrored console
2. explicit boot-info handoff and Multiboot2 memory-map parsing
3. deterministic panic and fault reporting cleanup
4. integration harness lane
5. destructive fault lane
6. repo-local verification commands
7. repository and vault documentation

That order improves observability first, then uses the better visibility to make the rest of Phase 0 safer to implement.
