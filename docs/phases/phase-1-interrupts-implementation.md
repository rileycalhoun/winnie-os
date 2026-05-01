# Phase 1 Core Interrupts And Timers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Winnie OS explicit x86_64 interrupt-controller ownership, a reliable periodic timer interrupt, and one minimal interrupt-safe synchronization primitive without destabilizing the current fault and IST paths.

**Architecture:** Mask or disable the legacy PIC, map and initialize the local APIC through the Phase 1 memory mapper, install one timer IRQ vector above the CPU exception range, and keep the timer handler intentionally small. Add only one interrupt-safe spinlock for the shared state the timer path actually needs.

**Tech Stack:** Rust nightly `no_std`, x86_64 IDT and TSS model, LAPIC MMIO, serial-first QEMU verification, existing destructive fault suite.

---

## File Structure Map

### Existing files that will likely change

- [`src/main.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/main.rs)
  - preserve the current shared early bring-up and scenario-dispatch boundary;
    avoid moving timer bring-up here unless the architecture changes
- [`src/lib.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/lib.rs)
  - integrate PIC/APIC/timer init into the normal boot path and add
    serial-visible verification hooks
- [`src/arch/x86_64/idt.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/idt.rs)
  - add the timer IRQ vector handler while preserving current fatal exception behavior
- [`src/arch/x86_64/mod.rs`](/Users/rileycalhoun/Documents/Projects/operating-system/src/arch/x86_64/mod.rs)
  - export new controller/timer modules

### New source files to create

- `src/arch/x86_64/pic.rs`
  - legacy PIC masking/disable logic
- `src/arch/x86_64/apic.rs`
  - LAPIC register access and initialization
- `src/arch/x86_64/timer.rs`
  - timer-vector constants, timer state, and timer init helpers
- `src/sync/mod.rs`
  - synchronization module root
- `src/sync/spinlock.rs`
  - minimal interrupt-safe spinlock primitive

### Expected dependencies on the memory track

- LAPIC MMIO mapping should use the runtime mapping helper from
  `src/arch/x86_64/paging.rs`
- interrupt docs and code should not bypass that layer with new bootstrap-only
  mappings

### Expected verification dependency

- the current `make smoke` path proves only that normal boot still reaches its
  stable boot marker
- this plan should add one dedicated timer-proof command or extend the smoke
  path so QEMU waits for a timer marker instead of exiting immediately after
  `Hello from WinnieOS!`

### External docs expected to update during implementation

- `docs/ROADMAP.md` only if Phase 1 wording needs tightening after landing
- `~/Documents/winnie-os/Handbook/Architecture/Current Trap And Fault Handling.md`
- `~/Documents/winnie-os/Handbook/Architecture/Current Boot Flow.md`
- `~/Documents/winnie-os/Handbook/Reference/Build And Boot Pipeline.md` if new
  verification commands or markers are added

## Task 1: Define IRQ Vector And Timer-State Boundaries

**Files:**

- Create: `src/arch/x86_64/timer.rs`
- Modify: `src/arch/x86_64/mod.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Pick the first timer IRQ vector**

Define one vector constant above the CPU exception range, for example in the
usual IRQ remap range.

Requirements:

- clearly separate exceptions from IRQ vectors
- document the intended purpose of the timer vector

- [x] **Step 2: Define the minimal shared timer state**

Add the smallest state needed to prove timer delivery, such as:

- one tick counter
- one “first tick observed” flag

Do not add scheduler-facing semantics yet.

- [x] **Step 3: Add the failing timer-proof command or smoke extension**

Write the narrowest proof that currently fails because no timer interrupt path
exists yet and because the current smoke path exits too early to observe one.

Examples:

- expected timer tick count stays zero
- expected serial marker never appears

- [x] **Step 4: Verify the proof fails**

Run the smallest relevant command and confirm the failure is due to the missing
timer path rather than unrelated boot issues.

- [x] **Step 5: Commit**

```bash
git add src/lib.rs src/arch/x86_64/mod.rs src/arch/x86_64/timer.rs
git commit -m "feat(timer): define first timer vector and state"
```

## Task 2: Take Explicit Ownership Of The Legacy PIC

**Files:**

- Create: `src/arch/x86_64/pic.rs`
- Modify: `src/lib.rs` or `src/main.rs` for init ordering if needed

- [x] **Step 1: Write the failing initialization proof**

Add the narrowest proof that explicit PIC ownership is missing, likely a serial
stage marker expectation around interrupt-controller init on the normal boot
path in `lib.rs`.

- [x] **Step 2: Verify the proof fails**

Run:

```bash
make smoke
```

Expected:

- the new controller stage marker is absent because the path is not implemented

- [x] **Step 3: Implement conservative PIC masking or disable**

Add one explicit helper that:

- masks or disables the PIC deterministically
- documents the narrow ownership transition clearly

Do not add APIC logic here.

- [x] **Step 4: Verify normal boot still succeeds**

Run:

```bash
make smoke
make faults
```

Expected:

- normal boot still succeeds
- destructive fault paths still behave identically

- [x] **Step 5: Commit**

```bash
git add src/lib.rs src/main.rs src/arch/x86_64/pic.rs
git commit -m "feat(irq): add explicit PIC handling"
```

## Task 3: Add LAPIC Mapping And Initialization

**Files:**

- Create: `src/arch/x86_64/apic.rs`
- Modify: `src/arch/x86_64/mod.rs`
- Modify: `src/lib.rs`
- Modify: memory-track mapping files only as needed for MMIO support

- [x] **Step 1: Write the failing LAPIC-init proof**

Add the narrowest proof that LAPIC init is missing, such as an expected serial
marker after the MMIO mapping and controller setup stage.

- [x] **Step 2: Verify the proof fails**

Run the timer-proof command or extended smoke path and confirm the new marker is
absent for the expected reason.

- [x] **Step 3: Implement LAPIC MMIO access**

Use the memory track’s runtime mapping helper to map the LAPIC page, then add
register access helpers for only the registers currently needed.

Prefer xAPIC MMIO first unless a clear x2APIC simplification is established.

- [x] **Step 4: Implement minimal LAPIC initialization**

Bring up only what the timer path needs:

- LAPIC enable path
- spurious-interrupt vector setup if required
- EOI support

- [x] **Step 5: Verify controller init and destructive-fault stability**

Run:

```bash
make smoke
make faults
```

Expected:

- normal boot still succeeds
- LAPIC init markers appear
- destructive fault scenarios remain intact

- [x] **Step 6: Commit**

```bash
git add src/lib.rs src/arch/x86_64/mod.rs src/arch/x86_64/apic.rs src/arch/x86_64/paging.rs src/memory/mod.rs
git commit -m "feat(apic): initialize LAPIC through runtime MMIO mapping"
```

## Task 4: Install The Timer IRQ Handler

**Files:**

- Modify: `src/arch/x86_64/idt.rs`
- Modify: `src/arch/x86_64/timer.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Write the failing timer-interrupt proof**

Add the narrowest proof that the timer vector still does not fire:

- a tick counter remains zero, or
- an expected “first timer tick” marker never appears

- [x] **Step 2: Verify the proof fails**

Run:

```bash
<timer-proof command>
```

Expected:

- no timer proof appears because the handler is not installed yet

- [x] **Step 3: Add the timer vector to the IDT**

Install the timer handler on the chosen IRQ vector without changing the current
fatal exception vectors or IST assignments.

- [x] **Step 4: Implement the minimal handler**

The handler should:

- update the small timer state
- optionally print a throttled marker or one-time confirmation
- send EOI through the LAPIC path
- return normally

- [x] **Step 5: Verify periodic interrupts are firing**

Run:

```bash
make smoke
<timer-proof command>
```

Expected:

- normal boot still reaches the stable boot marker
- serial output from the timer-proof path shows timer init success
- timer proof marker appears
- the normal boot path remains otherwise stable

- [x] **Step 6: Commit**

```bash
git add src/lib.rs src/arch/x86_64/idt.rs src/arch/x86_64/timer.rs
git commit -m "feat(timer): add periodic timer interrupt handler"
```

## Task 5: Add A Minimal Interrupt-Safe Spinlock

**Files:**

- Create: `src/sync/mod.rs`
- Create: `src/sync/spinlock.rs`
- Modify: `src/lib.rs`
- Modify: `src/arch/x86_64/timer.rs` if shared timer state needs protection

- [x] **Step 1: Write the failing synchronization proof**

Choose the narrowest proof that shared timer state needs controlled access.

This can be a design-level failing check in the harness or a minimal API test
that demonstrates the need for explicit lock discipline.

- [x] **Step 2: Verify the proof fails**

Confirm the failure corresponds to the missing synchronization primitive or API
boundary.

- [x] **Step 3: Implement the smallest interrupt-safe spinlock**

The primitive should:

- provide explicit acquire/release
- preserve or restore interrupt state intentionally if that is part of the
  chosen API
- avoid growing into a general synchronization subsystem

- [x] **Step 4: Protect the shared timer state if needed**

Use the spinlock only where the timer/IRQ path requires it.

- [x] **Step 5: Verify boot, timer, and fault paths**

Run:

```bash
make smoke
<timer-proof command>
make test
make faults
```

Expected:

- smoke boot still succeeds
- timer-proof path still succeeds
- harness still passes
- destructive fault scenarios still behave as designed

- [x] **Step 6: Commit**

```bash
git add src/lib.rs src/sync/mod.rs src/sync/spinlock.rs src/arch/x86_64/timer.rs
git commit -m "feat(sync): add minimal interrupt-safe spinlock"
```
