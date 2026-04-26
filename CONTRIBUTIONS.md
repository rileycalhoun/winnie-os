# CONTRIBUTIONS

## Purpose

This file defines the coding and contribution conventions for Winnie OS.

The project is an early-stage `no_std` x86_64 kernel. Changes should prefer clarity, debuggability, and architectural correctness over cleverness.

## General Rules

- Keep changes narrow and intentional.
- Prefer simple control flow over abstraction-heavy designs.
- Do not introduce dependencies unless they clearly reduce complexity or risk.
- Preserve higher-half boot, trap, and paging invariants unless the change is explicitly about those systems.
- Avoid broad refactors while core kernel subsystems are still being brought up.

## Rust Style

- Use stable, readable naming. Prefer descriptive names over abbreviations unless the abbreviation is architecture-standard.
- Keep modules focused. If a file starts carrying multiple responsibilities, split it.
- Prefer `const` and small helper types where they make architectural state easier to reason about.
- Keep `unsafe` blocks as small as possible.
- Add a short comment near every non-obvious `unsafe` block explaining the invariant being relied on.
- Do not hide low-level behavior behind unnecessary abstractions.
- Prefer explicit error paths over silent fallback behavior.

## `no_std` Kernel Conventions

- Assume allocation is unavailable unless a subsystem explicitly provides it.
- Avoid APIs that quietly depend on runtime facilities the kernel does not have yet.
- Keep panic paths simple.
- In destructive fault paths like `#PF` and `#DF`, prefer minimal output plus `hlt` over complex recovery or formatting logic.

## x86_64 And Assembly Style

- Treat `src/arch/x86_64/boot.rs` as architecture-critical code.
- Keep assembly comments focused on machine-state transitions, not obvious instruction-by-instruction narration.
- Document control-register writes, selector values, IST slots, and virtual-address constants when they matter.
- When changing paging layout, update the corresponding architecture notes in the Obsidian vault.
- Do not change boot-time memory layout casually; keep bootstrap sections, higher-half mappings, and stack mappings easy to audit.

## Fault And Trap Handling

- Match exception handler signatures to the architectural error-code behavior exactly.
- Keep `#PF` and `#DF` handlers minimal and robust.
- Distinguish clearly between kernel faults and future user faults.
- Prefer deterministic failure over partial recovery when machine state is already compromised.

## Documentation Style

- Update docs when behavior or layout changes materially.
- Keep architecture docs in `~/Documents/winnie-os/` aligned with the code.
- Favor concrete descriptions over aspirational language.
- Document what the system does now, what assumptions it relies on, and what is still missing.
- Keep the kernel thoroughly documented as it evolves.
- New kernel subsystems, major control-flow changes, memory-layout changes, syscall changes, trap-handling changes, and build-pipeline changes must be reflected in documentation.
- When a subsystem is added, document at least:
  - its purpose
  - its boundaries and responsibilities
  - key invariants
  - important control flow
  - current limitations
- Code comments should explain non-obvious machine-state assumptions and safety invariants, but they do not replace vault documentation.
- Prefer updating existing architecture and reference notes over leaving behavior undocumented until later.

## Testing And Verification

- Build before claiming success.
- For boot, trap, paging, or stack changes, prefer headless QEMU runs with fault logs when possible.
- Use the simplest test that proves the behavior you are changing.
- If a change is not fully verified, say so explicitly.

## Commit Style

- Use short, specific commit messages.
- Good examples:
  - `feat: add initial userspace syscall boundary`
  - `fix: route page faults to IST2`
  - `docs: add current boot pipeline note`

- Avoid vague messages like:
  - `misc changes`
  - `update stuff`

## Non-Goals For Contributions

- Do not add GUI-oriented assumptions.
- Do not optimize prematurely for multitasking, networking, or storage before the current milestone needs them.
- Do not replace understandable low-level code with abstraction layers that make debugging harder.
