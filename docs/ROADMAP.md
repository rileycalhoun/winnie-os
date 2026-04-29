# Winnie OS Roadmap

## Purpose

This roadmap defines a practical path from the current early x86_64 higher-half kernel bring-up to a server-focused operating system with:

- no GUI
- a POSIX-oriented terminal environment
- support for running Rust and C programs
- x86_64 and ARM64 support
- Ethernet networking first, with Wi-Fi later

The intent is to keep the project narrow, auditable, and milestone-driven. Winnie OS should become a text-first, networked server OS rather than a desktop environment.

## Current State

As of this roadmap, the repository provides:

- GRUB-loaded x86_64 higher-half boot
- early paging setup
- a TSS with dedicated IST stacks
- a minimal IDT with a few fatal exception handlers
- mirrored VGA and serial console output
- Multiboot2 memory-map parsing into a small owned kernel structure
- a bootable no-`std` kernel test harness
- dedicated panic and fault test scenarios
- scripted QEMU smoke, harness, and fault-test runners
- a terminal `hlt` path after startup

It does not yet provide:

- a physical memory manager
- a virtual memory subsystem beyond bootstrap tables
- timer interrupts or scheduling
- a syscall boundary
- userspace processes
- filesystems or storage drivers
- a POSIX terminal stack
- networking
- ARM64 bring-up

## Product Direction

Winnie OS should optimize for server use:

- text console, serial console, and remote administration first
- no windowing system, compositor, desktop shell, or GUI framework
- predictable kernel behavior over feature breadth
- simple boot and trap code
- portable kernel structure so x86_64 and ARM64 can share most higher-level code

## Guiding Principles

1. Keep x86_64 boot and trap paths conservative and easy to audit.
2. Build the smallest working vertical slice before adding subsystems around it.
3. Introduce architecture-neutral interfaces early enough to avoid hard-coding x86_64 assumptions everywhere.
4. Prefer static linking and simple userspace loading first; defer dynamic linking until later.
5. Use serial logs, headless QEMU, and deterministic test paths for kernel bring-up.
6. Treat terminal correctness, process isolation, and networking reliability as server features, not polish.

## Roadmap Summary

| Phase | Theme | Exit Criteria |
| --- | --- | --- |
| 0 | Foundation Hardening | Reliable debug/verification path for early kernel work |
| 1 | Memory And Core Interrupts | Kernel owns physical memory, timers, and interrupt routing |
| 2 | Kernel Execution Model | Preemption basics, scheduler, and kernel task model exist |
| 3 | Syscalls And Userspace | User processes can start and make basic syscalls |
| 4 | Filesystem And Program Loading | ELF binaries can be loaded from a real filesystem |
| 5 | POSIX Terminal Stack | TTY/PTY/session/process-group model works end-to-end |
| 6 | Rust/C Runtime Enablement | C and Rust programs can be built for and run on Winnie OS |
| 7 | Ethernet Networking | Wired networking, sockets, and remote login become usable |
| 8 | ARM64 Port | ARM64 reaches functional parity with the x86_64 server baseline |
| 9 | POSIX Gap Closure And Hardening | Stabilization, conformance work, and operational tooling |

## Phase 0: Foundation Hardening

### Goal

Turn the current boot-only kernel into a platform that is easy to debug and safe to evolve.

### Why this comes first

Right now the kernel can boot and print, but almost every next subsystem depends on stronger diagnostics and repeatable verification. Without that, later milestones will be slow and error-prone.

### Deliverables

- serial console output in addition to VGA text mode
- clearer boot-stage logging for paging, TSS, IDT, and later subsystem init
- bootloader memory map parsing
- a consistent kernel panic and fatal-fault reporting path that stays minimal
- scripted headless QEMU runs for normal boot and deliberate fault cases
- repository docs describing current bring-up flow, verification commands, and milestone status

### Exit criteria

- the kernel can emit logs over serial in headless QEMU
- destructive fault tests are easy to trigger and inspect
- boot-time memory regions are visible to the kernel in a structured form
- contributors can verify boot and fault behavior with a single documented command path

### Best next steps now

1. Finish documenting the now-scripted smoke, harness, and fault-test workflows.
2. Use the serial-first verification path as the default for Phase 1 memory and interrupt work.
3. Build the physical memory manager on top of the owned boot memory map.
4. Extend destructive test coverage only where later phases need new fault scenarios.

## Phase 1: Memory And Core Interrupts

### Goal

Move from bootstrap-only paging to a kernel-owned memory and interrupt foundation.

### Deliverables

- physical frame allocator
- kernel page allocator / virtual memory mapper
- explicit mapping helpers for kernel text, data, stacks, MMIO, and guard pages
- interrupt controller setup on x86_64
  - PIC disable or masking
  - APIC or x2APIC path
  - IOAPIC routing if needed for later devices
- timer interrupt source
  - PIT only if needed temporarily
  - LAPIC timer or HPET as the durable direction
- interrupt-safe spinlocks and minimal synchronization primitives
- kernel heap only when a subsystem truly needs dynamic allocation

### Design notes

- Keep early bootstrap page tables separate from the long-term VM subsystem so the transition remains auditable.
- Do not introduce a general heap before the frame allocator and mapper are trustworthy.
- Avoid SMP at first if it slows down the single-core bring-up path.

### Exit criteria

- kernel can allocate and free physical frames
- kernel can create and destroy mappings after boot
- timer interrupts fire reliably
- interrupt handling is stable enough to support a future scheduler

## Phase 2: Kernel Execution Model

### Goal

Introduce kernel tasks and scheduling so the system can advance beyond single-threaded bring-up.

### Deliverables

- kernel task abstraction
- per-task kernel stacks
- context-switch path
- simple scheduler
  - round-robin is sufficient initially
- timer-driven preemption
- sleep/wake primitives
- basic synchronization
  - mutex
  - wait queue
  - event or semaphore primitive
- architecture-neutral interfaces for:
  - trap entry
  - context switching
  - timer interrupts
  - CPU-local state

### Design notes

- This is the right time to start forcing architecture boundaries so the ARM64 port does not fight x86_64-specific assumptions later.
- Keep the scheduler minimal. Server capability matters more than advanced fairness policies early on.

### Exit criteria

- multiple kernel tasks can run and yield
- timer preemption works on x86_64
- core scheduler code is mostly architecture-neutral above the low-level switch/trap layer

## Phase 3: Syscalls And Userspace

### Goal

Create a minimal but real user/kernel boundary.

### Deliverables

- userspace address-space abstraction
- user page mapping and protection enforcement
- syscall entry path on x86_64
  - likely `syscall/sysret` eventually
  - `int 0x80` can be a temporary bring-up step if it reduces risk
- copy-in / copy-out helpers with robust fault handling
- thread and process primitives
- first user process launched from an in-memory image or initramfs
- minimal syscall set, focused on process and terminal bring-up
  - `write`
  - `read`
  - `exit`
  - `fork` or `posix_spawn`-oriented process creation path
  - `execve`
  - `mmap`
  - `brk` only if needed
  - `openat`, `close`, `dup`, `pipe`

### Design notes

- Aim for a small, coherent syscall surface rather than superficial POSIX breadth.
- Use a simple initramfs first so userspace bring-up does not wait on block storage.
- Process isolation and pointer validation need to be correct before terminal and network work starts building on top.

### Exit criteria

- the kernel can enter user mode and return through syscalls
- a tiny init process can execute and print through a kernel-provided console path
- user faults are distinguishable from kernel faults

## Phase 4: Filesystem And Program Loading

### Goal

Support loading real binaries and managing program-visible files.

### Deliverables

- VFS layer with narrow initial scope
- initramfs support if not already in place
- first writable filesystem
  - ext2 is a reasonable early target
  - a simpler custom read-only format is acceptable only as a temporary bring-up tool
- block-device path for virtualized environments
  - start with virtio-blk if possible
- ELF loader for statically linked user binaries
- file descriptor table model
- path resolution, permissions groundwork, and mount structure

### Design notes

- Do not chase a broad filesystem matrix early.
- Prefer virtualized-device-first drivers to reduce hardware complexity.
- Static ELF execution is enough for a long time; dynamic linker support should wait.

### Exit criteria

- kernel can mount an initial filesystem
- user programs can be loaded from disk or initramfs as ELF binaries
- standard file-descriptor operations work for regular files and pipes

## Phase 5: POSIX Terminal Stack

### Goal

Build the text-first user environment that defines the project’s server personality.

### Deliverables

- terminal device model
- TTY layer
- PTY master/slave support
- sessions, process groups, and controlling terminals
- line discipline
  - canonical mode
  - raw mode
  - echo
  - signals from special characters
- terminal-related syscalls and `ioctl`s needed for practical shell use
- serial-backed console integration
- a minimal terminal emulator path for local text consoles
- job control support sufficient for an interactive POSIX-style shell

### Design notes

- This phase is where “no GUI” becomes concrete: text console, serial console, and PTYs are first-class, while graphics remain out of scope.
- PTYs matter early because they become the foundation for shells, remote sessions, and later service supervision tooling.

### Exit criteria

- an interactive shell can run on a local or serial terminal
- pipelines, job control basics, and terminal modes behave predictably
- the terminal subsystem is strong enough to support remote login later

## Phase 6: Rust/C Runtime Enablement

### Goal

Make Winnie OS a practical target for Rust and C programs.

### Deliverables

- stable target definitions for Winnie OS
  - `x86_64-winnie-os`
  - `aarch64-winnie-os`
- toolchain story for C
  - Clang/LLVM target support
  - binutils or LLVM lld flow
  - libc integration
- toolchain story for Rust
  - target JSON and rustc support
  - `std` later, `core`/`alloc`-first if needed
- libc plan
  - start with a minimal libc port or musl-oriented porting path
- crt startup objects and linker conventions
- userspace build examples
  - hello world in C
  - hello world in Rust
  - shell-friendly utilities
- package/build workflow documentation

### Design notes

- Static linking is the right default early on.
- It is acceptable to support a smaller POSIX subset at first, as long as the ABI and syscall story are coherent.
- Prioritize enough libc/syscall coverage to build simple tools, a shell, and network utilities.

### Exit criteria

- small C programs can build and run on Winnie OS
- small Rust programs can build and run on Winnie OS
- the kernel/user ABI is documented and stable enough for basic userland growth

## Phase 7: Ethernet Networking

### Goal

Bring up practical server networking over wired Ethernet.

### Deliverables

- NIC driver for a virtualized device first
  - prefer virtio-net in QEMU
- kernel network buffer model
- Ethernet, ARP, IPv4, and ICMP
- UDP and TCP
- socket API with a POSIX-oriented surface
- DNS client support in userland or libc support layers
- initial remote administration tools
  - `ping`
  - simple DHCP or static IP configuration
  - remote shell path such as telnet for bring-up, then SSH later
- network test harnesses in QEMU

### Design notes

- Virtualized Ethernet first is the fastest path to a usable server OS.
- Wi-Fi should stay off the critical path. It adds firmware, scanning, authentication, and driver complexity that is unnecessary for early server milestones.
- Good socket semantics matter more than protocol breadth early on.

### Exit criteria

- two QEMU guests or a guest and host can exchange packets over virtio-net
- TCP sockets are usable from userland
- the system can support remote login and basic network services over Ethernet

## Phase 8: ARM64 Port

### Goal

Reach the same server-oriented baseline on ARM64 without cloning x86_64-specific kernel structure.

### Why this is later, but not last

ARM64 should not be the very first focus because the kernel still lacks stable architecture-neutral layers. It also should not be postponed until the end, because that would let x86_64 assumptions harden across the whole system.

### Deliverables

- `aarch64` target and build pipeline
- ARM64 bootstrap
  - exception level transitions
  - page-table bring-up
  - MMU enablement
  - stack and exception-vector setup
- ARM64 trap and interrupt entry
- ARM64 timer integration
- ARM64 context switching
- ARM64 syscall entry
- QEMU `virt` machine support as the primary bring-up target
- reuse of shared kernel subsystems for:
  - scheduler
  - VFS
  - process model
  - TTY
  - networking

### Recommended timing

Start active ARM64 bring-up once Phases 2 and 3 have produced:

- stable architecture-neutral scheduler and trap interfaces
- a minimal syscall model
- a clear userspace process abstraction

Do not wait for full POSIX completeness before starting ARM64 work.

### Exit criteria

- ARM64 boots into the higher-level kernel runtime
- ARM64 can run user processes and basic syscalls
- key server flows work on both architectures, even if device-driver breadth differs

## Phase 9: POSIX Gap Closure And Hardening

### Goal

Turn a functional server OS into a stable, better-specified one.

### Deliverables

- audit of syscall coverage versus targeted POSIX profile
- permission and credential model completion
  - users
  - groups
  - uid/gid semantics
- signals, process control, and wait semantics cleanup
- `poll`/`select`/`epoll` strategy
- better timekeeping and clocks
- service supervision / init improvements
- stronger test coverage
  - syscall tests
  - terminal behavior tests
  - filesystem tests
  - networking tests
  - cross-architecture boot and userland tests
- performance and reliability passes
- security review of kernel/user boundaries
- SSH, if desired, after socket and PTY infrastructure are stable

### Exit criteria

- Winnie OS can boot, run shells and services, and be administered remotely on both architectures
- terminal, filesystem, process, and socket behavior are consistent enough to support a modest POSIX-style userland
- the project has an explicit list of supported POSIX behaviors and known gaps

## Cross-Cutting Workstreams

These should progress alongside the main milestones rather than waiting for a single late phase.

### Documentation

- keep `~/Documents/winnie-os/` architecture notes aligned with major kernel changes
- add repo-local docs for build, boot, testing, and milestone status
- document invariants for each new subsystem before it spreads across the tree

### Verification

- maintain fast headless QEMU smoke tests
- add targeted regression tests for traps, paging, syscalls, and drivers
- make x86_64 and ARM64 boot verification symmetric once the ARM64 port exists

### Architecture Boundaries

- keep low-level architecture code in clearly bounded modules
- define shared kernel interfaces before adding a second implementation
- avoid leaking x86_64-specific register, trap, or page-table assumptions into generic code

### Userland Strategy

- prefer a small base system over an ambitious package ecosystem early
- bring up an init process, shell, and essential utilities before broad application support
- treat Rust and C toolchain usability as a product requirement, not an afterthought

## What Not To Do Early

- no GUI stack
- no compositor or desktop session model
- no Wi-Fi before Ethernet is solid
- no dynamic linking before static userland is reliable
- no broad driver matrix before QEMU-first device support is stable
- no heavy abstraction layers in boot, trap, or paging code
- no premature SMP or NUMA complexity unless a concrete milestone needs it

## Recommended Immediate Project Plan

If the project wants the best next steps from today’s codebase, prioritize this order:

1. Finish Phase 0 with serial logging, memory-map parsing, and repeatable QEMU verification.
2. Complete Phase 1 so the kernel owns memory management and timer interrupts rather than relying on bootstrap-only structures.
3. Build Phase 2 and Phase 3 as the first major vertical slice: scheduler, syscall entry, user mode, and a tiny init process from initramfs.
4. Add just enough filesystem and TTY infrastructure to support a shell and simple utilities.
5. Bring up virtio-net and sockets before attempting Wi-Fi or broad hardware support.
6. Start the ARM64 port once the architecture-neutral scheduler, trap, and syscall layers are real.

That sequence keeps the system focused on becoming a usable text-first server OS instead of spreading effort across too many partially built subsystems at once.
