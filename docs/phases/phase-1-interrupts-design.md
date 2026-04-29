# Phase 1 Core Interrupts And Timers Design

## Purpose

Phase 1 interrupt work turns Winnie OS from a kernel that can survive fatal
exceptions into a kernel that owns interrupt routing and a reliable periodic
timer.

This design stays intentionally narrow:

- preserve the current audited fault-handler and IST structure
- keep `#PF` on IST2 and `#DF` on IST1
- add one explicit x86_64 interrupt-controller path
- add one reliable timer interrupt source
- add only the synchronization primitive needed to make that path safe

Phase 1 interrupts do not introduce scheduling, SMP, userspace interrupt
delivery, or broad driver infrastructure. Their job is to establish the first
nonfatal recurring interrupt path the scheduler will later depend on.

## Current Baseline

The repository currently provides:

- a loaded IDT with fixed fatal handlers for:
  - divide error
  - invalid opcode
  - double fault
  - general protection fault
  - page fault
- TSS-backed IST switching for PF and DF
- serial-visible deterministic panic and fault markers
- scripted QEMU verification for smoke, harness, and destructive fault lanes

It does not yet provide:

- PIC disable or masking policy owned by the kernel
- LAPIC or x2APIC ownership
- an installed hardware interrupt vector for a timer
- an interrupt acknowledgment path
- a periodic timer source
- interrupt-safe synchronization primitives

## Selected Decisions

Phase 1 interrupts are based on the following decisions:

- keep fatal exception handlers minimal and separate from timer/IRQ paths
- start with a conservative x86_64 controller path:
  - explicitly mask or disable the legacy PIC
  - use the local APIC timer as the durable timer direction
- defer IOAPIC routing until an external device path truly needs it
- prefer xAPIC MMIO first over x2APIC unless x2APIC materially reduces risk in
  QEMU and the current single-core setup
- add one timer interrupt vector above the CPU exception range
- add one minimal interrupt-safe spinlock only when timer/IRQ code needs shared
  state

## Goals

Phase 1 interrupts should deliver:

- explicit legacy PIC handling owned by the kernel
- LAPIC initialization sufficient for a timer interrupt
- one installed timer interrupt vector in the IDT
- deterministic periodic timer interrupts in QEMU
- correct interrupt acknowledgment / end-of-interrupt handling
- one minimal interrupt-safe spinlock primitive
- serial-visible proof that timer interrupts are firing

## Non-Goals

Phase 1 interrupts should not:

- add a scheduler
- enable preemptive task switching
- implement SMP startup
- route device interrupts through IOAPIC unless strictly required
- generalize trap handling across architectures prematurely
- broaden panic or fatal fault machinery

## Architectural Constraints

The design must preserve these existing invariants:

- the kernel remains in the higher half
- `#PF` remains on IST2
- `#DF` remains on IST1
- fatal exception paths remain minimal and auditable
- destructive fault tests remain valid after timer/IRQ work lands

The design also adopts these Phase 1 interrupt constraints:

- single-core correctness first
- serial logging remains the primary verification channel
- LAPIC MMIO mapping depends on the Phase 1 memory mapper rather than ad-hoc
  bootstrap mapping changes
- synchronization primitives must stay smaller than a full kernel locking layer

## Proposed Architecture

Phase 1 interrupts are intentionally split into four bounded units.

### 1. Interrupt Controller Ownership

The kernel should explicitly take ownership of interrupt-controller state rather
than inheriting whatever GRUB or firmware happened to leave configured.

The first controller step is conservative:

- mask or disable the legacy PIC so it does not interfere with the future APIC
  path
- initialize the local APIC in one explicit x86_64 module

This makes the ownership boundary obvious:

- before Phase 1: exception-only kernel, no owned IRQ path
- after Phase 1: kernel explicitly owns its first IRQ-capable controller state

### 2. Timer Interrupt Path

The first timer goal is not timekeeping sophistication. It is a reliable
periodic interrupt the kernel can observe and acknowledge.

Required pieces:

- one timer vector constant above the CPU exception range
- one IDT handler for that vector
- LAPIC timer configuration
- interrupt acknowledgment path
- serial-visible proof that the interrupt fired

The handler should stay minimal:

- increment one counter or set one flag
- optionally print a throttled marker or log summary
- acknowledge the interrupt

It should not invoke scheduling or broad subsystem logic yet.

### 3. Synchronization Primitive

Phase 1 needs only one narrow synchronization primitive:

- an interrupt-safe spinlock or equivalent guard for shared timer/IRQ state

This primitive exists only because timer interrupts introduce the first real
concurrency boundary between ordinary kernel control flow and asynchronous IRQ
delivery.

It should be:

- single-core correct first
- explicit about interrupt-disable behavior
- small enough to audit

Phase 1 does not need mutexes, wait queues, or broader blocking primitives.

### 4. Verification Layer

Timer/IRQ work must remain easy to diagnose in QEMU.

The verification strategy should therefore include:

- serial-visible stage markers around controller and timer init
- one deterministic success criterion for periodic interrupts
- reuse of `make smoke`, `make test`, and `make faults`
- at least one headless timer-specific verification path that does not depend on
  a future scheduler

Normal smoke boot remains useful for proving that timer work did not break
ordinary boot, but it is not by itself a sufficient proof that periodic timer
interrupts are firing. The existing smoke runner currently exits as soon as it
sees the stable normal boot marker, so Phase 1 interrupt verification should
either extend that runner or add a dedicated timer-proof command that waits for
one serial-visible timer marker before terminating QEMU.

## Detailed Design

## PIC And APIC Direction

The kernel should make one clear decision about the interrupt-controller
baseline:

- legacy PIC is not the durable path
- LAPIC timer is the first durable interrupt source

That means Phase 1 should explicitly mask or disable the PIC and then move to a
LAPIC-owned timer path. IOAPIC routing can wait because Phase 1 does not yet
need external device interrupts.

This keeps the design narrow and aligned with later scheduler work:

- local timer first
- external device routing later

## LAPIC Mapping And Access

The LAPIC path depends on runtime memory ownership:

- LAPIC MMIO needs an explicit kernel mapping
- that mapping should be provided by the Phase 1 memory mapper

This is an important dependency boundary:

- memory phase provides “map one MMIO page”
- interrupt phase provides “interpret that MMIO page as LAPIC registers”

The interrupt design should not reopen bootstrap paging just to reach the LAPIC.

## Initialization Placement

Phase 1 interrupt-controller and timer initialization should happen only on the
normal boot path after the Phase 1 memory layer can provide the required MMIO
mapping support.

That means:

- `main.rs` should continue to own only the common early bring-up shared by all
  runtime lanes:
  - serial init
  - IDT init
  - scenario dispatch
- the normal boot path in `lib.rs` should become responsible for:
  - PIC ownership
  - LAPIC mapping and initialization
  - timer initialization

The bootable harness and destructive fault scenarios should not automatically
inherit the periodic timer path unless a later milestone explicitly needs that.
This keeps harness and fault verification deterministic while isolating timer
bring-up risk to the normal runtime path first.

## Timer Handler Behavior

The timer handler should be smaller than the fatal handlers plus success-exit
machinery used by destructive tests.

Recommended behavior:

- bump a counter in a tiny shared state structure
- optionally print one marker every N ticks or only once after the first tick
- acknowledge the LAPIC interrupt
- return cleanly

This proves the path without overloading serial or building scheduler behavior
too early.

## Spinlock Scope

The Phase 1 spinlock should only solve the immediate problem:

- protect small shared timer state from concurrent access between ordinary code
  and the timer interrupt path

It should not try to solve:

- multi-CPU scaling
- fairness
- lock ordering across subsystems
- blocking synchronization

The minimal acceptable behavior is:

- acquire with interrupts disabled on the current CPU context if required
- release and restore the previous interrupt state explicitly

## Verification Strategy

Phase 1 interrupts should use these proofs:

- `make smoke`
  - prove normal boot still succeeds after controller/timer init
- one dedicated timer-proof command or extended smoke mode
  - prove at least one periodic timer interrupt fired before QEMU exits
- serial-visible initialization markers
  - prove PIC/APIC init order
- `make faults`
  - re-run the destructive suite after timer/IDT changes to confirm PF/DF and
    fatal-handler behavior remain intact

If timer bring-up introduces instability, the first debugging path should be
headless QEMU plus serial output and `qemu.log`, not speculative refactors.

## Risks And Mitigations

### Risk: Controller Scope Creep

Interrupt work can easily expand into “full device interrupt architecture” too
early.

Mitigation:

- keep the first target to PIC handling plus LAPIC timer only
- defer IOAPIC and external IRQ routing until a later milestone actually needs
  them

### Risk: Timer Handler Doing Too Much

If the first timer handler tries to behave like a scheduler tick, it will
introduce too many new failure modes at once.

Mitigation:

- make the first timer handler minimal and observational
- defer scheduling semantics to Phase 2

### Risk: Breaking Fatal Fault Paths

IDT or interrupt-controller changes can accidentally destabilize the audited PF
and DF paths.

Mitigation:

- keep fatal exception vectors and timer IRQ vectors clearly separated
- re-run destructive fault verification after every material IDT/IRQ change

## Exit Criteria Satisfaction

This design satisfies the Phase 1 interrupt portion of the roadmap by producing:

- explicit kernel-owned interrupt-controller state
- a reliable timer interrupt source
- minimal interrupt-safe synchronization
- a clean foundation for scheduler work in Phase 2
