# FOSSIL Language Design

## Revised Working Specification

**Status:** Design draft\
**Compiler stage:** LLVM-backed implementation\
**Future stage:** Self-hosted / bootstrapped implementation\
**Design priority:** Minimal mental model, explicit behavior,
systems-level control

------------------------------------------------------------------------

## 1. Project direction

FOSSIL is a systems programming language intended to provide direct
control over data, execution, memory, and machine resources without
requiring a large conceptual framework.

The language should have a small set of orthogonal features. Features
are not included merely because they are common in other languages. Each
must justify its impact on the programmer's mental model, compiler
complexity, and generated code.

### 1.1 Core principles

1.  **Imperative first.** Programs are understood as procedures that
    operate on and modify state.
2.  **Explicit behavior.** Avoid hidden allocation, implicit runtime
    services, implicit conversions, and surprising control flow.
3.  **Data-oriented design.** Data layouts are defined separately from
    the procedures that operate on them.
4.  **Minimal language surface.** Prefer a small number of general
    mechanisms over many specialized constructs.
5.  **Systems control.** Provide access to pointers, memory layout,
    calling conventions, and machine-specific operations.
6.  **Predictable compilation.** Language semantics must be defined
    independently of LLVM implementation details.
7.  **Optional infrastructure.** Platform services and libraries should
    not be mandatory parts of the language core.

### 1.2 What FOSSIL is not trying to be

The initial design does not aim to be:

-   A functional programming language.
-   An object-oriented language with classes, inheritance, or implicit
    receivers.
-   A language with a mandatory garbage collector or runtime.
-   A language that hides allocation or platform dependencies.
-   A language that exposes every LLVM feature as a language feature.

Functional programming concepts may still be useful as research tools
when studying semantics or compiler implementation. That does not mean
they belong in FOSSIL's programmer-facing model.

------------------------------------------------------------------------

## 2. Development stages

FOSSIL is planned in two broad stages. Their platform and library models
are intentionally different.

### 2.1 Stage One: LLVM-backed FOSSIL

The first compiler uses LLVM for code generation.

The initial platform distinction is:

-   `MACHINE`: use the machine/platform-facing path for programs that
    need platform or operating-system facilities.
-   `NO_MACHINE`: compile in the freestanding path without relying on an
    operating-system environment.

Illustrative declarations:

``` relic
MACHINE

FN main(): i32
    RETURN 0
ENDFN
```

``` relic
NO_MACHINE

FN entry(): void
    // User-provided startup and hardware setup.
ENDFN
```

These examples illustrate the intended distinction, not a finalized
grammar.

`NO_MACHINE` does not mean that the compiler has no target architecture.
LLVM still requires a target triple and data layout. It means that the
program is compiled without assuming an operating-system environment.

Likewise, `MACHINE` must not silently imply a particular C runtime,
startup object, allocator, or standard library. The exact platform
services and linkage behavior must be explicit in the toolchain
configuration and language specification.

The initial compiler may provide only a limited set of platform
integrations. A generalized user-defined standard-library and
platform-package system is not a requirement for this stage.

### 2.2 Stage Two: Bootstrapped FOSSIL

After the LLVM-backed language and compiler have become sufficiently
stable, FOSSIL can gain a pluggable library and platform-environment
system.

The intention is to let a project select the facilities appropriate to
its domain:

-   UEFI applications and bootloaders.
-   ARM and other embedded targets.
-   Games and multimedia applications.
-   Operating-system-facing applications.
-   Unusual or custom hardware.
-   User-defined environments.

Illustrative future syntax:

``` relic
USE core
USE uefi
USE uefi.graphics
```

``` relic
USE core
USE arm
USE arm.mmio
USE arm.interrupts
```

This is proposed syntax only. The package format, dependency resolver,
module system, and compatibility rules remain undecided.

The bootstrapped compiler may initially continue to emit LLVM IR.
Self-hosting and replacing LLVM are separate decisions.

### 2.3 Stage distinction

  -----------------------------------------------------------------------
  Concern                 LLVM-backed stage       Bootstrapped stage
  ----------------------- ----------------------- -----------------------
  Compiler implementation External implementation Compiler implemented in
                          using LLVM              FOSSIL

  Platform selection      `MACHINE` /             Extensible platform
                          `NO_MACHINE`            environment

  Custom library          Limited or manually     Pluggable libraries
  ecosystem               supplied                

  Target code generation  LLVM                    LLVM initially is
                                                  possible; other
                                                  backends are future
                                                  work

  Freestanding support    `NO_MACHINE`            Freestanding
                                                  environment remains
                                                  supported
  -----------------------------------------------------------------------

**Important:** The future package system must not be retroactively
assumed to exist in the first compiler.

------------------------------------------------------------------------

## 3. Programmer-facing mental model

The intended model is deliberately straightforward:

1.  Define data.
2.  Define procedures that operate on that data.
3.  Control execution explicitly.
4.  Choose how memory is obtained and released.
5.  Select the machine/platform facilities required by the program.

A program should not require the programmer to reason about implicit
object receivers, closure capture, lazy evaluation, or transformation
pipelines.

### 3.1 Data and procedures

Types describe data representation. Functions operate on values and
pointers to those values.

``` relic
STRUCT Point
    x: i32
    y: i32
ENDSTRUCT

FN move_point(p: ptr[Point], dx: i32, dy: i32): void
    p.x := p.x + dx
    p.y := p.y + dy
ENDFN
```

There is no implicit `self` parameter. The data being modified is passed
explicitly.

### 3.2 Imperative control flow

The initial control-flow model uses explicit blocks and statements.

``` relic
FN absolute_difference(a: i32, b: i32): i32
    VAR result: i32 = 0

    IF a > b
        result := a - b
    ELSE
        result := b - a
    ENDIF

    RETURN result
ENDFN
```

The language should favor a small set of familiar constructs:

-   Sequential statements.
-   Conditional branches.
-   Loops.
-   Function calls and returns.
-   Explicit state updates.

Expression-oriented control flow is not a design priority.

------------------------------------------------------------------------

## 4. Syntax conventions

The existing design favors keyword-delimited blocks rather than braces.
Retain that direction while standardizing spelling and casing.

### 4.1 Block delimiters

  Construct                      Opening keyword   Closing keyword
  ------------------------------ ----------------- -----------------
  Function                       `FN`              `ENDFN`
  Conditional                    `IF`              `ENDIF`
  Loop                           `WHILE`           `ENDWHILE`
  Counted loop                   `FOR`             `ENDFOR`
  Structure                      `STRUCT`          `ENDSTRUCT`
  Enumeration                    `ENUM`            `ENDENUM`
  Match statement, if retained   `MATCH`           `ENDMATCH`

`ELSE` is part of an `IF` block and closes with `ENDIF`.

The canonical spelling is uppercase for reserved block keywords. The
earlier mixture of `fn`, `END fn`, `end if`, and uppercase variants
should not be carried forward.

### 4.2 Statement termination

Proposed rules:

-   A newline terminates a statement.
-   Multiple statements on one line are not supported initially.
-   Semicolons are not required.
-   Whether semicolons should be accepted as optional separators remains
    open; they should not be retained solely for backward compatibility.

### 4.3 Assignment and bindings

``` relic
VAR count: i32 = 0
LET limit: i32 = 100

count := count + 1
```

-   `VAR` declares a mutable binding.
-   `LET` declares an immutable binding, if retained in the final core.
-   `:=` updates an existing mutable binding or assignable location.
-   Initialization and reassignment must be distinct in the grammar and
    semantic analysis.

### 4.4 Naming and syntax consistency

The language specification must settle:

-   Whether identifiers are case-sensitive.
-   Whether all keywords are uppercase.
-   Whether type declarations use `:` consistently.
-   Whether function types are part of the initial language.
-   Whether square brackets are reserved for type parameters, arrays,
    and tuples.
-   Whether semicolons are rejected or accepted as optional separators.

No compiler implementation should depend on contradictory examples.

------------------------------------------------------------------------

## 5. Core type system

The type system should provide enough structure for systems software
without requiring a large abstraction framework.

### 5.1 Primitive types

Initial candidates:

  Type                        Meaning
  --------------------------- -------------------------------
  `bool`                      Boolean
  `i8`, `i16`, `i32`, `i64`   Signed fixed-width integers
  `u8`, `u16`, `u32`, `u64`   Unsigned fixed-width integers
  `f32`, `f64`                Floating-point values
  `void`                      No returned value

The exact integer overflow, floating-point, alignment, and conversion
rules must be specified. They must not be inferred from whichever LLVM
instruction happens to be emitted.

### 5.2 Product types

Structures describe product data layouts.

``` relic
STRUCT Vector2
    x: f32
    y: f32
ENDSTRUCT
```

The initial design favors plain data structures. Methods, implicit
receivers, inheritance, and virtual dispatch are not required.

### 5.3 Sum types

Tagged enumerations are useful for representing alternatives, state, and
error categories.

``` relic
ENUM DeviceState
    Offline
    Ready
    Failed
ENDENUM
```

Payload-bearing variants may be supported:

``` relic
ENUM ReadResult
    Success(i32)
    EndOfFile
    Failure(i32)
ENDENUM
```

The representation, tag size, payload alignment, and layout guarantees
must be specified before promising a particular memory layout.

Pattern matching is not automatically part of the core merely because
sum types exist. A conventional `MATCH` statement is a candidate, but
its necessity and complexity should be evaluated separately. A
`SWITCH`-style construct could also be considered.

### 5.4 Arrays and aggregate types

Arrays and fixed-capacity collections are important for systems
programming.

The design must distinguish:

-   Fixed-size arrays.
-   Dynamically sized sequences.
-   Slices or pointer-plus-length views.
-   Arrays embedded in structures.
-   Caller-provided storage.

The notation and exact semantics are not finalized.

### 5.5 Generics

Generics are not yet a settled core feature.

Before adopting them, determine whether the intended mechanism is:

-   Compile-time monomorphization.
-   A restricted form of parametric polymorphism.
-   Another explicit compile-time specialization mechanism.

Avoid adding a broad generic system before a concrete use case and
implementation model exist.

------------------------------------------------------------------------

## 6. Functions and return values

Functions are named procedures with explicit parameters and return
types.

``` relic
FN add(a: i32, b: i32): i32
    RETURN a + b
ENDFN
```

### 6.1 Multiple return values

Multiple return values are the preferred initial direction for status
and error reporting.

Illustrative syntax:

``` relic
FN divide(a: i32, b: i32): [i32, bool]
    IF b == 0
        RETURN 0, FALSE
    ENDIF

    RETURN a / b, TRUE
ENDFN

FN main(): i32
    VAR value: i32 = 0
    VAR success: bool = FALSE

    value, success := divide(20, 4)

    IF success == FALSE
        RETURN 1
    ENDIF

    RETURN 0
ENDFN
```

The example uses `[T1, T2]` as provisional product-return notation.

The language must decide whether multiple returns are semantically:

-   A tuple/product value that the compiler may lower into several ABI
    values, or
-   A distinct multiple-result calling convention.

The first option is conceptually simpler and allows the ABI to remain an
implementation detail.

### 6.2 Exceptions

Implicit exception handling and mandatory stack unwinding are not part
of the initial direction.

Errors should be visible in function signatures and control flow. Sum
types can represent richer error categories when a boolean status is
insufficient.

An error-propagation operator is not required for the initial language.

### 6.3 Higher-order functions and closures

The revised direction excludes these from the initial language model:

-   Lambda expressions.
-   Anonymous closures.
-   Closure capture.
-   Higher-order function abstractions as a core programming style.

Whether plain function pointers are needed for callbacks, interrupt
handlers, or C interoperability is a separate systems requirement. A
function pointer does not require a closure system.

------------------------------------------------------------------------

## 7. Memory model

Memory behavior is a central part of FOSSIL's design. The language should
make storage and ownership understandable without requiring a large
runtime.

### 7.1 Storage categories

The design must distinguish:

-   Static storage.
-   Stack or automatic storage.
-   Explicitly allocated dynamic storage.
-   Memory supplied by the caller or platform.

The compiler must not silently introduce heap allocation.

### 7.2 Raw pointers

The current pointer notation is:

``` relic
VAR p: ptr[i32]
VAR raw: ptr[void]
```

Raw pointers provide low-level access. Their exact semantics must
define:

-   Nullability.
-   Dereferencing.
-   Pointer arithmetic.
-   Alignment.
-   Mutability.
-   Aliasing and provenance.
-   Volatile access.
-   Lifetime validity.

A raw pointer should not imply ownership.

### 7.3 Ownership

The previous design proposed an owning pointer form:

``` relic
VAR object: own[ptr[Point]]
```

The intended concept is that an owner is responsible for releasing its
allocation, while `get()` provides non-owning access.

Ownership transfer must be a move, not a copy.

``` relic
VAR first: own[ptr[Point]]
VAR second: own[ptr[Point]]

second := first
// Ownership moves to second.
// first is no longer an owner.
```

This is illustrative. The exact initialization, move, release, and
destruction syntax remains undecided.

The old documentation claimed that `own` guaranteed no leaks or double
frees. Those guarantees are not established until the language defines
move checking, control-flow behavior, destruction order, and interaction
with raw pointers.

In particular:

-   A pointer returned by `get()` can outlive its owner and become
    invalid.
-   Moving an owner must invalidate the source as an owner.
-   Releasing ownership must clearly define who becomes responsible for
    deallocation.
-   Destruction must behave correctly across all exits, including early
    returns.
-   Ownership of a resource does not automatically prove that every
    alias is valid.

Do not describe `own` as fully safe until these rules are formalized.

### 7.4 Allocation and deallocation

The earlier proposal included `ALLOC`, `FREE`, `malloc`, and `free`.

The final design must decide whether allocation is:

-   A built-in language operation.
-   A compiler intrinsic.
-   A platform/library operation.
-   An explicit allocator interface.

This decision is particularly important because FOSSIL must support
environments where no general-purpose heap exists.

------------------------------------------------------------------------

## 8. `no_heap` mode

`no_heap` is a proposed compilation restriction that prohibits dynamic
heap allocation.

It is not merely an optimization flag.

Its intended benefits include:

-   Predictable allocation behavior.
-   Suitability for constrained and embedded environments.
-   Reduced dependence on runtime allocators.
-   Explicit storage capacity.
-   Easier auditing of dynamic allocation.

### 8.1 What remains available

A no-heap program may still use:

-   Static storage.
-   Stack storage.
-   Fixed-capacity arrays.
-   Caller-provided memory.
-   Intrusive data structures.
-   Index-based data structures backed by fixed storage.

`no_heap` does not mean that every object must live on the stack.

### 8.2 Data structure alternatives

  General-purpose structure   No-heap implementation
  --------------------------- ------------------------------------
  Dynamic array               Fixed-capacity array
  Linked list                 Intrusive list or index-based list
  Queue                       Ring buffer
  Hash map                    Fixed-capacity hash table
  Tree                        Array-indexed tree
  Graph                       Fixed-capacity adjacency storage
  String builder              Fixed-capacity buffer

A linked list does not inherently require heap allocation. Nodes can be
embedded in existing objects or stored in a fixed array and linked by
indices.

### 8.3 Constraints and unresolved details

`no_heap` does not automatically guarantee bounded total memory.
Recursion can exhaust the stack, and large automatic objects can consume
substantial storage.

The specification must define:

-   Whether heap allocation is rejected statically.
-   How library allocation requirements are declared.
-   Whether custom allocators are permitted if backed by caller-provided
    storage.
-   How stack usage is handled or measured.
-   Whether the mode is global, per module, or both.

A stricter bounded-memory profile may be considered later. It should not
be conflated with `no_heap`.

------------------------------------------------------------------------

## 9. Concurrency

The proposed concurrency direction is actor-inspired, with isolated
state and message passing.

An actor is a conceptual unit that owns state and processes incoming
messages. This is a possible language-level abstraction, not yet a
finalized syntax or runtime.

### 9.1 Design goals

-   State should have a clear owner.
-   Communication should be explicit.
-   Message delivery and processing rules must be defined.
-   The language should not require a large runtime merely to use basic
    concurrency.

### 9.2 Questions to resolve

1.  Are actors scheduled by OS threads, cooperative tasks, or an
    application-provided scheduler?
2.  Are mailboxes bounded or unbounded?
3.  Are messages copied, moved, or shared?
4.  How are actor creation and destruction represented?
5.  What happens when an actor fails?
6.  How do actors interact with atomics, interrupts, and shared-memory
    algorithms?
7.  Can actors operate in `NO_MACHINE` and `no_heap` configurations?

For embedded and no-heap use, bounded mailboxes and caller-provided
scheduler storage are plausible options.

Actor-style messaging should not eliminate lower-level concurrency
primitives. Systems software still needs atomics, memory ordering,
interrupts, and shared-memory coordination.

No actor syntax should be committed until these execution and resource
semantics are understood.

------------------------------------------------------------------------

## 10. Platform and machine interface

### 10.1 LLVM-backed stage

The initial compiler uses `MACHINE` and `NO_MACHINE` to distinguish the
intended platform path.

The architecture selected for code generation remains a separate
compiler target setting. For example, an x86-64 target can be used for
either an OS-facing program or a freestanding program.

`MACHINE` is intended for the machine/platform-facing path. `NO_MACHINE`
is the freestanding path.

The specification must still define precisely what facilities `MACHINE`
makes available and how those facilities are linked. It must not imply
that every `MACHINE` program automatically receives a particular
operating-system API or C runtime.

### 10.2 Machine-specific capabilities

Potential machine-facing facilities include:

-   Architecture-specific intrinsics.
-   Inline assembly.
-   Register constraints.
-   Calling convention and ABI control.
-   Memory-mapped I/O.
-   Volatile memory access.
-   Interrupt-related operations.
-   Atomic instructions and memory barriers.

These capabilities may require compiler support. They should not all be
implemented as ordinary library code.

### 10.3 Freestanding path

The freestanding path should not assume:

-   An operating system.
-   A C runtime.
-   A default entry point.
-   A standard library.
-   A default allocator.
-   Implicit startup code.

The application or platform integration supplies startup, linking, and
hardware setup as required.

Built-in language types remain available without a standard library.

------------------------------------------------------------------------

## 11. Pluggable libraries: bootstrapped-stage direction

The long-term library system is intended for the bootstrapped language,
not a prerequisite for the first LLVM compiler.

### 11.1 Package categories

The future ecosystem may contain:

1.  **Core libraries:** small, broadly reusable facilities.
2.  **Platform libraries:** OS, firmware, and hardware interfaces.
3.  **Domain libraries:** graphics, audio, networking, game development,
    and similar areas.
4.  **Custom libraries:** user-defined packages for unusual devices or
    environments.

Illustrative future imports:

``` relic
USE core
USE uefi
USE uefi.graphics
```

``` relic
USE core
USE custom_board
USE custom_board.display
```

### 11.2 Separation of responsibility

  -----------------------------------------------------------------------
  Component                           Responsibility
  ----------------------------------- -----------------------------------
  Language core                       Syntax, types, functions, control
                                      flow, memory semantics

  Compiler backend                    Target code generation, ABI
                                      lowering, machine instructions

  Platform package                    OS, firmware, or hardware-facing
                                      interfaces

  Library package                     Reusable functionality

  Application                         Project-specific behavior
  -----------------------------------------------------------------------

A library should not require compiler modification merely to provide
ordinary reusable code.

Some low-level facilities will necessarily require compiler intrinsics,
ABI support, or backend-specific lowering. Those interfaces must be
explicit.

### 11.3 Compatibility and resource declarations

The future package system should be able to describe:

-   Supported target architectures.
-   Supported environments.
-   Compiler or language interface versions.
-   Dependencies.
-   Required runtime services.
-   Heap-allocation requirements.
-   Required hardware features.

This enables a `no_heap` build to reject a package or package component
that requires unrestricted heap allocation.

The package format, module visibility rules, dependency resolution,
versioning, and build integration remain open design work.

------------------------------------------------------------------------

## 12. Compiler architecture

The LLVM-backed compiler should have a clean separation between language
semantics and LLVM code generation.

Proposed pipeline:

``` text
Source
  |
  v
Lexer
  |
  v
Parser
  |
  v
AST
  |
  v
Name and type analysis
  |
  v
Semantic validation
  |
  v
Small internal IR
  |
  v
LLVM IR generation
  |
  v
LLVM optimization
  |
  v
Object file / executable
```

### 12.1 Frontend

The frontend is responsible for:

-   Lexical analysis.
-   Parsing.
-   Name resolution.
-   Type checking.
-   Mutability checks.
-   Ownership checks, if ownership is retained.
-   Control-flow validation.
-   Platform and mode validation.

### 12.2 Internal representation

A small internal IR is recommended between semantic analysis and LLVM IR
generation.

It provides a stable boundary between FOSSIL's semantics and LLVM's
implementation details.

The IR should represent the constructs FOSSIL actually supports. It
should not simply mirror every LLVM instruction.

### 12.3 LLVM backend

The backend is responsible for:

-   Target triples and data layouts.
-   Type and aggregate lowering.
-   Function and calling-convention lowering.
-   Control-flow and memory operations.
-   Machine-specific intrinsic lowering.
-   Object-file generation and linker integration.

LLVM is a code-generation backend, not the definition of FOSSIL's
language semantics.

### 12.4 Bootstrap strategy

Bootstrapping should follow a working LLVM implementation.

Suggested milestones:

1.  Specify a coherent language core.
2.  Implement parsing and semantic analysis.
3.  Generate correct LLVM IR.
4.  Validate target and ABI behavior.
5.  Build useful programs and libraries.
6.  Stabilize the language and compiler interfaces.
7.  Implement the compiler in FOSSIL.
8.  Decide whether the self-hosted compiler continues using LLVM or
    gains another backend.

Self-hosting is a milestone, not a reason to rush unfinished semantics
into the language.

------------------------------------------------------------------------

## 13. Explicit exclusions from the initial design

The following are not part of the initial core unless a concrete
requirement justifies them:

-   Classes and inheritance.
-   Implicit `this` or implicit method receivers.
-   Virtual dispatch by default.
-   Garbage collection.
-   Implicit heap allocation.
-   Lambda expressions and closures.
-   Functional composition and pipeline-oriented abstractions.
-   Lazy evaluation.
-   Implicit conversions.
-   Broad operator overloading.
-   Reflection.
-   Mandatory exceptions and implicit unwinding.
-   Mandatory standard library or runtime.
-   A generalized package manager in the first LLVM compiler.

This is a list of current exclusions, not a claim that every feature is
permanently impossible.

------------------------------------------------------------------------

## 14. Open design decisions

These questions must be answered before the language specification can
be considered stable.

### Syntax and core semantics

-   Is the language case-sensitive?
-   Are semicolons rejected or optional?
-   Is `LET` necessary alongside `VAR`?
-   Is `MATCH` required, or is a simpler selection construct enough?
-   What is the exact array and slice model?
-   Are function pointers in the initial core?

### Type system

-   Nominal or structural structure typing?
-   How are enums represented?
-   Are generics required for the first useful release?
-   What is the minimum type-checking model?
-   Which conversions are implicit, if any?

### Memory

-   Is `own` a built-in type constructor or a language-level ownership
    qualifier?
-   What are the exact move rules?
-   Are borrowed references distinct from raw pointers?
-   What are the aliasing and lifetime rules?
-   How is allocation represented?
-   What happens on early return and abnormal termination?

### Multiple returns and errors

-   Are multiple returns product values?
-   How are returned values destructured?
-   How are error variants represented?
-   Is a propagation operator needed at all?

### Concurrency

-   What is the execution and scheduling model?
-   Are actors a language construct or a library abstraction?
-   How are messages stored and transferred?
-   What concurrency primitives remain available below actors?

### Platform model

-   What precisely does `MACHINE` enable in the LLVM stage?
-   Which platform facilities are compiler-provided versus externally
    linked?
-   How are entry points and startup objects configured?
-   How are target architecture and execution environment represented
    independently?

### Future library system

-   What is a package?
-   What is a module?
-   How are package versions and dependencies resolved?
-   How are target and environment constraints declared?
-   How does the compiler discover library interfaces?
-   How are platform-specific implementations selected?

------------------------------------------------------------------------

## 15. Recommended design order

The next work should be research and specification, not immediate
implementation of every proposed feature.

1.  **Core syntax and execution model:** settle declarations, functions,
    control flow, and statement rules.
2.  **Type system:** define primitives, structures, enums, arrays, and
    type checking.
3.  **Memory semantics:** settle pointers, ownership, allocation, and
    destruction.
4.  **Function semantics:** settle parameters, return values, function
    pointers, and ABI behavior.
5.  **Platform model:** precisely define `MACHINE` and `NO_MACHINE` for
    the LLVM compiler.
6.  **`no_heap`:** define what the compiler rejects and what storage
    remains available.
7.  **Concurrency:** specify execution and message semantics before
    choosing syntax.
8.  **Internal IR and LLVM lowering:** validate the language with real
    generated code.
9.  **Bootstrapped library architecture:** design package and platform
    interfaces after the initial language model stabilizes.
10. **Self-hosting:** bootstrap only after the LLVM-backed
    implementation is reliable.

------------------------------------------------------------------------

## 16. Design summary

FOSSIL is an imperative-first, data-oriented systems language with a
deliberately small conceptual surface.

Its core is intended to consist of:

-   Primitive values and explicit types.
-   Structures and tagged enumerations.
-   Named functions and direct calls.
-   Mutable state and explicit control flow.
-   Raw pointers and a precisely specified ownership model.
-   Explicit memory and platform interaction.
-   Multiple return values for status and error reporting.

The LLVM-backed compiler initially uses `MACHINE` and `NO_MACHINE` for
the platform distinction. A more flexible, user-extensible library and
environment system belongs to the bootstrapped stage.

`no_heap` is a resource restriction. Actor-inspired concurrency is a
proposal requiring further semantic design. Neither should force an
unnecessarily large runtime into the language.

The guiding rule remains:

> Keep the language small enough that a programmer can understand the
> behavior of a program by understanding its data, procedures, control
> flow, and memory operations.
