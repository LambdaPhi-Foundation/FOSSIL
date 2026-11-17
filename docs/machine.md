# MACHINE System Design — Carnax

Status: Design draft
Target: Stage One (LLVM-backed compiler)

This document defines the `MACHINE` / `NO_MACHINE` platform model: how Carnax programs are
set up by the underlying OS/toolchain so the programmer does not have to manually write
`entry()` or startup code, while never silently pulling in a C runtime, allocator, or
standard library.

## 1. Core idea: hosted vs freestanding

Yes — we extract inspiration from C's hosted/freestanding split, but we make the boundary
exact and explicit:

| | `MACHINE` (hosted) | `NO_MACHINE` (freestanding) |
|---|---|---|
| Entry point | Toolchain startup calls `main` | User defines the entry symbol |
| Startup code | Provided by toolchain package | None; user supplies |
| OS facilities | argc/argv, env, stdio, heap optional | None assumed |
| C runtime | Optional, opt-in, explicitly linked | None |
| Standard library | Optional package, not mandatory | None |
| Linker script | Toolchain-provided default | User-provided or target default |

`MACHINE` does **not** mean "link against libc by default". It means "this program expects
an operating-system environment; set up that environment before `main` runs". The exact
services and how they are linked are declared in the toolchain configuration — never implied
by the language keyword alone.

## 2. The `main` convention

### 2.1 Minimal form

```relic
MACHINE

FN main(): i32
    RETURN 0
ENDFN
```

The toolchain startup calls `main()` with no arguments and treats the return value as the
process exit code.

### 2.2 Full form (OS-facing)

```relic
MACHINE

FN main(argc: i32, argv: ptr[ptr[u8]]): i32
    ...
ENDFN
```

The startup places the argument count and argument vector into `argc`/`argv` before
branching to `main`. `argv` is a pointer to an array of `ptr[u8]` (C-style `char**`),
terminated by a null pointer.

### 2.3 Entry symbol

- `MACHINE`: the toolchain startup object exports the platform entry symbol (e.g. `_start`
  on ELF, `WinMainCRTStartup` on Windows, etc.) and calls `@main` from Carnax. The Carnax
  compiler emits `@main` with external linkage.
- `NO_MACHINE`: no entry symbol is assumed. The programmer writes `FN entry(): void` or any
  named function and marks it as the entry via toolchain configuration (linker `--entry`
  or equivalent). The compiler emits whatever function the user designates.

## 3. What `MACHINE` provides (and what it does not)

### Provided by the toolchain startup (opt-in per platform package)

1. **Process entry**: the platform-specific entry symbol that the OS loader jumps to.
2. **Argument/environment setup**: `argc`, `argv`, and environment pointer passed to `main`
   (full form only).
3. **Standard streams**: `stdin`, `stdout`, `stderr` initialized as file descriptors /
   file handles before `main` runs.
4. **Exit forwarding**: the return value of `main` is passed to the OS exit system call.
5. **Heap (optional)**: if the platform package includes an allocator, the heap is
   initialized before `main`. Not automatic — declared in the package.

### NOT provided (Carnax principles)

- No implicit garbage collector.
- No mandatory standard library.
- No implicit heap allocation.
- No implicit `libc` linkage.
- No hidden runtime initialization beyond what the platform package explicitly declares.

## 4. Toolchain configuration (explicit, not implicit)

Because `MACHINE` must not silently imply a particular C runtime or startup object, the
platform services are declared in a **toolchain configuration file** (TOML-like, format
TBD):

```toml
[target]
triple = "x86_64-pc-windows-msvc"

[machine]
startup = "crt0-carnax.obj"          # startup object provided by platform package
entry_symbol = "main"                # symbol the startup calls
linker_script = "machine-default.ld" # optional

[machine.services]
args = true                          # provide argc/argv to main
env = true                           # provide environment
stdio = true                         # initialize stdin/stdout/stderr
heap = "default"                     # heap allocator name, or "none"

[machine.libs]
# Explicit libraries to link (none are automatic)
# libc = "msvcrt.lib"   # only if explicitly requested
```

The compiler reads this configuration when `-machine` mode is active and passes the
appropriate startup object, linker flags, and entry symbol to the linker. The language
itself never hardcodes a path to `libc` or `crt0`.

## 5. `EXIT` builtin vs `RETURN` from `main`

Two ways to terminate a `MACHINE` program:

```relic
FN main(): i32
    RETURN 0          ; return value becomes exit code
ENDFN
```

```relic
FN main(): i32
    EXIT(0)           ; builtin: terminates process immediately with code
ENDFN
```

`EXIT(code: i32): void` is a compiler intrinsic that lowers to the platform exit syscall
(or `exit()` from the C runtime if the platform package provides it). It does not run
local destructors or drops — it is an immediate termination. `RETURN` from `main` runs
normal scope-exit cleanup (drops for `own` values) before the exit code is forwarded.

`EXIT` is available in both `MACHINE` and `NO_MACHINE` modes (in `NO_MACHINE` it maps to
whatever the platform package defines as "halt", or is rejected if no halt exists).

## 6. Standard streams and environment

If the platform package declares `stdio = true`, the compiler provides three builtin
values:

```relic
VAR stdin:  ptr[FILE] = STDIN
VAR stdout: ptr[FILE] = STDOUT
VAR stderr: ptr[FILE] = STDERR
```

`STDIN`, `STDOUT`, `STDERR` are compiler intrinsics lowering to the appropriate global
pointers set up by startup. `FILE` is an opaque type whose layout is defined by the
platform package, not by Carnax.

The environment block (if `env = true`) is accessible via `ENV(): ptr[ptr[u8]]`, lowering
to the startup-provided environment pointer.

## 7. `no_heap` interaction

`no_heap` is orthogonal to `MACHINE`:

| | `MACHINE` | `NO_MACHINE` |
|---|---|---|
| `no_heap` off | Heap available if platform package declares one | Heap only if user supplies |
| `no_heap` on | Heap rejected even if `MACHINE` | Heap rejected |

A `MACHINE` + `no_heap` program runs under an OS but is forbidden from calling any
heap-allocation intrinsic. The semantic analyzer enforces this; the codegen backend never
emits `malloc` in `no_heap` mode.

## 8. Linking and output

The compiler driver:

1. Parses the source, sees the top-level `MACHINE` or `NO_MACHINE` declaration.
2. Loads the matching toolchain configuration.
3. Emits LLVM IR → object file.
4. Links with:
   - The user's object file.
   - The platform startup object (for `MACHINE`).
   - Explicitly declared libraries only (never implicit `libc`).
   - The platform-provided linker script or defaults.

Example link command (illustrative):

```
ld.lld \
  -o program.exe \
  crt0-carnax.obj        # platform startup (MACHINE only)
  program.o              # user code
  -entry main            # entry symbol (MACHINE only)
  -L platform/lib        # platform package libs
```

## 9. Open decisions (deferred to bootstrapped stage)

- Exact format of the toolchain configuration file.
- Whether platform packages can declare additional builtins beyond `STDIN/STDOUT/STDERR`.
- How `atexit` / `at_quick_exit` are represented (if at all in stage one).
- How multiple entry points or `WinMain`-style GUI entry points are handled on Windows.
- Whether `main` can return `[i32, bool]` (multiple returns) and how that maps to exit codes.

## 10. Summary

`MACHINE` = "give me an OS environment; set up entry, args, stdio, and forward my exit
code". It is inspired by C's hosted model but differs critically: **nothing is implicit**.
The startup object, allocator, and libraries are all declared in toolchain configuration.
`NO_MACHINE` = "I am the startup; give me nothing". The language keyword chooses the path;
the toolchain configuration fills in the details.
