# CORE Architecture Overview

## Vision

Introduce **CORE** — a freestanding standard library layer providing memory operations, formatting, error handling, allocation contracts, and I/O abstractions without platform dependencies. MACHINE is refactored to build *on top of* CORE.

## Syntax Conventions

| Concept | Syntax | Example |
|---------|--------|---------|
| Generics | `[T]` not `<T>` | `Slice[T]`, `Option[T]` |
| Tagged unions | `[T, E]` | `[i32, Error]` = Result |
| Slices | `slice[T]` | `slice[u8]` |
| Arrays | `[T; N]` | `[u8; 32]` |

## Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    User Code (.fsl)                         │
└────────────────────────────┬────────────────────────────────┘
                             │
                    ┌────────▼────────┐
                    │   CORE          │  ← Freestanding, no platform deps
                    │  (src/core/)    │
                    │  - core.mem     │
                    │  - core.slice   │
                    │  - core.io      │
                    │  - core.fmt     │
                    │  - core.err     │
                    │  - core.panic   │
                    │  - core.alloc   │
                    │  - core.atomic  │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │   MACHINE       │  ← Platform-specific
                    │  (src/machine/) │
                    │  - core (reexport)        │
                    │  - win32 / linux / macos  │
                    │  - alloc (libc impl)      │
                    │  - io (Write impl)        │
                    │  - thread/time/file/proc  │
                    └───────────────────────────┘
```

## Key Design Decisions

1. **No `Result` type** — Use primitive tagged unions `[T, E]`
2. **No `Option` type** — Use `[T, void]` or dedicated `Option[T]` union
3. **Generics as `[T]`** — Consistent with array syntax
4. **Slices as `slice[T]`** — Fat pointer: `{ ptr: ptr[T], len: usize }`
5. **Traits via vtables** — Dynamic dispatch via fat pointers
5. **Format at compile time** — `format!` macro expands to direct calls

## Module Naming

- CORE modules: `CORE.mem`, `CORE.slice`, `CORE.io`, etc.
- MACHINE modules: `MACHINE.core`, `MACHINE.win32`, `MACHINE.alloc`, etc.
- User imports: `USE CORE.io`, `USE MACHINE.io`

## Build Integration

- CORE modules loaded by compiler at compile time (like MACHINE)
- Available in both `MACHINE` and `NO_MACHINE` modes
- No external dependencies — pure Fossil + LLVM intrinsics