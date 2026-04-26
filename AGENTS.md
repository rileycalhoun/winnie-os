# AGENTS

## Start Here

Read [CONTRIBUTIONS.md](./CONTRIBUTIONS.md) before making changes. Its code-style and contribution rules apply to all agents working in this repository.

## Agent Priorities

- Preserve correctness of the boot path first.
- Prefer small, reviewable changes.
- Keep architecture-critical code easy to audit.
- Leave the repository easier to understand than you found it.

## Repository-Specific Advice

### 1. Treat Boot And Trap Code As High Risk

The most fragile parts of the current codebase are:

- `src/arch/x86_64/boot.rs`
- `src/arch/x86_64/idt.rs`
- `linker.ld`

Changes to these files should be conservative and easy to reason about. If you touch one of them, verify that the surrounding assumptions still hold.

### 2. Respect Current Architectural Invariants

The repository currently depends on these invariants:

- the kernel runs in the higher half
- low bootstrap sections remain loadable by GRUB
- the kernel stack is separate from PF and DF IST stacks
- `#PF` uses IST2
- `#DF` uses IST1
- destructive fault paths stay simple enough to survive damaged stack state

Do not casually break these invariants while working on unrelated tasks.

### 3. Prefer Minimal Fault Handlers

When working on page faults, double faults, or other destructive tests:

- prefer `println!` plus `hlt`
- avoid routing those paths through complex panic or formatting machinery unless the task is explicitly about panic infrastructure

### 4. Use The Right Verification Path

For changes involving:

- paging
- higher-half mappings
- TSS or IST setup
- exception handlers
- stack guards

prefer headless QEMU runs and fault-log inspection over guesswork.

### 5. Keep Docs In Sync

This repo has a paired Obsidian vault at:

- `~/Documents/winnie-os/`

If you materially change:

- boot flow
- memory layout
- trap handling
- build pipeline
- milestone implementation state

update the relevant vault notes.

### 6. Do Not Over-Engineer Early Milestones

Winnie OS is still in foundation work. Avoid introducing:

- scheduler complexity
- premature abstractions
- generalized subsystems before the milestone needs them

Build the narrowest working vertical slice first.

## Expected Workflow

1. Read the task and inspect the relevant code.
2. Check `CONTRIBUTIONS.md`.
3. Make the smallest change that solves the problem.
4. Verify with the most relevant build or boot path.
5. Update documentation if the architecture or workflow changed.
6. Report clearly what was changed, what was verified, and what remains unverified.

## If You Are Unsure

- choose the simpler implementation
- preserve existing behavior
- add a short note explaining the uncertainty
- avoid speculative refactors
