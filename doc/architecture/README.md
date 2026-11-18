# CORE Architecture Documentation

## Overview

This directory contains the architectural plan for introducing **CORE** — a freestanding standard library layer for the Fossil language. CORE provides memory operations, formatting, error handling, allocation contracts, and I/O abstractions without platform dependencies. MACHINE is refactored to build *on top of* CORE.

## Syntax Conventions

| Concept | Syntax | Example |
|---------|--------|---------|
| Generics | `[T]` not `<T>` | `Slice[T]`, `Option[T]` |
| Tagged unions | `[T, E]` | `[i32, Error]` = Result |
| Slices | `slice[T]` | `slice[u8]` |
| Arrays | `[T; N]` | `[u8; 32]` |

## Phases

| Phase | Document | Description |
|-------|----------|-------------|
| 0 | `00_overview.md` | High-level vision and conventions |
| 1 | `01_phase1_foundation.md` | Directory structure, module loader |
| 2 | `02_phase2_core_modules.md` | `core.mem`, `core.slice`, `core.err` |
| 3 | `03_phase3_traits.md` | Trait system (Write, Format, Allocator) |
| 4 | `04_phase4_io_fmt_panic_alloc.md` | I/O, formatting, panic, allocator |
| 5 | `05_phase5_atomics.md` | Atomic operations (optional) |
| 6 | `06_phase6_machine_refactor.md` | Refactor MACHINE on CORE |
| 7 | `07_phase7_compiler_changes.md` | Parser, semantic, codegen, intrinsics |
| 8 | `08_phase8_testing_integration.md` | Tests, CI, regression checklist |

## Layer Architecture

```
User Code (.fsl)
       │
       ▼
┌──────────────────┐
│      CORE        │  ← Freestanding, no platform deps
│  (src/core/)     │
│  - core.mem      │
│  - core.slice    │
│  - core.io       │
│  - core.fmt      │
│  - core.err      │
│  - core.panic    │
│  - core.alloc    │
│  - core.atomic   │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│     MACHINE      │  ← Platform-specific
│  (src/machine/)  │
│  - core          │  (re-exports CORE)
│  - win32         │  (raw Windows API)
│  - alloc         │  (libc malloc/free)
│  - io            │  (Write impl for handles)
│  - thread        │
│  - time          │
│  - file          │
│  - process       │
└──────────────────┘
```

## Implementation Order

1. **Phase 1** — Directory structure + module loader for CORE
2. **Phase 2** — `core.mem`, `core.slice`, `core.err` (no traits needed)
3. **Phase 3** — Trait system in compiler (parser, semantic, codegen)
4. **Phase 4** — `core.io` (Write trait), `core.fmt` (formatting), `core.panic`, `core.alloc`
5. **Phase 5** — `core.atomic` (optional)
6. **Phase 6** — Refactor MACHINE modules to use CORE
7. **Phase 7** — Compiler changes (macros, intrinsics, panic runtime)
8. **Phase 8** — Tests, CI, regression verification

## Key Design Decisions

1. **No `Result` type** — Use primitive tagged unions `[T, E]`
2. **No `Option` type** — Use `[T, void]` or dedicated `Option[T]` union
3. **Generics as `[T]`** — Consistent with array syntax
4. **Slices as `slice[T]`** — Fat pointer: `{ ptr: ptr[T], len: usize }`
5. **Traits via vtables** — Dynamic dispatch via fat pointers
6. **Format at compile time** — `format!` macro expands to direct calls

## Status Tracking

- [ ] Phase 1: Foundation
- [ ] Phase 2: Core Modules
- [ ] Phase 3: Traits
- [ ] Phase 4: IO/Fmt/Panic/Alloc
- [ ] Phase 5: Atomics
- [ ] Phase 6: MACHINE Refactor
- [ ] Phase 7: Compiler Changes
- [ ] Phase 8: Testing & Integration

---

## Next Step

Start **Phase 1**: Create `src/core/` directory and extend module loader for `CORE` prefix.