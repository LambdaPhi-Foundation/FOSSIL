# Phase 5: CORE.atomic (Optional)

## Goal
Provide atomic types and operations for lock-free programming. Optional — only needed if threading is used.

---

## 5.1 `src/core/atomic.fsl`

```fsl
MODULE atomic
    USE CORE.err

    ; --- Atomic Types (zero-sized wrappers) ---

    STRUCT AtomicBool
        value: u8
    ENDSTRUCT

    STRUCT AtomicI8
        value: i8
    ENDSTRUCT

    STRUCT AtomicU8
        value: u8
    ENDSTRUCT

    STRUCT AtomicI16
        value: i16
    ENDSTRUCT

    STRUCT AtomicU16
        value: u16
    ENDSTRUCT

    STRUCT AtomicI32
        value: i32
    ENDSTRUCT

    STRUCT AtomicU32
        value: u32
    ENDSTRUCT

    STRUCT AtomicI64
        value: i64
    ENDSTRUCT

    STRUCT AtomicU64
        value: u64
    ENDSTRUCT

    STRUCT AtomicPtr[T]
        value: ptr[T]
    ENDSTRUCT

    STRUCT AtomicIsize
        value: isize
    ENDSTRUCT

    STRUCT AtomicUsize
        value: usize
    ENDSTRUCT

    ; --- Memory Ordering ---

    ENUM Ordering
        Relaxed
        Acquire
        Release
        AcqRel
        SeqCst
    ENDENUM

    ; --- Operations (map to LLVM atomicrmw/cmpxchg) ---

    ; Load
    EXTERN FN atomic_load_bool(a: ptr[AtomicBool], ord: Ordering): bool ENDFN
    EXTERN FN atomic_load_i8(a: ptr[AtomicI8], ord: Ordering): i8 ENDFN
    EXTERN FN atomic_load_u8(a: ptr[AtomicU8], ord: Ordering): u8 ENDFN
    EXTERN FN atomic_load_i16(a: ptr[AtomicI16], ord: Ordering): i16 ENDFN
    EXTERN FN atomic_load_u16(a: ptr[AtomicU16], ord: Ordering): u16 ENDFN
    EXTERN FN atomic_load_i32(a: ptr[AtomicI32], ord: Ordering): i32 ENDFN
    EXTERN FN atomic_load_u32(a: ptr[AtomicU32], ord: Ordering): u32 ENDFN
    EXTERN FN atomic_load_i64(a: ptr[AtomicI64], ord: Ordering): i64 ENDFN
    EXTERN FN atomic_load_u64(a: ptr[AtomicU64], ord: Ordering): u64 ENDFN
    EXTERN FN atomic_load_isize(a: ptr[AtomicIsize], ord: Ordering): isize ENDFN
    EXTERN FN atomic_load_usize(a: ptr[AtomicUsize], ord: Ordering): usize ENDFN
    EXTERN FN atomic_load_ptr[T](a: ptr[AtomicPtr[T]], ord: Ordering): ptr[T] ENDFN

    ; Store
    EXTERN FN atomic_store_bool(a: ptr[AtomicBool], val: bool, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_i8(a: ptr[AtomicI8], val: i8, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_u8(a: ptr[AtomicU8], val: u8, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_i16(a: ptr[AtomicI16], val: i16, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_u16(a: ptr[AtomicU16], val: u16, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_i32(a: ptr[AtomicI32], val: i32, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_u32(a: ptr[AtomicU32], val: u32, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_i64(a: ptr[AtomicI64], val: i64, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_u64(a: ptr[AtomicU64], val: u64, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_isize(a: ptr[AtomicIsize], val: isize, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_usize(a: ptr[AtomicUsize], val: usize, ord: Ordering): void ENDFN
    EXTERN FN atomic_store_ptr[T](a: ptr[AtomicPtr[T]], val: ptr[T], ord: Ordering): void ENDFN

    ; Swap (atomic exchange)
    EXTERN FN atomic_swap_bool(a: ptr[AtomicBool], val: bool, ord: Ordering): bool ENDFN
    EXTERN FN atomic_swap_i8(a: ptr[AtomicI8], val: i8, ord: Ordering): i8 ENDFN
    EXTERN FN atomic_swap_u8(a: ptr[AtomicU8], val: u8, ord: Ordering): u8 ENDFN
    EXTERN FN atomic_swap_i16(a: ptr[AtomicI16], val: i16, ord: Ordering): i16 ENDFN
    EXTERN FN atomic_swap_u16(a: ptr[AtomicU16], val: u16, ord: Ordering): u16 ENDFN
    EXTERN FN atomic_swap_i32(a: ptr[AtomicI32], val: i32, ord: Ordering): i32 ENDFN
    EXTERN FN atomic_swap_u32(a: ptr[AtomicU32], val: u32, ord: Ordering): u32 ENDFN
    EXTERN FN atomic_swap_i64(a: ptr[AtomicI64], val: i64, ord: Ordering): i64 ENDFN
    EXTERN FN atomic_swap_u64(a: ptr[AtomicU64], val: u64, ord: Ordering): u64 ENDFN
    EXTERN FN atomic_swap_isize(a: ptr[AtomicIsize], val: isize, ord: Ordering): isize ENDFN
    EXTERN FN atomic_swap_usize(a: ptr[AtomicUsize], val: usize, ord: Ordering): usize ENDFN
    EXTERN FN atomic_swap_ptr[T](a: ptr[AtomicPtr[T]], val: ptr[T], ord: Ordering): ptr[T] ENDFN

    ; Compare-and-swap (CAS)
    EXTERN FN atomic_cas_bool(a: ptr[AtomicBool], expected: ptr[bool], desired: bool, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_i8(a: ptr[AtomicI8], expected: ptr[i8], desired: i8, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_u8(a: ptr[AtomicU8], expected: ptr[u8], desired: u8, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_i16(a: ptr[AtomicI16], expected: ptr[i16], desired: i16, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_u16(a: ptr[AtomicU16], expected: ptr[u16], desired: u16, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_i32(a: ptr[AtomicI32], expected: ptr[i32], desired: i32, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_u32(a: ptr[AtomicU32], expected: ptr[u32], desired: u32, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_i64(a: ptr[AtomicI64], expected: ptr[i64], desired: i64, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_u64(a: ptr[AtomicU64], expected: ptr[u64], desired: u64, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_isize(a: ptr[AtomicIsize], expected: ptr[isize], desired: isize, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_usize(a: ptr[AtomicUsize], expected: ptr[usize], desired: usize, ord: Ordering): bool ENDFN
    EXTERN FN atomic_cas_ptr[T](a: ptr[AtomicPtr[T]], expected: ptr[ptr[T]], desired: ptr[T], ord: Ordering): bool ENDFN

    ; Fetch-and-add (atomicrmw)
    EXTERN FN atomic_fetch_add_i8(a: ptr[AtomicI8], val: i8, ord: Ordering): i8 ENDFN
    EXTERN FN atomic_fetch_add_u8(a: ptr[AtomicU8], val: u8, ord: Ordering): u8 ENDFN
    EXTERN FN atomic_fetch_add_i16(a: ptr[AtomicI16], val: i16, ord: Ordering): i16 ENDFN
    EXTERN FN atomic_fetch_add_u16(a: ptr[AtomicU16], val: u16, ord: Ordering): u16 ENDFN
    EXTERN FN atomic_fetch_add_i32(a: ptr[AtomicI32], val: i32, ord: Ordering): i32 ENDFN
    EXTERN FN atomic_fetch_add_u32(a: ptr[AtomicU32], val: u32, ord: Ordering): u32 ENDFN
    EXTERN FN atomic_fetch_add_i64(a: ptr[AtomicI64], val: i64, ord: Ordering): i64 ENDFN
    EXTERN FN atomic_fetch_add_u64(a: ptr[AtomicU64], val: u64, ord: Ordering): u64 ENDFN
    EXTERN FN atomic_fetch_add_isize(a: ptr[AtomicIsize], val: isize, ord: Ordering): isize ENDFN
    EXTERN FN atomic_fetch_add_usize(a: ptr[AtomicUsize], val: usize, ord: Ordering): usize ENDFN

    ; Fetch-and-sub
    EXTERN FN atomic_fetch_sub_i8(a: ptr[AtomicI8], val: i8, ord: Ordering): i8 ENDFN
    EXTERN FN atomic_fetch_sub_u8(a: ptr[AtomicU8], val: u8, ord: Ordering): u8 ENDFN
    EXTERN FN atomic_fetch_sub_i16(a: ptr[AtomicI16], val: i16, ord: Ordering): i16 ENDFN
    EXTERN FN atomic_fetch_sub_u16(a: ptr[AtomicU16], val: u16, ord: Ordering): u16 ENDFN
    EXTERN FN atomic_fetch_sub_i32(a: ptr[AtomicI32], val: i32, ord: Ordering): i32 ENDFN
    EXTERN FN atomic_fetch_sub_u32(a: ptr[AtomicU32], val: u32, ord: Ordering): u32 ENDFN
    EXTERN FN atomic_fetch_sub_i64(a: ptr[AtomicI64], val: i64, ord: Ordering): i64 ENDFN
    EXTERN FN atomic_fetch_sub_u64(a: ptr[AtomicU64], val: u64, ord: Ordering): u64 ENDFN
    EXTERN FN atomic_fetch_sub_isize(a: ptr[AtomicIsize], val: isize, ord: Ordering): isize ENDFN
    EXTERN FN atomic_fetch_sub_usize(a: ptr[AtomicUsize], val: usize, ord: Ordering): usize ENDFN

    ; Fetch-and-and/or/xor
    EXTERN FN atomic_fetch_and_u8(a: ptr[AtomicU8], val: u8, ord: Ordering): u8 ENDFN
    EXTERN FN atomic_fetch_and_u16(a: ptr[AtomicU16], val: u16, ord: Ordering): u16 ENDFN
    EXTERN FN atomic_fetch_and_u32(a: ptr[AtomicU32], val: u32, ord: Ordering): u32 ENDFN
    EXTERN FN atomic_fetch_and_u64(a: ptr[AtomicU64], val: u64, ord: Ordering): u64 ENDFN
    EXTERN FN atomic_fetch_or_u8(a: ptr[AtomicU8], val: u8, ord: Ordering): u8 ENDFN
    EXTERN FN atomic_fetch_or_u16(a: ptr[AtomicU16], val: u16, ord: Ordering): u16 ENDFN
    EXTERN FN atomic_fetch_or_u32(a: ptr[AtomicU32], val: u32, ord: Ordering): u32 ENDFN
    EXTERN FN atomic_fetch_or_u64(a: ptr[AtomicU64], val: u64, ord: Ordering): u64 ENDFN
    EXTERN FN atomic_fetch_xor_u8(a: ptr[AtomicU8], val: u8, ord: Ordering): u8 ENDFN
    EXTERN FN atomic_fetch_xor_u16(a: ptr[AtomicU16], val: u16, ord: Ordering): u16 ENDFN
    EXTERN FN atomic_fetch_xor_u32(a: ptr[AtomicU32], val: u32, ord: Ordering): u32 ENDFN
    EXTERN FN atomic_fetch_xor_u64(a: ptr[AtomicU64], val: u64, ord: Ordering): u64 ENDFN

    ; --- Fence ---
    EXTERN FN atomic_fence(ord: Ordering): void ENDFN

ENDMODULE
```

---

## 5.2 Codegen Lowering

| Operation | LLVM Instruction |
|-----------|------------------|
| `atomic_load_*` | `load atomic` |
| `atomic_store_*` | `store atomic` |
| `atomic_swap_*` | `atomicrmw xchg` |
| `atomic_cas_*` | `cmpxchg` |
| `atomic_fetch_add_*` | `atomicrmw add` |
| `atomic_fetch_sub_*` | `atomicrmw sub` |
| `atomic_fetch_and_*` | `atomicrmw and` |
| `atomic_fetch_or_*` | `atomicrmw or` |
| `atomic_fetch_xor_*` | `atomicrmw xor` |
| `atomic_fence` | `fence` |

### Example
```llvm
; atomic_fetch_add_i32(a, 1, SeqCst)
%old = atomicrmw add i32* %a, i32 1 seqcst
```

---

## 5.3 Usage Example

```fsl
NO_MACHINE
USE CORE.atomic

VAR counter: AtomicUsize = AtomicUsize { value: 0 }

FN increment(): usize
    RETURN atomic_fetch_add_usize(&counter, 1, Ordering.SeqCst)
ENDFN

FN main(): i32
    VAR a = increment()
    VAR b = increment()
    RETURN (a + b) as i32
ENDFN
```

---

## Acceptance Criteria
- [ ] All atomic types defined
- [ ] Load/store/CAS/fetch-add operations lower to LLVM atomics
- [ ] Memory ordering respected
- [ ] Works in `NO_MACHINE` mode
- [ ] Thread-safe when used with MACHINE threading