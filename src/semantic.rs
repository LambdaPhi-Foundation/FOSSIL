use crate::ast::*;
use crate::module_loader::ModuleLoader;
use std::collections::HashMap;

/// Result of semantic analysis.
pub struct SemanticResult {
    pub errors: Vec<String>,
    pub has_errors: bool,
}

impl SemanticResult {
    pub fn with_error(mut self, msg: String) -> Self {
        self.errors.push(msg);
        self.has_errors = true;
        self
    }
}

/// Resolved type used during analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Void,
    Ptr(Box<Ty>),
    Own(Box<Ty>),
    Named(String),
    Array(Box<Ty>, usize),
    Tuple(Vec<Ty>),       // (T1, T2, ...) - product type
    Union(Vec<Ty>),       // [T1, T2, ...] - sum type
    Generic(String),      // Generic type parameter (unresolved)
    NullPtr,              // null pointer literal
    Error,
}

impl Ty {
    pub fn is_owner(&self) -> bool {
        matches!(self, Ty::Own(_))
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self, Ty::Ptr(_) | Ty::Own(_))
    }

    pub fn inner(&self) -> Option<&Ty> {
        match self {
            Ty::Ptr(inner) | Ty::Own(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn inner_cloned(&self) -> Option<Ty> {
        self.inner().cloned()
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64 | Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64)
    }

    pub fn types_eq(&self, other: &Ty) -> bool {
        match (self, other) {
            (Ty::Bool, Ty::Bool)
            | (Ty::I8, Ty::I8)
            | (Ty::I16, Ty::I16)
            | (Ty::I32, Ty::I32)
            | (Ty::I64, Ty::I64)
            | (Ty::U8, Ty::U8)
            | (Ty::U16, Ty::U16)
            | (Ty::U32, Ty::U32)
            | (Ty::U64, Ty::U64)
            | (Ty::F32, Ty::F32)
            | (Ty::F64, Ty::F64)
            | (Ty::Void, Ty::Void)
            | (Ty::NullPtr, Ty::NullPtr)
            | (Ty::NullPtr, Ty::Ptr(_))
            | (Ty::Ptr(_), Ty::NullPtr)
            | (Ty::Error, _)
            | (_, Ty::Error) => true,
            (Ty::Ptr(a), Ty::Ptr(b)) => {
                if a.types_eq(b) {
                    return true;
                }
                if let Ty::Array(elem, _) = &**a
                    && elem.types_eq(b)
                {
                    return true;
                }
                if let Ty::Array(elem, _) = &**b
                    && elem.types_eq(a)
                {
                    return true;
                }
                false
            }
            (Ty::Own(a), Ty::Own(b)) => a.types_eq(b),
            (Ty::Named(a), Ty::Named(b)) if (a == "string" && b == "str") || (a == "str" && b == "string") => {
                true
            }
            (Ty::Named(a), Ty::Named(b)) => a == b,
            (Ty::Named(s), Ty::Ptr(inner)) | (Ty::Ptr(inner), Ty::Named(s))
                if (s == "string" || s == "str") && (inner.types_eq(&Ty::U8) || inner.types_eq(&Ty::I8)) =>
            {
                true
            }
            (Ty::Array(a, n), Ty::Array(b, m)) => n == m && a.types_eq(b),
            (Ty::Tuple(a), Ty::Tuple(b)) => a.len() == b.len()
                && a.iter().zip(b.iter()).all(|(x, y)| x.types_eq(y)),
            (Ty::Union(a), Ty::Union(b)) => a.len() == b.len()
                && a.iter().zip(b.iter()).all(|(x, y)| x.types_eq(y)),
            (Ty::Generic(a), Ty::Generic(b)) => a == b,
            _ => false,
        }
    }

    pub fn types_compatible_for_const(&self, other: &Ty) -> bool {
        if self.types_eq(other) {
            return true;
        }
        if self.is_integer() && other.is_integer() {
            return true;
        }
        false
    }

    pub fn display(&self) -> String {
        match self {
            Ty::Bool => "bool".into(),
            Ty::I8 => "i8".into(),
            Ty::I16 => "i16".into(),
            Ty::I32 => "i32".into(),
            Ty::I64 => "i64".into(),
            Ty::U8 => "u8".into(),
            Ty::U16 => "u16".into(),
            Ty::U32 => "u32".into(),
            Ty::U64 => "u64".into(),
            Ty::F32 => "f32".into(),
            Ty::F64 => "f64".into(),
            Ty::Void => "void".into(),
            Ty::Ptr(inner) => format!("ptr[{}]", inner.display()),
            Ty::Own(inner) => format!("own[ptr[{}]]", inner.display()),
            Ty::Named(n) => n.clone(),
            Ty::Array(inner, n) => format!("[{}; {}]", inner.display(), n),
            Ty::Tuple(ts) => {
                let parts: Vec<String> = ts.iter().map(|t| t.display()).collect();
                format!("({})", parts.join(", "))
            }
            Ty::Union(ts) => {
                let parts: Vec<String> = ts.iter().map(|t| t.display()).collect();
                format!("[{}]", parts.join(", "))
            }
            Ty::Generic(name) => name.clone(),
            Ty::NullPtr => "nullptr".into(),
            Ty::Error => "error".into(),
        }
    }
}

/// Information about a variable in scope.
#[derive(Debug, Clone)]
struct VarState {
    _name: String,
    ty: Ty,
    is_const: bool,
    is_owner: bool,
    is_initialized: bool,
    is_moved: bool,
}

/// A scope level: variables declared in this block.
#[derive(Debug, Clone)]
struct Scope {
    vars: HashMap<String, VarState>,
}

impl Scope {
    fn new() -> Self {
        Self { vars: HashMap::new() }
    }

    fn insert(&mut self, name: String, state: VarState) -> Option<VarState> {
        self.vars.insert(name, state)
    }

    fn get(&self, name: &str) -> Option<&VarState> {
        self.vars.get(name)
    }

    fn get_mut(&mut self, name: &str) -> Option<&mut VarState> {
        self.vars.get_mut(name)
    }
}

/// Top-level function signature collected during declaration pass.
#[derive(Debug, Clone)]
struct FnSignature {
    _name: String,
    generic_params: Vec<String>,
    params: Vec<(String, Ty, bool)>, // name, type, is_owner_param
    return_ty: Ty,
}

/// Top-level struct definition.
#[derive(Debug, Clone)]
struct StructDef {
    _name: String,
    fields: Vec<(String, Ty)>,
}

/// Top-level enum definition.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<(String, Option<Ty>)>,
}

/// Semantic analyzer: name resolution, type checking, ownership checking.
pub struct SemanticAnalyzer {
    errors: Vec<String>,
    scopes: Vec<Scope>,
    functions: HashMap<String, FnSignature>,
    structs: HashMap<String, StructDef>,
    enums: HashMap<String, EnumDef>,
    current_fn_return: Ty,
    machine_mode: Option<bool>, // None = unset, Some(true) = MACHINE, Some(false) = NO_MACHINE
    module_loader: ModuleLoader,
    imported_modules: HashMap<String, String>, // short_name -> full_path (e.g., "IO" -> "MACHINE.IO")
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            scopes: vec![Scope::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            current_fn_return: Ty::Void,
            machine_mode: None,
            module_loader: ModuleLoader::new(),
            imported_modules: HashMap::new(),
        }
    }

    pub fn analyze(mut self, program: &Program) -> SemanticResult {
        self.collect_declarations(program);
        
        // Analyze main program items
        for item in &program.items {
            self.analyze_item(item);
        }
        
        self.validate_machine_mode(program);
        let has_errors = !self.errors.is_empty();
        SemanticResult {
            errors: self.errors,
            has_errors,
        }
    }

    // ---------- declaration collection ----------

    fn collect_declarations(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Function(f) => {
                    let mut params = Vec::with_capacity(f.params.len());
                    for p in &f.params {
                        params.push((p.name.clone(), self.ast_type_to_ty(&p.ty), p.is_own));
                    }
                    let return_ty = self.ast_type_to_ty(&f.return_type);
                    let generic_params: Vec<String> = f.generic_params.iter().map(|p| p.name.clone()).collect();
                    self.functions.insert(
                        f.name.clone(),
                        FnSignature {
                            _name: f.name.clone(),
                            generic_params,
                            params,
                            return_ty,
                        },
                    );
                }
                Item::StructDecl(s) => {
                    let fields: Vec<(String, Ty)> = s
                        .fields
                        .iter()
                        .map(|f| (f.name.clone(), self.ast_type_to_ty(&f.ty)))
                        .collect();
                    self.structs.insert(
                        s.name.clone(),
                        StructDef { _name: s.name.clone(), fields },
                    );
                }
                Item::EnumDecl(e) => {
                    let variants: Vec<(String, Option<Ty>)> = e
                        .variants
                        .iter()
                        .map(|v| (v.name.clone(), v.payload.as_ref().map(|t| self.ast_type_to_ty(t))))
                        .collect();
                    self.enums.insert(
                        e.name.clone(),
                        EnumDef { name: e.name.clone(), variants },
                    );
                }
                Item::MachineDecl(m) => {
                    if self.machine_mode.is_some() {
                        self.errors.push("duplicate MACHINE / NO_MACHINE declaration".into());
                    }
                    self.machine_mode = Some(m.is_machine);
                }
                Item::ExternDecl(e) => {
                    let mut params = Vec::with_capacity(e.params.len());
                    for p in &e.params {
                        params.push((p.name.clone(), self.ast_type_to_ty(&p.ty), p.is_own));
                    }
                    let return_ty = self.ast_type_to_ty(&e.return_type);
                    self.functions.insert(
                        e.name.clone(),
                        FnSignature {
                            _name: e.name.clone(),
                            generic_params: Vec::new(),
                            params,
                            return_ty,
                        },
                    );
                }
                Item::ImportDecl(imp) => {
                    // Handle built-in MACHINE::* imports by loading module files
                    if imp.path.len() >= 2 && imp.path[0] == "MACHINE" {
                        let full_path = imp.path.join(".");
                        let short_name = imp.path.last().unwrap().clone();
                        // Register namespace alias: short_name -> full_path
                        self.imported_modules.insert(short_name.clone(), full_path.clone());
                        
                        match self.module_loader.load_machine_module(&imp.path) {
                            Ok(module) => {
                                // Collect declarations from the loaded module
                                self.collect_module_declarations(&module, &imp.path);
                            }
                            Err(e) => {
                                self.errors.push(format!("Failed to load module {}: {}", imp.path.join("::"), e));
                            }
                        }
                    }
                }
                Item::ModuleDecl(m) => {
                    // Recursively collect declarations from module items
                    for item in &m.items {
                        self.collect_declarations(&Program { items: vec![item.clone()] });
                    }
                }
                Item::ConstDecl(c) => {
                    // Module-level constant - store as a global constant
                    // For now, treat as a variable that can't be modified
                    let ty = self.ast_type_to_ty(&c.ty);
                    let init_ty = self.type_of_expr(&c.init);
                    if !ty.types_compatible_for_const(&init_ty) && init_ty != Ty::Error {
                        self.errors.push(format!(
                            "type mismatch: constant '{}' declared as {} but initialized with {}",
                            c.name,
                            ty.display(),
                            init_ty.display()
                        ));
                    }
                    // Store in current scope as const
                    let state = VarState {
                        _name: c.name.clone(),
                        ty,
                        is_const: true,
                        is_owner: false,
                        is_initialized: true,
                        is_moved: false,
                    };
                    self.current_scope_mut().insert(c.name.clone(), state);
                }
                Item::VarDecl(v) => {
                    // Module-level variable - store as a mutable global
                    let ty = self.ast_type_to_ty(&v.ty);
                    let is_owner = ty.is_owner();
                    let init_ty = if let Some(ref init) = v.init {
                        self.type_of_expr(init)
                    } else {
                        Ty::NullPtr
                    };
                    if let Some(ref init) = v.init
                        && !ty.types_eq(&init_ty)
                        && init_ty != Ty::Error
                    {
                        self.errors.push(format!(
                            "type mismatch: variable '{}' declared as {} but initialized with {}",
                            v.name,
                            ty.display(),
                            init_ty.display()
                        ));
                    }
                    // Store in current scope as mutable
                    let state = VarState {
                        _name: v.name.clone(),
                        ty,
                        is_const: false,
                        is_owner,
                        is_initialized: v.init.is_some(),
                        is_moved: false,
                    };
                    self.current_scope_mut().insert(v.name.clone(), state);
                }
                Item::AsmBlock(_) => {
                    // ASM blocks don't need declaration collection
                }
            }
        }
    }

    fn collect_module_declarations(&mut self, module: &ModuleDecl, path: &[String]) {
        let prefix = path.join(".");
        for item in &module.items {
            match item {
                Item::Function(f) => {
                    let qualified_name = format!("{}.{}", prefix, f.name);
                    let mut params = Vec::with_capacity(f.params.len());
                    for p in &f.params {
                        params.push((p.name.clone(), self.ast_type_to_ty(&p.ty), p.is_own));
                    }
                    let return_ty = self.ast_type_to_ty(&f.return_type);
                    let generic_params: Vec<String> = f.generic_params.iter().map(|p| p.name.clone()).collect();
                    self.functions.insert(
                        qualified_name,
                        FnSignature {
                            _name: f.name.clone(),
                            generic_params,
                            params,
                            return_ty,
                        },
                    );
                }
                Item::StructDecl(s) => {
                    let qualified_name = format!("{}.{}", prefix, s.name);
                    let fields: Vec<(String, Ty)> = s
                        .fields
                        .iter()
                        .map(|f| (f.name.clone(), self.ast_type_to_ty(&f.ty)))
                        .collect();
                    self.structs.insert(
                        qualified_name,
                        StructDef { _name: s.name.clone(), fields },
                    );
                }
                Item::EnumDecl(e) => {
                    let qualified_name = format!("{}.{}", prefix, e.name);
                    let variants: Vec<(String, Option<Ty>)> = e
                        .variants
                        .iter()
                        .map(|v| (v.name.clone(), v.payload.as_ref().map(|t| self.ast_type_to_ty(t))))
                        .collect();
                    self.enums.insert(
                        qualified_name,
                        EnumDef { name: e.name.clone(), variants },
                    );
                }
                Item::ModuleDecl(m) => {
                    let sub_path = [path.to_vec(), vec![m.name.clone()]].concat();
                    self.collect_module_declarations(m, &sub_path);
                }
                Item::ExternDecl(e) => {
                    let qualified_name = format!("{}.{}", prefix, e.name);
                    let mut params = Vec::with_capacity(e.params.len());
                    for p in &e.params {
                        params.push((p.name.clone(), self.ast_type_to_ty(&p.ty), p.is_own));
                    }
                    let return_ty = self.ast_type_to_ty(&e.return_type);
                    self.functions.insert(
                        qualified_name,
                        FnSignature {
                            _name: e.name.clone(),
                            generic_params: Vec::new(),
                            params,
                            return_ty,
                        },
                    );
                }
                Item::ImportDecl(imp) => {
                    // Handle nested imports
                    if imp.path.len() >= 2 && imp.path[0] == "MACHINE" {
                        if let Ok(sub_module) = self.module_loader.load_machine_module(&imp.path) {
                            self.collect_module_declarations(&sub_module, &imp.path);
                        }
                    }
                }
                Item::ConstDecl(c) => {
                    let qualified_name = format!("{}.{}", prefix, c.name);
                    let ty = self.ast_type_to_ty(&c.ty);
                    let init_ty = self.type_of_expr(&c.init);
                    if !ty.types_compatible_for_const(&init_ty) && init_ty != Ty::Error {
                        self.errors.push(format!(
                            "type mismatch: constant '{}' declared as {} but initialized with {}",
                            qualified_name,
                            ty.display(),
                            init_ty.display()
                        ));
                    }
                    // Store as a global constant (for now, just track the type)
                    // Could add a constants map if needed
                }
                _ => {}
            }
        }
    }

    fn add_machine_io_functions(&mut self) {
        // Windows API functions (from win32.fsl)
        self.add_win32_functions();
        
        // Conout functions
        self.add_conout_functions();
        
        // Conin functions
        self.add_conin_functions();
        
        // Conerr functions
        self.add_conerr_functions();
        
        // IO convenience functions
        self.add_io_convenience_functions();
    }

    fn add_win32_functions(&mut self) {
        // GetStdHandle(nStdHandle: i32): ptr[u8]
        self.functions.insert("GetStdHandle".into(), FnSignature {
            _name: "GetStdHandle".into(),
            generic_params: Vec::new(),
            params: vec![("nStdHandle".into(), Ty::I32, false)],
            return_ty: Ty::Ptr(Box::new(Ty::U8)),
        });
        // GetCurrentProcess(): ptr[u8]
        self.functions.insert("GetCurrentProcess".into(), FnSignature {
            _name: "GetCurrentProcess".into(),
            generic_params: Vec::new(),
            params: vec![],
            return_ty: Ty::Ptr(Box::new(Ty::U8)),
        });
        // WriteConsoleA(hConsoleOutput: ptr[u8], lpBuffer: ptr[u8], nNumberOfCharsToWrite: u32, lpNumberOfCharsWritten: ptr[u32], lpReserved: ptr[u8]): i32
        self.functions.insert("WriteConsoleA".into(), FnSignature {
            _name: "WriteConsoleA".into(),
            generic_params: Vec::new(),
            params: vec![
                ("hConsoleOutput".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("lpBuffer".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("nNumberOfCharsToWrite".into(), Ty::U32, false),
                ("lpNumberOfCharsWritten".into(), Ty::Ptr(Box::new(Ty::U32)), false),
                ("lpReserved".into(), Ty::Ptr(Box::new(Ty::U8)), false),
            ],
            return_ty: Ty::I32,
        });
// ReadConsoleA(hConsoleInput: ptr[u8], lpBuffer: ptr[u8], nNumberOfCharsToRead: u32, lpNumberOfCharsRead: ptr[u32], pInputControl: ptr[u8]): i32
        self.functions.insert("ReadConsoleA".into(), FnSignature {
            _name: "ReadConsoleA".into(),
            generic_params: Vec::new(),
            params: vec![
                ("hConsoleInput".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("lpBuffer".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("nNumberOfCharsToRead".into(), Ty::U32, false),
                ("lpNumberOfCharsRead".into(), Ty::Ptr(Box::new(Ty::U32)), false),
                ("pInputControl".into(), Ty::Ptr(Box::new(Ty::U8)), false),
            ],
            return_ty: Ty::I32,
        });
// WriteFile(hFile: ptr[u8], lpBuffer: ptr[u8], nNumberOfBytesToWrite: u32, lpNumberOfBytesWritten: ptr[u32], lpOverlapped: ptr[u8]): i32
        self.functions.insert("WriteFile".into(), FnSignature {
            _name: "WriteFile".into(),
            generic_params: Vec::new(),
            params: vec![
                ("hFile".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("lpBuffer".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("nNumberOfBytesToWrite".into(), Ty::U32, false),
                ("lpNumberOfBytesWritten".into(), Ty::Ptr(Box::new(Ty::U32)), false),
                ("lpOverlapped".into(), Ty::Ptr(Box::new(Ty::U8)), false),
            ],
            return_ty: Ty::I32,
        });
// ReadFile(hFile: ptr[u8], lpBuffer: ptr[u8], nNumberOfBytesToRead: u32, lpNumberOfBytesRead: ptr[u32], lpOverlapped: ptr[u8]): i32
        self.functions.insert("ReadFile".into(), FnSignature {
            _name: "ReadFile".into(),
            generic_params: Vec::new(),
            params: vec![
                ("hFile".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("lpBuffer".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("nNumberOfBytesToRead".into(), Ty::U32, false),
                ("lpNumberOfBytesRead".into(), Ty::Ptr(Box::new(Ty::U32)), false),
                ("lpOverlapped".into(), Ty::Ptr(Box::new(Ty::U8)), false),
            ],
            return_ty: Ty::I32,
        });
// FlushFileBuffers(hFile: ptr[u8]): i32
        self.functions.insert("FlushFileBuffers".into(), FnSignature {
            _name: "FlushFileBuffers".into(),
            generic_params: Vec::new(),
            params: vec![("hFile".into(), Ty::Ptr(Box::new(Ty::U8)), false)],
            return_ty: Ty::I32,
        });
// ExitProcess(uExitCode: u32): void
        self.functions.insert("ExitProcess".into(), FnSignature {
            _name: "ExitProcess".into(),
            generic_params: Vec::new(),
            params: vec![("uExitCode".into(), Ty::U32, false)],
            return_ty: Ty::Void,
        });
// GetConsoleMode(hConsoleHandle: ptr[u8], lpMode: ptr[u32]): i32
        self.functions.insert("GetConsoleMode".into(), FnSignature {
            _name: "GetConsoleMode".into(),
            generic_params: Vec::new(),
            params: vec![
                ("hConsoleHandle".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("lpMode".into(), Ty::Ptr(Box::new(Ty::U32)), false),
            ],
            return_ty: Ty::I32,
        });
// SetConsoleMode(hConsoleHandle: ptr[u8], dwMode: u32): i32
        self.functions.insert("SetConsoleMode".into(), FnSignature {
            _name: "SetConsoleMode".into(),
            generic_params: Vec::new(),
            params: vec![
                ("hConsoleHandle".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("dwMode".into(), Ty::U32, false),
            ],
            return_ty: Ty::I32,
        });
    }

    fn add_conout_functions(&mut self) {
// Conout.write(buf: ptr[u8], len: usize): isize
        self.functions.insert("Conout.write".into(), FnSignature {
            _name: "Conout.write".into(),
            generic_params: Vec::new(),
            params: vec![
                ("buf".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("len".into(), Ty::U64, false),
            ],
            return_ty: Ty::I64,
        });
// Conout.write_str(s: str): isize
        self.functions.insert("Conout.write_str".into(), FnSignature {
            _name: "Conout.write_str".into(),
            generic_params: Vec::new(),
            params: vec![("s".into(), Ty::Ptr(Box::new(Ty::U8)), false)],
            return_ty: Ty::I64,
        });
// Conout.writeln(buf: ptr[u8], len: usize): isize
        self.functions.insert("Conout.writeln".into(), FnSignature {
            _name: "Conout.writeln".into(),
            generic_params: Vec::new(),
            params: vec![
                ("buf".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("len".into(), Ty::U64, false),
            ],
            return_ty: Ty::I64,
        });
// Conout.writeln_str(s: str): isize
        self.functions.insert("Conout.writeln_str".into(), FnSignature {
            _name: "Conout.writeln_str".into(),
            generic_params: Vec::new(),
            params: vec![("s".into(), Ty::Ptr(Box::new(Ty::U8)), false)],
            return_ty: Ty::I64,
        });
// Conout.flush(): void
        self.functions.insert("Conout.flush".into(), FnSignature {
            _name: "Conout.flush".into(),
            generic_params: Vec::new(),
            params: vec![],
            return_ty: Ty::Void,
        });
// Conout.handle(): ptr[u8]
        self.functions.insert("Conout.handle".into(), FnSignature {
            _name: "Conout.handle".into(),
            generic_params: Vec::new(),
            params: vec![],
            return_ty: Ty::Ptr(Box::new(Ty::U8)),
        });
    }

    fn add_conin_functions(&mut self) {
// Conin.read(buf: ptr[u8], len: usize): isize
        self.functions.insert("Conin.read".into(), FnSignature {
            _name: "Conin.read".into(),
            generic_params: Vec::new(),
            params: vec![
                ("buf".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("len".into(), Ty::U64, false),
            ],
            return_ty: Ty::I64,
        });
// Conin.scanline(buf: ptr[u8], max_len: usize): isize
        self.functions.insert("Conin.scanline".into(), FnSignature {
            _name: "Conin.scanline".into(),
            generic_params: Vec::new(),
            params: vec![
                ("buf".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("max_len".into(), Ty::U64, false),
            ],
            return_ty: Ty::I64,
        });
// Conin.read_char(): i32
        self.functions.insert("Conin.read_char".into(), FnSignature {
            _name: "Conin.read_char".into(),
            generic_params: Vec::new(),
            params: vec![],
            return_ty: Ty::I32,
        });
// Conin.read_str(buf: ptr[u8], len: usize): isize
        self.functions.insert("Conin.read_str".into(), FnSignature {
            _name: "Conin.read_str".into(),
            generic_params: Vec::new(),
            params: vec![
                ("buf".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("len".into(), Ty::U64, false),
            ],
            return_ty: Ty::I64,
        });
// Conin.handle(): ptr[u8]
        self.functions.insert("Conin.handle".into(), FnSignature {
            _name: "Conin.handle".into(),
            generic_params: Vec::new(),
            params: vec![],
            return_ty: Ty::Ptr(Box::new(Ty::U8)),
        });
    }

    fn add_conerr_functions(&mut self) {
// Conerr.write(buf: ptr[u8], len: usize): isize
        self.functions.insert("Conerr.write".into(), FnSignature {
            _name: "Conerr.write".into(),
            generic_params: Vec::new(),
            params: vec![
                ("buf".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("len".into(), Ty::U64, false),
            ],
            return_ty: Ty::I64,
        });
// Conerr.write_str(s: str): isize
        self.functions.insert("Conerr.write_str".into(), FnSignature {
            _name: "Conerr.write_str".into(),
            generic_params: Vec::new(),
            params: vec![("s".into(), Ty::Ptr(Box::new(Ty::U8)), false)],
            return_ty: Ty::I64,
        });
// Conerr.writeln(buf: ptr[u8], len: usize): isize
        self.functions.insert("Conerr.writeln".into(), FnSignature {
            _name: "Conerr.writeln".into(),
            generic_params: Vec::new(),
            params: vec![
                ("buf".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("len".into(), Ty::U64, false),
            ],
            return_ty: Ty::I64,
        });
// Conerr.writeln_str(s: str): isize
        self.functions.insert("Conerr.writeln_str".into(), FnSignature {
            _name: "Conerr.writeln_str".into(),
            generic_params: Vec::new(),
            params: vec![("s".into(), Ty::Ptr(Box::new(Ty::U8)), false)],
            return_ty: Ty::I64,
        });
// Conerr.flush(): void
        self.functions.insert("Conerr.flush".into(), FnSignature {
            _name: "Conerr.flush".into(),
            generic_params: Vec::new(),
            params: vec![],
            return_ty: Ty::Void,
        });
// Conerr.handle(): ptr[u8]
        self.functions.insert("Conerr.handle".into(), FnSignature {
            _name: "Conerr.handle".into(),
            generic_params: Vec::new(),
            params: vec![],
            return_ty: Ty::Ptr(Box::new(Ty::U8)),
        });
    }

    fn add_io_convenience_functions(&mut self) {
// print(s: str): isize
        self.functions.insert("print".into(), FnSignature {
            _name: "print".into(),
            generic_params: Vec::new(),
            params: vec![("s".into(), Ty::Ptr(Box::new(Ty::U8)), false)],
            return_ty: Ty::I64,
        });
// println(s: str): isize
        self.functions.insert("println".into(), FnSignature {
            _name: "println".into(),
            generic_params: Vec::new(),
            params: vec![("s".into(), Ty::Ptr(Box::new(Ty::U8)), false)],
            return_ty: Ty::I64,
        });
// eprint(s: str): isize
        self.functions.insert("eprint".into(), FnSignature {
            _name: "eprint".into(),
            generic_params: Vec::new(),
            params: vec![("s".into(), Ty::Ptr(Box::new(Ty::U8)), false)],
            return_ty: Ty::I64,
        });
// eprintln(s: str): isize
        self.functions.insert("eprintln".into(), FnSignature {
            _name: "eprintln".into(),
            generic_params: Vec::new(),
            params: vec![("s".into(), Ty::Ptr(Box::new(Ty::U8)), false)],
            return_ty: Ty::I64,
        });
// read_line(buf: ptr[u8], len: usize): isize
        self.functions.insert("read_line".into(), FnSignature {
            _name: "read_line".into(),
            generic_params: Vec::new(),
            params: vec![
                ("buf".into(), Ty::Ptr(Box::new(Ty::U8)), false),
                ("len".into(), Ty::U64, false),
            ],
            return_ty: Ty::I64,
        });
// exit(code: i32): void
        self.functions.insert("exit".into(), FnSignature {
            _name: "exit".into(),
            generic_params: Vec::new(),
            params: vec![("code".into(), Ty::I32, false)],
            return_ty: Ty::Void,
        });
    }

    fn validate_machine_mode(&mut self, _program: &Program) {
        match self.machine_mode {
            Some(true) => {
                // MACHINE mode: require a `main` function with a valid signature.
                let has_main = self.functions.contains_key("main");
                if !has_main {
                    self.errors.push("MACHINE mode requires a 'main' function".into());
                    return;
                }

                let sig = self.functions.get("main").unwrap();
                // Return type must be i32.
                if !sig.return_ty.types_eq(&Ty::I32) {
                    self.errors.push(format!(
                        "main in MACHINE mode must return i32, got {}",
                        sig.return_ty.display()
                    ));
                }
                // Parameters: either none, or (i32, ptr[ptr[u8]]).
                match sig.params.len() {
                    0 => {} // FN main(): i32
                    2 => {
                        let (_, p0_ty, _) = &sig.params[0];
                        let (_, p1_ty, _) = &sig.params[1];
                        if !p0_ty.types_eq(&Ty::I32) {
                            self.errors.push(format!(
                                "main argc parameter must be i32, got {}",
                                p0_ty.display()
                            ));
                        }
                        if !Self::is_ptr_ptr_u8(p1_ty) {
                            self.errors.push("main argv parameter must be ptr[ptr[u8]]".into());
                        }
                    }
                    n => {
                        self.errors.push(format!(
                            "main must have 0 or 2 parameters, got {}",
                            n
                        ));
                    }
                }
            }
            Some(false) => {
                // NO_MACHINE mode: no main required; entry is user-defined.
            }
            None => {
                // No platform declaration; no requirement.
            }
        }
    }

    // ---------- item analysis ----------

    fn analyze_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => self.analyze_function(f),
            Item::StructDecl(_) => {}
            Item::EnumDecl(_) => {}
            Item::ModuleDecl(m) => self.analyze_module(m),
            Item::MachineDecl(_) => {}
            Item::ExternDecl(_) => {}
            Item::ImportDecl(_) => {}
            Item::ConstDecl(_) => {} // Already type-checked in declaration collection
            Item::VarDecl(_) => {} // Already type-checked in declaration collection
            Item::AsmBlock(asm) => {
                // Type-check asm operands
                for instr in &asm.instructions {
                    for operand in &instr.operands {
                        let _ = self.type_of_expr(&operand.expr);
                    }
                }
            }
        }
    }

    fn analyze_module(&mut self, m: &ModuleDecl) {
        // Enter module scope
        self.scopes.push(Scope::new());

        for item in &m.items {
            self.analyze_item(item);
        }

        // Module scope exit: check owners
        self.check_scope_exit_owners();
        self.scopes.pop();
    }

    fn analyze_function(&mut self, f: &Function) {
        let signature = self.functions.get(&f.name).cloned().unwrap_or(FnSignature {
            _name: f.name.clone(),
            generic_params: Vec::new(),
            params: Vec::new(),
            return_ty: Ty::Error,
        });

        self.current_fn_return = signature.return_ty.clone();

        // enter function scope
        self.scopes.push(Scope::new());

        // add parameters
        for param in &f.params {
            let ty = self.ast_type_to_ty(&param.ty);
            let is_owner = ty.is_owner();
            let state = VarState {
                _name: param.name.clone(),
                ty,
                is_const: true,
                is_owner,
                is_initialized: true,
                is_moved: false,
            };
            self.scopes.last_mut().unwrap().insert(param.name.clone(), state);
        }

        // analyze body
        self.analyze_block(&f.body);

        // scope exit: check owners are not leaked
        self.check_scope_exit_owners();

        self.scopes.pop();
    }

    // ---------- block / statement analysis ----------

    fn analyze_block(&mut self, block: &Block) {
        // enter block scope
        self.scopes.push(Scope::new());

        for stmt in &block.statements {
            self.analyze_statement(stmt);
        }

        // scope exit: check owners
        self.check_scope_exit_owners();

        self.scopes.pop();
    }

    fn analyze_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VarDecl { name, ty, is_const, init } => {
                let declared_ty = self.ast_type_to_ty(ty);
                let is_owner = declared_ty.is_owner();

                // type-check initializer
                if let Some(init_expr) = init {
                    let init_ty = self.type_of_expr(init_expr);
                    
                    // For union types, allow initialization with any type that matches a union variant
                    // or with an exact union type match
                    let type_matches = match &declared_ty {
                        Ty::Union(union_tys) => {
                            union_tys.iter().any(|ut| init_ty.types_eq(ut)) || declared_ty.types_eq(&init_ty)
                        },
                        _ => declared_ty.types_eq(&init_ty),
                    };
                    
                    if !type_matches && init_ty != Ty::Error {
                        self.errors.push(format!(
                            "type mismatch: cannot initialize '{}' of type {} with {}",
                            name,
                            declared_ty.display(),
                            init_ty.display()
                        ));
                    }

                    // ownership: if initializing an owner from an identifier that is itself an owner,
                    // that is a move.
                    if is_owner && declared_ty.inner_cloned().is_some()
                        && let Expr::Ident(ref src_name) = *init_expr
                    {
                        self.mark_move_from(src_name, init_expr);
                    }

                    let state = VarState {
                        _name: name.clone(),
                        ty: declared_ty,
                        is_const: *is_const,
                        is_owner,
                        is_initialized: true,
                        is_moved: false,
                    };
                    self.current_scope_mut().insert(name.clone(), state);
                } else {
                    // no initializer: owner must be explicitly assigned before use
                    let state = VarState {
                        _name: name.clone(),
                        ty: declared_ty,
                        is_const: *is_const,
                        is_owner,
                        is_initialized: false,
                        is_moved: false,
                    };
                    self.current_scope_mut().insert(name.clone(), state);
                }
            }

            Statement::Assignment { target, value } => {
                let value_ty = self.type_of_expr(value);

                match target {
                    AssignmentTarget::Ident(name) => {
                        self.check_ident_assignment(name, &value_ty, value);
                    }
                    AssignmentTarget::Member { object, member } => {
                        let obj_ty = self.type_of_expr(object);
                        match obj_ty.inner_cloned().as_ref() {
                            Some(Ty::Named(struct_name)) => {
                                if let Some(struct_def) = self.structs.get(struct_name) {
                                    if let Some((_, field_ty)) =
                                        struct_def.fields.iter().find(|(n, _)| n == member)
                                    {
                                        if !field_ty.types_eq(&value_ty) && value_ty != Ty::Error {
                                            self.errors.push(format!(
                                                "type mismatch: cannot assign {} to field '{}' of type {}",
                                                value_ty.display(),
                                                member,
                                                field_ty.display()
                                            ));
                                        }
                                    } else {
                                        self.errors.push(format!(
                                            "struct '{}' has no field '{}'",
                                            struct_name, member
                                        ));
                                    }
                                } else {
                                    self.errors.push(format!("unknown struct '{}'", struct_name));
                                }
                            }
                            _ => {
                                self.errors.push(format!(
                                    "cannot access field '{}' on non-struct type {}",
                                    member,
                                    obj_ty.display()
                                ));
                            }
                        }
                    }
                    AssignmentTarget::Index { object, index } => {
                        let _obj_ty = if let Expr::Ident(name) = object.as_ref() {
                            if let Some(var) = self.lookup_var(name) {
                                let ty = var.ty.clone();
                                if let Some(var_mut) = self.lookup_var_mut(name) {
                                    var_mut.is_initialized = true;
                                }
                                ty
                            } else {
                                self.errors.push(format!("undeclared variable '{}'", name));
                                Ty::Error
                            }
                        } else {
                            self.type_of_expr(object)
                        };
                        let index_ty = self.type_of_expr(index);
                        if index_ty != Ty::I32 && index_ty != Ty::I64 && index_ty != Ty::Error {
                            self.errors.push(format!("array index must be integer, got {}", index_ty.display()));
                        }
                        // element type check skipped: depends on array/ptr type
                    }
                    AssignmentTarget::VolatileDeref { object } => {
                        let obj_ty = self.type_of_expr(object);
                        match obj_ty {
                            Ty::Ptr(_) | Ty::Own(_) => {}
                            _ => {
                                self.errors.push(format!("VOLATILE store requires pointer type, got {}", obj_ty.display()));
                            }
                        }
                        // element type check skipped: depends on ptr type
                    }
                    AssignmentTarget::Tuple(targets) => {
                        // multiple assignment: value must be a tuple
                        match &value_ty {
                            Ty::Tuple(elem_tys) => {
                                if elem_tys.len() != targets.len() {
                                    self.errors.push(format!(
                                        "assignment arity mismatch: expected {} values, got {}",
                                        targets.len(),
                                        elem_tys.len()
                                    ));
                                }
                                for (tgt, elem_ty) in targets.iter().zip(elem_tys.iter()) {
                                    match tgt {
                                        AssignmentTarget::Ident(name) => {
                                            // build a synthetic expr for type + move check
                                            self.check_ident_assignment(name, elem_ty, value);
                                        }
                                        _ => {
                                            self.errors.push("nested tuple assignment target not supported".into());
                                        }
                                    }
                                }
                            }
                            _ => {
                                self.errors.push("right-hand side of tuple assignment must be a tuple".into());
                            }
                        }
                    }
                }
            }

            Statement::If { condition, then_branch, else_branch } => {
                let cond_ty = self.type_of_expr(condition);
                if cond_ty != Ty::Bool && cond_ty != Ty::Error {
                    self.errors.push(format!("if condition must be bool, got {}", cond_ty.display()));
                }

                // snapshot state before branches
                let before = self.current_scope().clone();

                // then branch
                self.analyze_block(then_branch);
                let then_state = self.current_scope().clone();

                // restore and do else
                self.pop_scope_and_replace(before.clone());
                if let Some(else_branch) = else_branch {
                    self.analyze_block(else_branch);
                }
                let else_state = self.current_scope().clone();

                // merge: moved if moved in either branch
                let merged = self.merge_scopes(&then_state, &else_state, &before);
                self.replace_current_scope(merged);
            }

            Statement::While { condition, body } => {
                let cond_ty = self.type_of_expr(condition);
                if cond_ty != Ty::Bool && cond_ty != Ty::Error {
                    self.errors.push(format!("while condition must be bool, got {}", cond_ty.display()));
                }
                let before = self.current_scope().clone();
                self.analyze_block(body);
                // after loop, variables may have been moved conservatively
                let after = self.current_scope().clone();
                let merged = self.merge_scopes(&after, &before, &before);
                self.replace_current_scope(merged);
            }

            Statement::For { variable, start, end, body } => {
                let start_ty = self.type_of_expr(start);
                let end_ty = self.type_of_expr(end);
                if start_ty != Ty::I32 && start_ty != Ty::I64 && start_ty != Ty::Error {
                    self.errors.push(format!("for start must be integer, got {}", start_ty.display()));
                }
                if end_ty != Ty::I32 && end_ty != Ty::I64 && end_ty != Ty::Error {
                    self.errors.push(format!("for end must be integer, got {}", end_ty.display()));
                }

                self.scopes.push(Scope::new());
                let state = VarState {
                    _name: variable.clone(),
                    ty: Ty::I32,
                    is_const: false,
                    is_owner: false,
                    is_initialized: true,
                    is_moved: false,
                };
                self.current_scope_mut().insert(variable.clone(), state);

                self.analyze_block(body);

                self.check_scope_exit_owners();
                self.scopes.pop();
            }

            Statement::Return { values } => {
                let expected = self.current_fn_return.clone();
                match &expected {
                    Ty::Void => {
                        if !values.is_empty() {
                            self.errors.push("function returning void cannot return values".into());
                        }
                    }
                    Ty::Tuple(elem_tys) => {
                        if values.len() != elem_tys.len() {
                            self.errors.push(format!(
                                "return arity mismatch: expected {} values, got {}",
                                elem_tys.len(),
                                values.len()
                            ));
                        }
                        for (val, exp_ty) in values.iter().zip(elem_tys.iter()) {
                            let val_ty = self.type_of_expr(val);
                            if !val_ty.types_eq(exp_ty) && val_ty != Ty::Error {
                                self.errors.push(format!(
                                    "return type mismatch: expected {}, got {}",
                                    exp_ty.display(),
                                    val_ty.display()
                                ));
                            }
                            // if returning an owner from an ident, mark moved
                            if exp_ty.is_owner()
                                && let Expr::Ident(ref name) = *val
                            {
                                self.mark_move_from(name, val);
                            }
                        }
                    }
                    Ty::Union(union_tys) => {
                        // Union return: exactly one value that matches one of the union types
                        if values.len() != 1 {
                            self.errors.push(format!(
                                "return arity mismatch: union return expects 1 value, got {}",
                                values.len()
                            ));
                        } else if let Some(val) = values.first() {
                            let val_ty = self.type_of_expr(val);
                            // Check if the value type matches any of the union's element types
                            let matches = union_tys.iter().any(|ut| val_ty.types_eq(ut));
                            if !matches && val_ty != Ty::Error {
                                self.errors.push(format!(
                                    "return type mismatch: union [{}] does not contain type {}",
                                    union_tys.iter().map(|t| t.display()).collect::<Vec<_>>().join(", "),
                                    val_ty.display()
                                ));
                            }
                            // if returning an owner from an ident, mark moved
                            if union_tys.iter().any(|ut| ut.is_owner())
                                && let Expr::Ident(ref name) = *val
                            {
                                self.mark_move_from(name, val);
                            }
                        }
                    }
                    _ => {
                        if values.len() != 1 {
                            self.errors.push(format!(
                                "return arity mismatch: expected 1 value, got {}",
                                values.len()
                            ));
                        }
                        if let Some(val) = values.first() {
                            let val_ty = self.type_of_expr(val);
                            if !val_ty.types_eq(&expected) && val_ty != Ty::Error {
                                self.errors.push(format!(
                                    "return type mismatch: expected {}, got {}",
                                    expected.display(),
                                    val_ty.display()
                                ));
                            }
                            if expected.is_owner()
                                && let Expr::Ident(ref name) = *val
                            {
                                self.mark_move_from(name, val);
                            }
                        }
                    }
                }
            }

            Statement::Match { expr, arms, else_branch } => {
                self.analyze_match(expr, arms, else_branch);
            }

            Statement::Expr(expr) => {
                self.type_of_expr(expr);
            }

            Statement::AsmBlock(asm) => {
                // Type-check each operand in asm instructions
                for instr in &asm.instructions {
                    for operand in &instr.operands {
                        let _ = self.type_of_expr(&operand.expr);
                    }
                }
                // TODO: Validate clobbers, flags, etc.
            }
            Statement::Unreachable => {
                // Mark subsequent code as unreachable (for now just a no-op)
            }
        }
    }

    fn analyze_match(
        &mut self,
        expr: &Expr,
        arms: &[MatchArm],
        else_branch: &Option<Block>,
    ) {
        let expr_ty = self.type_of_expr(expr);
        if expr_ty == Ty::Error {
            return;
        }

        let before = self.current_scope().clone();
        let mut arm_scopes = Vec::new();

        for arm in arms {
            self.pop_scope_and_replace(before.clone());
            self.scopes.push(Scope::new());

            match &arm.pattern {
                Pattern::Literal(lit) => {
                    let lit_ty = match lit {
                        Literal::Int(_) => Ty::I32,
                        Literal::Float(_) => Ty::F64,
                        Literal::String(_) => Ty::Named("string".into()),
                        Literal::Char(_) => Ty::I8,
                        Literal::Bool(_) => Ty::Bool,
                        Literal::Null => Ty::NullPtr,
                    };
                    if !expr_ty.types_eq(&lit_ty) {
                        self.errors.push(format!(
                            "pattern type mismatch: cannot match literal of type {} against {}",
                            lit_ty.display(),
                            expr_ty.display()
                        ));
                    }
                }
                Pattern::Variant { enum_name, variant_name } => {
                    self.validate_pattern_variant(&expr_ty, enum_name.as_deref(), variant_name, false, None);
                }
                Pattern::VariantWithPayload { enum_name, variant_name, binding } => {
                    self.validate_pattern_variant(&expr_ty, enum_name.as_deref(), variant_name, true, Some(binding));
                }
                Pattern::Wildcard => {}
            }

            self.analyze_block(&arm.body);
            self.check_scope_exit_owners();
            arm_scopes.push(self.current_scope().clone());
            self.scopes.pop();
        }

        if let Some(eb) = else_branch {
            self.pop_scope_and_replace(before.clone());
            self.analyze_block(eb);
            arm_scopes.push(self.current_scope().clone());
        }

        // Merge scopes
        if !arm_scopes.is_empty() {
            let mut merged = arm_scopes[0].clone();
            for s in &arm_scopes[1..] {
                merged = self.merge_scopes(&merged, s, &before);
            }
            self.replace_current_scope(merged);
        }
    }

    fn validate_pattern_variant(
        &mut self,
        expr_ty: &Ty,
        pattern_enum: Option<&str>,
        variant_name: &str,
        has_payload_binding: bool,
        binding: Option<&str>,
    ) {
        let enum_name = match expr_ty {
            Ty::Named(n) => n.clone(),
            _ => {
                self.errors.push(format!(
                    "cannot match enum variant '{}' against non-enum type {}",
                    variant_name,
                    expr_ty.display()
                ));
                return;
            }
        };

        if let Some(pe) = pattern_enum
            && pe != enum_name
        {
            self.errors.push(format!(
                "pattern enum '{}' does not match matched expression enum '{}'",
                pe, enum_name
            ));
            return;
        }

        let edef = match self.enums.get(&enum_name) {
            Some(e) => e.clone(),
            None => {
                self.errors.push(format!("'{}' is not an enum", enum_name));
                return;
            }
        };

        let var_match = edef.variants.iter().find(|(vn, _)| vn == variant_name);
        match var_match {
            None => {
                self.errors.push(format!(
                    "enum '{}' has no variant '{}'",
                    enum_name, variant_name
                ));
            }
            Some((_, payload_ty)) => {
                match (payload_ty, has_payload_binding) {
                    (Some(pty), true) => {
                        if let Some(b) = binding {
                            let state = VarState {
                                _name: b.to_string(),
                                ty: pty.clone(),
                                is_const: true,
                                is_owner: pty.is_owner(),
                                is_initialized: true,
                                is_moved: false,
                            };
                            self.current_scope_mut().insert(b.to_string(), state);
                        }
                    }
                    (Some(_), false) => {
                        self.errors.push(format!(
                            "variant '{}.{}' requires a payload",
                            enum_name, variant_name
                        ));
                    }
                    (None, true) => {
                        self.errors.push(format!(
                            "variant '{}.{}' does not take a payload",
                            enum_name, variant_name
                        ));
                    }
                    (None, false) => {}
                }
            }
        }
    }

    // ---------- expression type inference ----------

    fn type_of_expr(&mut self, expr: &Expr) -> Ty {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => Ty::I32,
                Literal::Float(_) => Ty::F64,
                Literal::String(_) => Ty::Named("string".into()),
                Literal::Char(_) => Ty::I8,
                Literal::Bool(_) => Ty::Bool,
                Literal::Null => Ty::NullPtr,
            },
            Expr::Ident(name) => {
                if let Some(var) = self.lookup_var(name) {
                    if var.is_moved {
                        self.errors.push(format!("use of moved variable '{}'", name));
                        return Ty::Error;
                    }
                    if !var.is_initialized {
                        self.errors.push(format!("use of possibly uninitialized variable '{}'", name));
                        return Ty::Error;
                    }
                    var.ty.clone()
                } else {
                    // Check if bare unit enum variant
                    let mut found_variant = None;
                    for (en, edef) in &self.enums {
                        if let Some((_, opt_pty)) = edef.variants.iter().find(|(vn, _)| vn == name) {
                            found_variant = Some((en.clone(), opt_pty.clone()));
                            break;
                        }
                    }
                    if let Some((en, opt_pty)) = found_variant {
                        if opt_pty.is_none() {
                            Ty::Named(en)
                        } else {
                            self.errors.push(format!("enum variant '{}' requires payload arguments", name));
                            Ty::Error
                        }
                    } else {
                        self.errors.push(format!("undeclared variable '{}'", name));
                        Ty::Error
                    }
                }
            }
            Expr::Binary { left, op, right } => {
                let lt = self.type_of_expr(left);
                let rt = self.type_of_expr(right);
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                        if lt != Ty::Error && rt != Ty::Error && lt != rt {
                            self.errors.push(format!(
                                "binary operator {:?} requires same-type operands, got {} and {}",
                                op, lt.display(), rt.display()
                            ));
                        }
                        lt
                    }
                    BinaryOp::Eq | BinaryOp::Neq => Ty::Bool,
                    BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge => {
                        if lt != Ty::Error && rt != Ty::Error && lt != rt {
                            self.errors.push(format!(
                                "comparison requires same-type operands, got {} and {}",
                                lt.display(), rt.display()
                            ));
                        }
                        Ty::Bool
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if lt != Ty::Bool && lt != Ty::Error {
                            self.errors.push(format!("logical operand must be bool, got {}", lt.display()));
                        }
                        if rt != Ty::Bool && rt != Ty::Error {
                            self.errors.push(format!("logical operand must be bool, got {}", rt.display()));
                        }
                        Ty::Bool
                    }
                    BinaryOp::Amp | BinaryOp::Pipe | BinaryOp::Xor | BinaryOp::LShift | BinaryOp::RShift => {
                        if lt != Ty::Error && rt != Ty::Error && lt != rt {
                            self.errors.push(format!(
                                "bitwise operator requires same-type operands, got {} and {}",
                                lt.display(), rt.display()
                            ));
                        }
                        lt
                    }
                }
            }
            Expr::Unary { op, operand } => {
                if *op == UnaryOp::Amp {
                    if let Expr::Ident(ref name) = **operand {
                        if let Some(var) = self.lookup_var(name) {
                            if var.is_moved {
                                self.errors.push(format!("cannot borrow moved variable '{}'", name));
                                return Ty::Error;
                            }
                            let ty = var.ty.clone();
                            if let Some(var_mut) = self.lookup_var_mut(name) {
                                var_mut.is_initialized = true;
                            }
                            return Ty::Ptr(Box::new(ty));
                        } else {
                            self.errors.push(format!("undeclared variable '{}'", name));
                            return Ty::Error;
                        }
                    } else {
                        self.errors.push("address-of requires an identifier".to_string());
                        return Ty::Error;
                    }
                }
                let ot = self.type_of_expr(operand);
                match op {
                    UnaryOp::Neg => {
                        if ot != Ty::I8 && ot != Ty::I16 && ot != Ty::I32 && ot != Ty::I64
                            && ot != Ty::U8 && ot != Ty::U16 && ot != Ty::U32 && ot != Ty::U64
                            && ot != Ty::F32 && ot != Ty::F64 && ot != Ty::Error
                        {
                            self.errors.push(format!("unary minus requires numeric operand, got {}", ot.display()));
                        }
                        ot
                    }
                    UnaryOp::Not => {
                        if ot != Ty::Bool && ot != Ty::Error {
                            self.errors.push(format!("unary not requires bool operand, got {}", ot.display()));
                        }
                        Ty::Bool
                    }
                    UnaryOp::Amp => {
                        // borrow: returns pointer to operand type
                        Ty::Ptr(Box::new(ot))
                    }
                    UnaryOp::Star => {
                        // deref: pointer -> inner type
                        match ot {
                            Ty::Ptr(inner) | Ty::Own(inner) => (*inner).clone(),
                            _ => {
                                self.errors.push(format!("cannot dereference type {}", ot.display()));
                                Ty::Error
                            }
                        }
                    }
                    UnaryOp::Volatile => {
                        // volatile load/store: type passes through, codegen handles volatility
                        ot
                    }
                    UnaryOp::VolatileDeref => {
                        // volatile deref: like Star but marks as volatile for codegen
                        match ot {
                            Ty::Ptr(inner) | Ty::Own(inner) => (*inner).clone(),
                            _ => {
                                self.errors.push(format!("cannot dereference type {}", ot.display()));
                                Ty::Error
                            }
                        }
                    }
                    UnaryOp::Tilde => ot,
                }
            }
            Expr::Call { callee, args } => {
                match callee.as_ref() {
                    Expr::GenericInst { callee: inner_callee, type_args } => {
                        // Generic function instantiation
                        if let Expr::Ident(func_name) = inner_callee.as_ref() {
                            if let Some(sig) = self.functions.get(func_name).cloned() {
                                // Check generic parameter count matches
                                if sig.generic_params.len() != type_args.len() {
                                    self.errors.push(format!(
                                        "function '{}' expects {} generic arguments, got {}",
                                        func_name,
                                        sig.generic_params.len(),
                                        type_args.len()
                                    ));
                                    return Ty::Error;
                                }
                                // Convert AST type_args to Ty
                                let concrete_args: Vec<Ty> = type_args.iter()
                                    .map(|t| self.ast_type_to_ty(t))
                                    .collect();
                                
                                // Substitute generic parameters in parameter types and return type
                                let subst_params: Vec<(String, Ty, bool)> = sig.params.iter()
                                    .map(|(name, ty, is_own)| {
                                        let subst_ty = self.substitute_generics(ty, &sig.generic_params, &concrete_args);
                                        (name.clone(), subst_ty, *is_own)
                                    })
                                    .collect();
                                let subst_return_ty = self.substitute_generics(&sig.return_ty, &sig.generic_params, &concrete_args);
                                
                                // Check regular arguments
                                if args.len() != subst_params.len() {
                                    self.errors.push(format!(
                                        "call to '{}' expects {} arguments, got {}",
                                        func_name,
                                        subst_params.len(),
                                        args.len()
                                    ));
                                    return Ty::Error;
                                }
                                for (arg, (param_name, param_ty, is_own)) in
                                    args.iter().zip(subst_params.iter())
                                {
                                    let arg_ty = self.type_of_expr(arg);
                                    if !arg_ty.types_eq(param_ty) && arg_ty != Ty::Error {
                                        self.errors.push(format!(
                                            "argument '{}' type mismatch: expected {}, got {}",
                                            param_name,
                                            param_ty.display(),
                                            arg_ty.display()
                                        ));
                                    }
                                    if *is_own
                                        && let Expr::Ident(ref name) = *arg
                                    {
                                        self.mark_move_from(name, arg);
                                    }
                                }
                                subst_return_ty
                            } else {
                                self.errors.push(format!("undeclared function '{}'", func_name));
                                Ty::Error
                            }
                        } else {
                            self.errors.push("generic instantiation requires function identifier".into());
                            Ty::Error
                        }
                    }
                    Expr::Ident(func_name) => {
                        self.call_function(func_name, args)
                    }
                    Expr::Member { object, member } => {
                        // Check if bare enum variant constructor (e.g., Enum.Variant) FIRST
                        // This must come before qualified function check to avoid treating Enum.Variant as a function
                        if let Expr::Ident(ref enum_name) = **object
                            && let Some(edef) = self.enums.get(enum_name).cloned()
                        {
                            // Look up variant by member name
                            if let Some((_, opt_pty)) = edef.variants.iter().find(|(vn, _)| vn == member) {
                                match opt_pty {
                                    Some(pty) => {
                                        if args.len() != 1 {
                                            self.errors.push(format!("variant '{}.{}' expects 1 argument, got {}", enum_name, member, args.len()));
                                            return Ty::Error;
                                        }
                                        let arg_ty = self.type_of_expr(&args[0]);
                                        if !arg_ty.types_eq(&pty) && arg_ty != Ty::Error {
                                            self.errors.push(format!("payload type mismatch for '{}.{}': expected {}, got {}", enum_name, member, pty.display(), arg_ty.display()));
                                        }
                                        return Ty::Named(enum_name.clone());
                                    }
                                    None => {
                                        self.errors.push(format!("variant '{}.{}' does not take arguments", enum_name, member));
                                        return Ty::Error;
                                    }
                                }
                            } else {
                                self.errors.push(format!("enum '{}' has no variant '{}'", enum_name, member));
                                return Ty::Error;
                            }
                        }
                        
                        // Try qualified function name (e.g., IO.println)
                        if let Some(qualified_name) = self.extract_qualified_name(callee.as_ref()) {
                            return self.call_function(&qualified_name, args);
                        }
                        
                        // Not a qualified function or enum variant constructor
                        self.errors.push("invalid qualified function call".into());
                        return Ty::Error;
                    }
                    _ => {
                        self.errors.push("indirect function calls not yet supported".into());
                        return Ty::Error;
                    }
                    }
                    }
                    Expr::Member { object, member } => {
                if let Expr::Ident(ref name) = **object
                    && let Some(edef) = self.enums.get(name).cloned()
                {
                    match edef.variants.iter().find(|(vn, _)| vn == member) {
                        Some((_, None)) => Ty::Named(name.clone()),
                        Some((_, Some(_))) => {
                            Ty::Named(name.clone())
                        }
                        None => {
                            self.errors.push(format!("enum '{}' has no variant '{}'", name, member));
                            Ty::Error
                        }
                    }
                } else {
                    let obj_ty = self.type_of_expr(object);
                    let obj_ty_match = obj_ty.clone();
                    match obj_ty_match {
                    Ty::Named(ref struct_name) => {
                        if let Some(struct_def) = self.structs.get(struct_name) {
                            match struct_def.fields.iter().find(|(n, _)| n == member) {
                                Some((_, field_ty)) => field_ty.clone(),
                                None => {
                                    self.errors.push(format!(
                                        "struct '{}' has no field '{}'",
                                        struct_name, member
                                    ));
                                    Ty::Error
                                }
                            }
                        } else {
                            self.errors.push(format!("unknown struct '{}'", struct_name));
                            Ty::Error
                        }
                    }
                    Ty::Ptr(inner) | Ty::Own(inner) => {
                        match &*inner {
                            Ty::Named(struct_name) => {
                                if let Some(struct_def) = self.structs.get(struct_name) {
                                    match struct_def.fields.iter().find(|(n, _)| n == member) {
                                        Some((_, field_ty)) => field_ty.clone(),
                                        None => {
                                            self.errors.push(format!(
                                                "struct '{}' has no field '{}'",
                                                struct_name, member
                                            ));
                                            Ty::Error
                                        }
                                    }
                                } else {
                                    self.errors.push(format!("unknown struct '{}'", struct_name));
                                    Ty::Error
                                }
                            }
                            _ => {
                                self.errors.push(format!(
                                    "cannot access field '{}' on non-struct type {}",
                                    member,
                                    obj_ty.display()
                                ));
                                Ty::Error
                            }
                        }
                    }
                    _ => {
                        self.errors.push(format!(
                            "cannot access field '{}' on type {}",
                            member,
                            obj_ty.display()
                        ));
                        Ty::Error
                    }
                }
            }
        }
            Expr::Index { object, index } => {
                let obj_ty = self.type_of_expr(object);
                let obj_ty_match = obj_ty.clone();
                let _index_ty = self.type_of_expr(index);
                match obj_ty_match {
                    Ty::Array(inner, _) => (*inner).clone(),
                    Ty::Ptr(inner) | Ty::Own(inner) => {
                        if let Ty::Array(elem, _) = &*inner {
                            *elem.clone()
                        } else {
                            (*inner).clone()
                        }
                    }
                    _ => {
                        self.errors.push(format!("cannot index type {}", obj_ty.display()));
                        Ty::Error
                    }
                }
            }
            Expr::Tuple(elems) => {
                let elem_tys: Vec<Ty> = elems.iter().map(|e| self.type_of_expr(e)).collect();
                Ty::Tuple(elem_tys)
            }
            Expr::GenericInst { callee, type_args: _ } => {
                // Generic instantiation - the type is the return type of the specialized function
                // For now, just type-check the callee
                self.type_of_expr(callee)
            }
            Expr::Cast { expr, ty } => {
                let _ = self.type_of_expr(expr);
                self.ast_type_to_ty(ty)
            }
        }
    }

    // ---------- helpers ----------

    fn current_scope(&self) -> &Scope {
        self.scopes.last().expect("scope stack empty")
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("scope stack empty")
    }

    fn lookup_var(&self, name: &str) -> Option<&VarState> {
        // search from innermost scope outward
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn lookup_var_mut(&mut self, name: &str) -> Option<&mut VarState> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(v) = scope.get_mut(name) {
                return Some(v);
            }
        }
        None
    }

    fn ast_type_to_ty(&self, ty: &Type) -> Ty {
        match ty {
            Type::Bool => Ty::Bool,
            Type::I8 => Ty::I8,
            Type::I16 => Ty::I16,
            Type::I32 => Ty::I32,
            Type::I64 => Ty::I64,
            Type::U8 => Ty::U8,
            Type::U16 => Ty::U16,
            Type::U32 => Ty::U32,
            Type::U64 => Ty::U64,
            Type::F32 => Ty::F32,
            Type::F64 => Ty::F64,
            Type::Void => Ty::Void,
            Type::Ptr(inner) => Ty::Ptr(Box::new(self.ast_type_to_ty(inner))),
            Type::Own(inner) => Ty::Own(Box::new(self.ast_type_to_ty(inner))),
            Type::Named(n) => Ty::Named(n.clone()),
            Type::Array(inner, n) => Ty::Array(Box::new(self.ast_type_to_ty(inner)), *n),
            Type::Tuple(ts) => {
                Ty::Tuple(ts.iter().map(|t| self.ast_type_to_ty(t)).collect())
            }
            Type::Union(ts) => {
                Ty::Union(ts.iter().map(|t| self.ast_type_to_ty(t)).collect())
            }
            Type::Generic(name) => {
                Ty::Generic(name.clone())
            }
        }
    }

    /// Substitute generic type parameters in a Ty with concrete types.
    /// `generic_params` are the names of generic parameters (e.g., ["T", "U"])
    /// `type_args` are the concrete types to substitute (e.g., [Ty::I32, Ty::Bool])
    fn substitute_generics(&self, ty: &Ty, generic_params: &[String], type_args: &[Ty]) -> Ty {
        let mut subst_map: std::collections::HashMap<String, Ty> = generic_params.iter()
            .zip(type_args.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.substitute_generics_rec(ty, &subst_map)
    }

    fn substitute_generics_rec(&self, ty: &Ty, subst_map: &std::collections::HashMap<String, Ty>) -> Ty {
        match ty {
            Ty::Generic(name) => {
                subst_map.get(name).cloned().unwrap_or_else(|| Ty::Generic(name.clone()))
            }
            Ty::Ptr(inner) => Ty::Ptr(Box::new(self.substitute_generics_rec(inner, subst_map))),
            Ty::Own(inner) => Ty::Own(Box::new(self.substitute_generics_rec(inner, subst_map))),
            Ty::Named(n) => Ty::Named(n.clone()),
            Ty::Array(inner, n) => Ty::Array(Box::new(self.substitute_generics_rec(inner, subst_map)), *n),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.substitute_generics_rec(t, subst_map)).collect()),
            Ty::Union(ts) => Ty::Union(ts.iter().map(|t| self.substitute_generics_rec(t, subst_map)).collect()),
            other => other.clone(),
        }
    }

    /// Returns true if `ty` is `ptr[ptr[u8]]` (the argv parameter type).
    fn is_ptr_ptr_u8(ty: &Ty) -> bool {
        matches!(
            ty,
            Ty::Ptr(inner) if matches!(
                inner.as_ref(),
                Ty::Ptr(inner2) if matches!(&**inner2, Ty::U8)
            )
        )
    }

    fn check_ident_assignment(&mut self, name: &str, value_ty: &Ty, value_expr: &Expr) {
        // find the variable in any scope (innermost first) and record its index
        let scope_idx = match self
            .scopes
            .iter()
            .rev()
            .position(|s| s.vars.contains_key(name))
        {
            Some(idx) => idx,
            None => {
                self.errors.push(format!("undeclared variable '{}'", name));
                return;
            }
        };

        // copy info out under an immutable borrow
        let (is_const, is_moved, var_ty) = {
            let scope = &self.scopes[self.scopes.len() - 1 - scope_idx];
            let var = scope.vars.get(name).unwrap();
            (var.is_const, var.is_moved, var.ty.clone())
        };

        if is_const {
            self.errors.push(format!("cannot assign to constant '{}'", name));
        }
        if is_moved {
            self.errors.push(format!("cannot assign to moved variable '{}'", name));
        }
        
        // For union types, allow assignment of any type that matches a union variant
        let type_matches = match &var_ty {
            Ty::Union(union_tys) => union_tys.iter().any(|ut| value_ty.types_eq(ut)),
            _ => var_ty.types_eq(value_ty),
        };
        
        if !type_matches && *value_ty != Ty::Error {
            self.errors.push(format!(
                "type mismatch: cannot assign {} to variable '{}' of type {}",
                value_ty.display(),
                name,
                var_ty.display()
            ));
        }

        // mark target as initialized and not moved (it's being reassigned)
        {
            let scope_index = self.scopes.len() - 1 - scope_idx;
            let scope = &mut self.scopes[scope_index];
            let var = scope.vars.get_mut(name).unwrap();
            var.is_initialized = true;
            var.is_moved = false;
        }

        // if the value expression reads an owner variable as an ident, that's a move
        if value_ty.is_owner()
            && let Expr::Ident(ref src_name) = *value_expr
        {
            self.mark_move_from(src_name, value_expr);
        }
    }

    fn mark_move_from(&mut self, src_name: &str, _expr: &Expr) {
        // search scopes from innermost outward
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(src_name) {
                if var.is_owner {
                    var.is_moved = true;
                }
                return;
            }
        }
    }

    fn check_scope_exit_owners(&mut self) {
        let scope = self.current_scope();
        for (name, var) in &scope.vars {
            if var.is_owner && var.is_initialized && !var.is_moved {
                // owner is being dropped at scope exit: that's the cleanup.
                // We just track it; actual drop codegen comes later.
                let _ = (name, var);
            }
        }
    }

    // scope snapshot / merge helpers for control flow

    fn pop_scope_and_replace(&mut self, replacement: Scope) {
        self.scopes.pop();
        self.scopes.push(replacement);
    }

    fn replace_current_scope(&mut self, replacement: Scope) {
        self.scopes.pop();
        self.scopes.push(replacement);
    }

    fn merge_scopes(&self, after_a: &Scope, after_b: &Scope, before: &Scope) -> Scope {
        // Conservative merge: a variable is considered moved if moved in either branch.
        // Variables not present in one branch (declared inside) are dropped.
        let mut merged = Scope::new();
        for (name, var_before) in &before.vars {
            let var_a = after_a.vars.get(name);
            let var_b = after_b.vars.get(name);
            match (var_a, var_b) {
                (Some(va), Some(vb)) => {
                    let is_moved = va.is_moved || vb.is_moved;
                    merged.insert(
                        name.clone(),
                        VarState {
                            _name: name.clone(),
                            ty: var_before.ty.clone(),
                            is_const: var_before.is_const,
                            is_owner: var_before.is_owner,
                            is_initialized: va.is_initialized && vb.is_initialized,
                            is_moved,
                        },
                    );
                }
                (None, None) => {
                    // unchanged in both
                    merged.insert(name.clone(), var_before.clone());
                }
                (_, None) | (None, _) => {
                    // only modified in one branch: conservative - keep before state
                    merged.insert(name.clone(), var_before.clone());
                }
            }
        }
        merged
    }

    fn extract_qualified_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Ident(name) => {
                // Check if this identifier is an imported module alias
                if let Some(full_path) = self.imported_modules.get(name) {
                    Some(full_path.clone())
                } else {
                    Some(name.clone())
                }
            }
            Expr::Member { object, member } => {
                if let Some(prefix) = self.extract_qualified_name(object) {
                    Some(format!("{}.{}", prefix, member))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn call_function(&mut self, func_name: &str, args: &[Expr]) -> Ty {
        if let Some(sig) = self.functions.get(func_name).cloned() {
            if args.len() != sig.params.len() {
                self.errors.push(format!(
                    "call to '{}' expects {} arguments, got {}",
                    func_name,
                    sig.params.len(),
                    args.len()
                ));
                return Ty::Error;
            }
            for (arg, (param_name, param_ty, is_own)) in
                args.iter().zip(sig.params.iter())
            {
                let arg_ty = self.type_of_expr(arg);
                if !arg_ty.types_eq(param_ty) && arg_ty != Ty::Error {
                    self.errors.push(format!(
                        "argument '{}' type mismatch: expected {}, got {}",
                        param_name,
                        param_ty.display(),
                        arg_ty.display()
                    ));
                }
                if *is_own
                    && let Expr::Ident(ref name) = *arg
                {
                    self.mark_move_from(name, arg);
                }
            }
            sig.return_ty.clone()
        } else {
            self.errors.push(format!("undeclared function '{}'", func_name));
            Ty::Error
        }
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn analyze(program: &Program) -> SemanticResult {
    SemanticAnalyzer::new().analyze(program)
}
