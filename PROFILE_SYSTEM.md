# Pluggable Profile System Plan

## Overview

Extend the current MACHINE / NO_MACHINE binary choice into a pluggable profile system with built-in profiles:
- hosted (formerly MACHINE) -- standard OS-hosted applications
- freestanding (formerly NO_MACHINE) -- bare-metal, no OS
- uefi -- UEFI applications/drivers

Each profile defines: entry point requirements, available built-in modules, linking strategy, and target-specific codegen.

---

## Current State Analysis

### Current Implementation

| Component | Location | Behavior |
|-----------|----------|----------|
| AST | ast.rs:23-24 | MachineDecl { is_machine: bool } |
| Parser | parser.rs:73-74,91-94 | MACHINE / NO_MACHINE tokens -> MachineDecl |
| Semantic | semantic.rs:233,313-317,834-893 | machine_mode: Option<bool>; validates main signature for MACHINE |
| Codegen | codegen.rs:106-141,264 | Hardcoded MACHINE module loading |
| Module Loader | module_loader.rs:19-46 | Only loads MACHINE::* modules |
| Driver | driver.rs:107,168,183,253 | Different linking for MACHINE vs NO_MACHINE |

### Key Differences Today

| Aspect | MACHINE (is_machine=true) | NO_MACHINE (is_machine=false) |
|--------|----------------------------|--------------------------------|
| Entry point | main(): i32 required | No main required |
| main signature | (), (i32, ptr[ptr[u8]]) | N/A |
| Linking | Two-step (.o -> exe via clang) | Single-step (-ffreestanding -nostdlib) |
| Built-in modules | MACHINE::IO, MACHINE::win32, etc. | None |
| CRT | Linked | Not linked |

---

## Design: Pluggable Profile System

### Core Concept: Profile Trait

`
ust
// src/profile.rs (new)
pub trait Profile: Send + Sync {
    fn name(&self) -> &str;
    fn entry_point(&self) -> EntryPointSpec;
    fn linking(&self) -> LinkingStrategy;
    fn builtin_modules(&self) -> Vec<BuiltinModule>;
    fn target_triple(&self) -> Option<String>;
    fn default_cpu_features(&self) -> Vec<String>;
    fn panic_strategy(&self) -> PanicStrategy;
}

pub enum EntryPointSpec {
    Required { name: String, signatures: Vec<FunctionSignature> },
    Optional,
    Custom(String),  // e.g., efi_main for UEFI
}

pub enum LinkingStrategy {
    Hosted { two_step: bool },
    Freestanding,
    Uefi { subsystem: u16 },
}
`

### Built-in Profiles

`
ust
pub struct HostedProfile;      // formerly MACHINE
pub struct FreestandingProfile; // formerly NO_MACHINE
pub struct UefiProfile;        // new
`

### Profile Registry

`
ust
pub struct ProfileRegistry {
    profiles: HashMap<String, Box<dyn Profile>>,
    default: String,
}

impl ProfileRegistry {
    pub fn new() -> Self {
        let mut reg = Self { profiles: HashMap::new(), default: hosted.into() };
        reg.register(Box::new(HostedProfile));
        reg.register(Box::new(FreestandingProfile));
        reg.register(Box::new(UefiProfile));
        reg
    }
}
`

---

## Profile Specifications

### 1. Hosted Profile (Default)

`
PROFILE hosted
    ENTRY main(i32) or main(i32, ptr[ptr[u8]])
    LINKING hosted
    MODULES MACHINE::IO, MACHINE::win32, MACHINE::io_handles
    TARGET auto
    CPU auto
    PANIC unwind
ENDPROFILE
`

Codegen:
- Two-step linking: .ll -> .o -> exe
- Links CRT, standard library
- Default target triple from host

### 2. Freestanding Profile

`
PROFILE freestanding
    ENTRY none (user-defined)
    LINKING freestanding -ffreestanding -nostdlib
    MODULES none
    TARGET auto
    CPU auto
    PANIC abort
ENDPROFILE
`

Codegen:
- Single-step: .ll -> exe directly
- -ffreestanding -nostdlib flags
- No CRT, no standard library

### 3. UEFI Profile

`
PROFILE uefi
    ENTRY efi_main(ptr[EFI_HANDLE], ptr[EFI_SYSTEM_TABLE])
    LINKING uefi subsystem=10
    MODULES UEFI::BootServices, UEFI::RuntimeServices, UEFI::Protocols
    TARGET x86_64-unknown-uefi
    CPU x86-64
    PANIC abort
ENDPROFILE
`

Codegen:
- Links as UEFI application (subsystem 10)
- Entry point: efi_main(EFI_HANDLE, *EFI_SYSTEM_TABLE) -> EFI_STATUS
- No CRT, but UEFI boot services available
- PE/COFF output with UEFI header

---

## Implementation Plan

### Phase 1: Core Profile Infrastructure (Week 1)

#### 1.1 New Files
`
src/
profile.rs           # Profile trait, registry, built-in profiles
profile/
    mod.rs
    hosted.rs
    freestanding.rs
    uefi.rs
`

#### 1.2 AST Changes
`
ust
// ast.rs - replace MachineDecl
pub enum ProfileDecl {
    Builtin { name: String },      // PROFILE hosted
    Custom { name: String, config: ProfileConfig },  // PROFILE custom { ... }
}

pub struct ProfileConfig {
    entry: EntryPointSpec,
    linking: LinkingStrategy,
    modules: Vec<String>,
    target: Option<String>,
    cpu_features: Vec<String>,
    panic: PanicStrategy,
}
`

#### 1.3 Parser
`
ust
// parser.rs
fn parse_profile_decl(&mut self) -> Result<ProfileDecl, String> {
    // PROFILE hosted
    // PROFILE freestanding
    // PROFILE uefi
    // PROFILE custom { ... }
}
`

#### 1.4 Semantic Analyzer
`
ust
// semantic.rs - replace machine_mode with profile: Option<Box<dyn Profile>>
fn collect_declarations(&mut self, program: &Program) {
    for item in &program.items {
        match item {
            Item::ProfileDecl(p) => self.load_profile(p),
            ...
        }
    }
}
`

### Phase 2: Built-in Profile Implementation (Week 2)

#### 2.1 Hosted Profile
`
ust
impl Profile for HostedProfile {
    fn name(&self) -> &str { hosted }
    fn entry_point(&self) -> EntryPointSpec {
        EntryPointSpec::Required {
            name: main.into(),
            signatures: vec![
                FunctionSignature { params: vec![], ret: Type::I32 },
                FunctionSignature { params: vec![Type::I32, Type::Ptr(Box::new(Type::Ptr(Box::new(Type::U8))))], ret: Type::I32 },
            ],
        }
    }
    fn linking(&self) -> LinkingStrategy { LinkingStrategy::Hosted { two_step: true } }
    fn builtin_modules(&self) -> Vec<BuiltinModule> {
        vec![
            (MACHINE::IO, include_str!(machine/io.fsl)),
            (MACHINE::win32, include_str!(machine/win32.fsl)),
            (MACHINE::io_handles, include_str!(machine/io_handles.fsl)),
            (MACHINE::Conout, include_str!(machine/conout.fsl)),
            (MACHINE::Conin, include_str!(machine/conin.fsl)),
            (MACHINE::Conerr, include_str!(machine/conerr.fsl)),
        ]
    }
}
`

#### 2.2 Freestanding Profile
`
ust
impl Profile for FreestandingProfile {
    fn name(&self) -> &str { freestanding }
    fn entry_point(&self) -> EntryPointSpec { EntryPointSpec::Optional }
    fn linking(&self) -> LinkingStrategy { LinkingStrategy::Freestanding }
    fn builtin_modules(&self) -> Vec<BuiltinModule> { vec![] }
    fn panic_strategy(&self) -> PanicStrategy { PanicStrategy::Abort }
}
`

#### 2.3 UEFI Profile
`
ust
impl Profile for UefiProfile {
    fn name(&self) -> &str { uefi }
    fn entry_point(&self) -> EntryPointSpec {
        EntryPointSpec::Required {
            name: efi_main.into(),
            signatures: vec![FunctionSignature {
                params: vec![
                    Type::Ptr(Box::new(Type::Named(EFI_HANDLE.into()))),
                    Type::Ptr(Box::new(Type::Named(EFI_SYSTEM_TABLE.into()))),
                ],
                ret: Type::Named(EFI_STATUS.into()),
            }],
        }
    }
    fn linking(&self) -> LinkingStrategy { 
        LinkingStrategy::Uefi { subsystem: 10 } 
    }
    fn builtin_modules(&self) -> Vec<BuiltinModule> {
        vec![
            (UEFI::BootServices, include_str!(uefi/boot_services.fsl)),
            (UEFI::RuntimeServices, include_str!(uefi/runtime_services.fsl)),
            (UEFI::Protocols, include_str!(uefi/protocols.fsl)),
            (UEFI::Types, include_str!(uefi/types.fsl)),
        ]
    }
    fn target_triple(&self) -> Option<String> {
        Some(x86_64-unknown-uefi.into())
    }
}
`

### Phase 3: Integration & Migration (Week 3)

#### 3.1 Update Driver
`
ust
// driver.rs - use profile's linking strategy
fn compile_ir_to_binary(ir: &str, output: &Path, profile: &dyn Profile) -> Result<(), String> {
    match profile.linking() {
        LinkingStrategy::Hosted { two_step } => { ... }
        LinkingStrategy::Freestanding => { ... }
        LinkingStrategy::Uefi { subsystem } => { ... }
    }
}
`

#### 3.2 Backward Compatibility
`sl
; Old syntax still works
MACHINE        ; -> PROFILE hosted
NO_MACHINE     ; -> PROFILE freestanding

; New syntax preferred
PROFILE hosted
PROFILE freestanding
PROFILE uefi
`

Parser: Treat MACHINE/NO_MACHINE as syntactic sugar for PROFILE hosted/PROFILE freestanding.

#### 3.3 Module Loader
`
ust
// module_loader.rs - use profile's module list
fn load_builtin_modules(&mut self, profile: &dyn Profile) {
    for (name, source) in profile.builtin_modules() {
        self.load_module(name, source);
    }
}
`

### Phase 4: UEFI Module Implementation (Week 4)

#### 4.1 Create UEFI Module Files
`
src/machine/
uefi/
    types.fsl          # EFI_HANDLE, EFI_STATUS, EFI_SYSTEM_TABLE, GUID
    boot_services.fsl  # AllocatePages, CreateEvent, LocateProtocol, etc.
    runtime_services.fsl # GetTime, SetVariable, ResetSystem
    protocols.fsl      # SimpleTextOutput, SimpleFileSystem, LoadedImage
    memory.fsl         # Memory map, memory descriptors
`

#### 4.2 Example UEFI Types
`
; uefi/types.fsl
TYPEDEF u64 EFI_STATUS
TYPEDEF ptr[u8] EFI_HANDLE
TYPEDEF ptr[u8] EFI_EVENT

CONST EFI_SUCCESS: EFI_STATUS = 0
CONST EFI_LOAD_ERROR: EFI_STATUS = 1
CONST EFI_INVALID_PARAMETER: EFI_STATUS = 2
; ...

STRUCT EFI_GUID {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

STRUCT EFI_SYSTEM_TABLE {
    hdr: EFI_TABLE_HEADER,
    fw_vendor: ptr[u16],
    fw_revision: u32,
    con_in_handle: EFI_HANDLE,
    con_in: ptr[EFI_SIMPLE_TEXT_INPUT_PROTOCOL],
    con_out_handle: EFI_HANDLE,
    con_out: ptr[EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL],
    stderr_handle: EFI_HANDLE,
    stderr: ptr[EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL],
    runtime: ptr[EFI_RUNTIME_SERVICES],
    boot_services: ptr[EFI_BOOT_SERVICES],
    num_table_entries: usize,
    configuration_table: ptr[EFI_CONFIGURATION_TABLE],
}
`

#### 4.3 Example UEFI Hello World
`
PROFILE uefi

USE UEFI::BootServices
USE UEFI::Protocols

FN efi_main(handle: ptr[EFI_HANDLE], systab: ptr[EFI_SYSTEM_TABLE]): EFI_STATUS
    VAR status = systab.con_out.OutputString(systab.con_out, LHello UEFI!
)
    RETURN status
ENDFN
`

---

## CLI Interface

`
carnax build --profile hosted app.fsl
carnax build --profile freestanding kernel.fsl
carnax build --profile uefi bootloader.fsl

# Or from source:
# PROFILE uefi  (in source file)
carnax build bootloader.fsl
`

### Default Behavior
| Source has PROFILE | --profile flag | Result |
|---------------------|------------------|--------|
| No | No | hosted (default) |
| Yes | No | Source profile |
| No | Yes | Flag profile |
| Yes | Yes | Error (conflict) |

---

## Testing Strategy

### Test Cases Per Profile

| Test | Hosted | Freestanding | UEFI |
|------|--------|--------------|------|
| Minimal main | | N/A | (efi_main) |
| No main | | | |
| IO module | | | UEFI::Protocols |
| CRT linked | | | |
| -ffreestanding | | | |
| Subsystem | console | native | efi_app (10) |

### Integration Tests
`
ust
// tests/profile_tests.rs
#[test] fn hosted_requires_main() { ... }
#[test] fn freestanding_no_main_ok() { ... }
#[test] fn uefi_requires_efi_main() { ... }
#[test] fn hosted_links_crt() { ... }
#[test] fn freestanding_no_crt() { ... }
#[test] fn uefi_subsystem_10() { ... }
`

---

## Migration Path

### For Existing Users
`
# Old code continues to work
MACHINE          # -> treated as PROFILE hosted
NO_MACHINE       # -> treated as PROFILE freestanding

# New projects use explicit profiles
PROFILE hosted
PROFILE freestanding
PROFILE uefi
`

### For CI/CD
`yaml
# Build for different targets
- carnax build --profile hosted app.fsl -o app.exe
- carnax build --profile freestanding kernel.fsl -o kernel.bin
- carnax build --profile uefi bootloader.fsl -o bootloader.efi
`

---

## Open Questions

1. Custom profiles: Allow users to define .profile.fsl files?
2. Profile inheritance: PROFILE my_embedded EXTENDS freestanding { ... }?
3. Profile-specific stdlib: Separate std per profile?
4. Cross-compilation: --target flag vs profile target_triple?
5. UEFI secure boot: Support signed binaries?

---

## File Changes Summary

| File | Change |
|------|--------|
| src/profile.rs | New: Profile trait + registry |
| src/profile/*.rs | New: Built-in profiles |
| src/ast.rs | Replace MachineDecl with ProfileDecl |
| src/parser.rs | Parse PROFILE declarations |
| src/semantic.rs | Use profile: Box<dyn Profile> instead of machine_mode |
| src/codegen.rs | Use profile's linking strategy |
| src/driver.rs | Use profile for linking |
| src/module_loader.rs | Load profile's builtin modules |
| src/token.rs | Add Profile, EndProfile, Custom keywords |
| src/machine/uefi/*.fsl | New: UEFI module files |

---

## Timeline

| Week | Deliverable |
|------|-------------|
| 1 | Core profile trait, AST, parser, semantic integration |
| 2 | Hosted, Freestanding, UEFI profile implementations |
| 3 | Driver/module loader integration, backward compat |
| 4 | UEFI module files, testing, documentation |

---

## Success Criteria

- [ ] PROFILE hosted works identically to old MACHINE
- [ ] PROFILE freestanding works identically to old NO_MACHINE
- [ ] PROFILE uefi produces valid .efi that boots in QEMU/OVMF
- [ ] Old MACHINE/NO_MACHINE syntax still works (deprecated warning)
- [ ] All existing tests pass
- [ ] New tests for each profile pass
