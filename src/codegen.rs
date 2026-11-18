use crate::ast::*;
use crate::parser::parse_source;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

/// LLVM IR text emitter. Produces human-readable LLVM IR that can be piped
/// to `llc` (or `clang`) when an LLVM toolchain is available.
pub struct Codegen {
    struct_defs: Vec<String>,
    struct_order: Vec<String>,
    enum_defs: Vec<String>,
    global_defs: Vec<String>,
    string_constants: HashMap<String, String>,
    functions_output: String,
    reg_counter: usize,
    label_counter: usize,
    var_alloca: HashMap<String, String>,
    var_ty: HashMap<String, String>,
    structs: HashMap<String, StructDef>,
    enums: HashMap<String, EnumCodegenDef>,
    functions: HashMap<String, FnSig>,
    current_fn: Option<CurrentFn>,
    is_terminated: bool,
    indent: usize,
    imported_modules: HashMap<String, String>, // short_name -> full_path
    module_stack: Vec<String>,
    // Trait system
    traits: HashMap<String, TraitCodegenDef>,
    impls: Vec<ImplCodegenDef>,
    vtables: Vec<String>,
}

#[derive(Clone)]
struct TraitCodegenDef {
    name: String,
    methods: Vec<TraitMethodCodegenSig>,
}

#[derive(Clone)]
struct TraitMethodCodegenSig {
    name: String,
    params: Vec<(String, String)>,
    ret_ty: String,
}

#[derive(Clone)]
struct ImplCodegenDef {
    trait_name: String,
    for_ty: String,
    methods: HashMap<String, FnSig>,
}

#[derive(Clone)]
struct EnumCodegenDef {
    name: String,
    variants: Vec<(String, Option<String>)>,
    payload_ty: Option<String>,
}

#[derive(Clone)]
struct StructDef {
    name: String,
    fields: Vec<(String, String)>,
}

#[derive(Clone)]
#[allow(dead_code)]
struct FnSig {
    name: String,
    params: Vec<(String, String)>,
    ret_ty: String,
}

#[allow(dead_code)]
struct CurrentFn {
    name: String,
    ret_ty: String,
    params: Vec<(String, String)>,
    is_union_ret: bool,
}

#[derive(Debug)]
pub enum CodegenError {
    Msg(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::Msg(s) => write!(f, "{}", s),
        }
    }
}

impl From<fmt::Error> for CodegenError {
    fn from(_: fmt::Error) -> Self {
        CodegenError::Msg("fmt write error".to_string())
    }
}

type CodegenResult<T> = Result<T, CodegenError>;

impl Codegen {
    pub fn new() -> Self {
        let mut this = Self {
            struct_defs: Vec::new(),
            struct_order: Vec::new(),
            enum_defs: Vec::new(),
            global_defs: Vec::new(),
            string_constants: HashMap::new(),
            functions_output: String::new(),
            reg_counter: 0,
            label_counter: 0,
            var_alloca: HashMap::new(),
            var_ty: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            functions: HashMap::new(),
            current_fn: None,
            is_terminated: false,
            indent: 0,
            imported_modules: HashMap::new(),
            module_stack: Vec::new(),
            traits: HashMap::new(),
            impls: Vec::new(),
            vtables: Vec::new(),
        };
        
        // Built-in libc functions (always available)
        this.functions.insert("puts".into(), FnSig {
            name: "puts".into(),
            params: vec![("s".into(), "i8*".into())],
            ret_ty: "i32".into(),
        });
        this.functions.insert("putchar".into(), FnSig {
            name: "putchar".into(),
            params: vec![("c".into(), "i32".into())],
            ret_ty: "i32".into(),
        });
        
        this
    }

    fn load_machine_module(&mut self, path: &[String]) -> Result<ModuleDecl, String> {
        if path.is_empty() || path[0] != "MACHINE" {
            return Err("Not a MACHINE module path".into());
        }

        let module_key = path.join("::");
        if path.len() == 1 {
            return Ok(ModuleDecl {
                name: "MACHINE".into(),
                items: vec![],
                reexports: vec![],
            });
        } else if path.len() == 2 {
            let filename = match path[1].to_lowercase().as_str() {
                "io" => "io.fsl",
                "win32" => "win32.fsl",
                "io_handles" => "io_handles.fsl",
                "conout" => "conout.fsl",
                "conin" => "conin.fsl",
                "conerr" => "conerr.fsl",
                _ => return Err(format!("Unknown MACHINE module: {}", path[1])),
            };
            
            let file_path = Path::new("src/machine").join(filename);
            let source = fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read module file {}: {}", filename, e))?;
            
            let program = parse_source(&source)?;
            for item in &program.items {
                if let Item::ModuleDecl(m) = item {
                    if m.name.to_lowercase() == path[1].to_lowercase() {
                        return Ok(m.clone());
                    }
                }
            }
            Err(format!("Module '{}' not found in parsed file", path[1]))
        } else {
            Err(format!("Invalid MACHINE module path: {}", module_key))
        }
    }

    pub fn emit(mut self, program: &Program) -> Result<String, CodegenError> {
        for item in &program.items {
            match item {
                Item::StructDecl(s) => {
                    let fields: Vec<(String, String)> = s
                        .fields
                        .iter()
                        .map(|f| Ok((f.name.clone(), self.ty_to_llvm(&f.ty)?)))
                        .collect::<CodegenResult<Vec<_>>>()?;
                    self.struct_order.push(s.name.clone());
                    self.structs.insert(
                        s.name.clone(),
                        StructDef {
                            name: s.name.clone(),
                            fields,
                        },
                    );
                }
                Item::Function(f) => {
                    let params: Vec<(String, String)> = f
                        .params
                        .iter()
                        .map(|p| Ok((p.name.clone(), self.ty_to_llvm(&p.ty)?)))
                        .collect::<CodegenResult<Vec<_>>>()?;
                    let ret_ty = self.ty_to_llvm(&f.return_type)?;
                    self.functions.insert(
                        f.name.clone(),
                        FnSig {
                            name: f.name.clone(),
                            params,
                            ret_ty,
                        },
                    );
                }
                Item::ExternDecl(e) => {
                    let params: Vec<(String, String)> = e
                        .params
                        .iter()
                        .map(|p| Ok((p.name.clone(), self.ty_to_llvm(&p.ty)?)))
                        .collect::<CodegenResult<Vec<_>>>()?;
                    let ret_ty = self.ty_to_llvm(&e.return_type)?;
                    self.functions.insert(
                        e.name.clone(),
                        FnSig {
                            name: e.name.clone(),
                            params,
                            ret_ty,
                        },
                    );
                }
                Item::ImportDecl(imp) => {
                    // Handle built-in MACHINE::* imports by loading module files
                    if imp.path.len() >= 2 && imp.path[0] == "MACHINE" {
                        let full_path = imp.path.join(".");
                        let short_name = imp.path.last().unwrap().clone();
                        self.imported_modules.insert(short_name, full_path);
                        
                        if let Ok(module) = self.load_machine_module(&imp.path) {
                            for item in &module.items {
                                self.emit_module_item(item)?;
                            }
                        }
                    }
                }
                Item::EnumDecl(e) => {
                    let mut variants = Vec::new();
                    let mut payload_ty = None;
                    for v in &e.variants {
                        let opt_llvm_ty = if let Some(ref pty) = v.payload {
                            let lty = self.ty_to_llvm(pty)?;
                            if payload_ty.is_none() {
                                payload_ty = Some(lty.clone());
                            }
                            Some(lty)
                        } else {
                            None
                        };
                        variants.push((v.name.clone(), opt_llvm_ty));
                    }
                    self.enums.insert(
                        e.name.clone(),
                        EnumCodegenDef {
                            name: e.name.clone(),
                            variants,
                            payload_ty,
                        },
                    );
                }
                Item::ModuleDecl(m) => {
                    for item in &m.items {
                        self.emit_module_item(item)?;
                    }
                }
                Item::ConstDecl(c) => {
                    let llvm_ty = self.ty_to_llvm(&c.ty)?;
                    let init_val = self.emit_expr_const(&c.init)?;
                    let gv_name = format!("@.const.{}", c.name);
                    self.global_defs.push(format!(
                        "{} = constant {} {}",
                        gv_name, llvm_ty, init_val
                    ));
                }
                Item::VarDecl(v) => {
                    let llvm_ty = self.ty_to_llvm(&v.ty)?;
                    if let Some(init_expr) = &v.init {
                        let init_val = self.emit_expr_const(init_expr)?;
                        let gv_name = format!("@.var.{}", v.name);
                        self.global_defs.push(format!(
                            "{} = global {} {}",
                            gv_name, llvm_ty, init_val
                        ));
                    } else {
                        let gv_name = format!("@.var.{}", v.name);
                        self.global_defs.push(format!(
                            "{} = global {} zeroinitializer",
                            gv_name, llvm_ty
                        ));
                    }
                }
                Item::MachineDecl(_) => {}
                Item::AsmBlock(asm) => {
                    self.emit_asm_block(asm)?;
                }
                Item::TraitDecl(t) => {
                    let methods: Vec<TraitMethodCodegenSig> = t.methods.iter().map(|m| {
                        let params: Vec<(String, String)> = m.params.iter().map(|p| {
                            (p.name.clone(), self.ty_to_llvm(&p.ty).unwrap())
                        }).collect();
                        let ret_ty = self.ty_to_llvm(&m.return_type).unwrap();
                        TraitMethodCodegenSig {
                            name: m.name.clone(),
                            params,
                            ret_ty,
                        }
                    }).collect();
                    self.traits.insert(t.name.clone(), TraitCodegenDef {
                        name: t.name.clone(),
                        methods,
                    });
                }
                Item::ImplDecl(i) => {
                    let mut method_map = HashMap::new();
                    for m in &i.methods {
                        let params: Vec<(String, String)> = m.params.iter().map(|p| {
                            (p.name.clone(), self.ty_to_llvm(&p.ty).unwrap())
                        }).collect();
                        let ret_ty = self.ty_to_llvm(&m.return_type).unwrap();
                        let generic_params: Vec<String> = m.generic_params.iter().map(|p| p.name.clone()).collect();
                        method_map.insert(m.name.clone(), FnSig {
                            name: m.name.clone(),
                            params,
                            ret_ty: self.ty_to_llvm(&m.return_type).unwrap(),
                        });
                    }
                    self.impls.push(ImplCodegenDef {
                        trait_name: i.trait_name.clone(),
                        for_ty: self.ty_to_llvm(&i.for_type).unwrap(),
                        methods: method_map,
                    });
                }
            }
        }

        // Struct definitions in stable declaration order
        for name in &self.struct_order {
            if let Some(def) = self.structs.get(name) {
                let fields: Vec<String> = def.fields.iter().map(|(_, t)| t.clone()).collect();
                self.struct_defs
                    .push(format!("%struct.{} = type {{ {} }}", def.name, fields.join(", ")));
            }
        }

        // Enum definitions
        for def in self.enums.values() {
            if let Some(ref pty) = def.payload_ty {
                self.enum_defs.push(format!("%enum.{} = type {{ i32, {} }}", def.name, pty));
            } else {
                self.enum_defs.push(format!("%enum.{} = type {{ i32 }}", def.name));
            }
        }

        // VTable generation for trait implementations
        for impl_def in &self.impls {
            if let Some(trait_def) = self.traits.get(&impl_def.trait_name) {
                let vtable_name = format!("%vtable.{}.{}", impl_def.trait_name.replace("::", "."), self.sanitize_name(&impl_def.for_ty));
                
                // Build vtable entries
                let mut vtable_entries = Vec::new();
                for method in &trait_def.methods {
                    let method_key = format!("{}.{}", impl_def.trait_name, method.name);
                    if let Some(fn_sig) = impl_def.methods.get(&method_key) {
                        // Build function pointer type
                        let param_strs: Vec<String> = fn_sig.params.iter().map(|(_, ty)| ty.clone()).collect();
                        let fn_ptr_ty = format!("{} ({})*", fn_sig.ret_ty, param_strs.join(", "));
                        vtable_entries.push(format!("{}* getelementptr inbounds ({}, ptr @{}, i32 0, i32 0)", 
                            fn_ptr_ty, fn_ptr_ty, fn_sig.name));
                    }
                }
                
                let vtable_ty = format!("{{ {} }}", vtable_entries.join(", "));
                let vtable_global = format!("{} = constant {} {{ {} }}", vtable_name, vtable_ty, vtable_entries.join(", "));
                self.vtables.push(vtable_global);
            }
        }

        // Emit extern function declarations
        for item in &program.items {
            if let Item::ExternDecl(e) = item {
                self.emit_extern_decl(e)?;
            }
        }

        // Emit built-in MACHINE::IO functions as extern declarations
        self.emit_machine_io_decls();

        // Emit CORE intrinsics (memory operations, etc.)
        self.emit_core_intrinsics();

        // Emit functions (string constants will be gathered into self.global_defs)
        for item in &program.items {
            if let Item::Function(f) = item {
                self.emit_function(f)?;
            }
        }

        // Assemble full output
        let mut full_output = String::new();
        for s in &self.struct_defs {
            full_output.push_str(s);
            full_output.push('\n');
        }
        for e in &self.enum_defs {
            full_output.push_str(e);
            full_output.push('\n');
        }
        if !self.struct_defs.is_empty() || !self.enum_defs.is_empty() {
            full_output.push('\n');
        }

        for g in &self.global_defs {
            full_output.push_str(g);
            full_output.push('\n');
        }
        if !self.global_defs.is_empty() {
            full_output.push('\n');
        }

        full_output.push_str(&self.functions_output);

        Ok(full_output)
    }

    fn emit_module_item(&mut self, item: &Item) -> CodegenResult<()> {
        match item {
            Item::StructDecl(s) => {
                let fields: Vec<(String, String)> = s
                    .fields
                    .iter()
                    .map(|f| Ok((f.name.clone(), self.ty_to_llvm(&f.ty)?)))
                    .collect::<CodegenResult<Vec<_>>>()?;
                self.struct_order.push(s.name.clone());
                self.structs.insert(
                    s.name.clone(),
                    StructDef {
                        name: s.name.clone(),
                        fields,
                    },
                );
            }
            Item::Function(f) => {
                let qualified_name = if !self.module_stack.is_empty() {
                    format!("{}.{}", self.module_stack.join("."), f.name)
                } else {
                    f.name.clone()
                };
                let params: Vec<(String, String)> = f
                    .params
                    .iter()
                    .map(|p| Ok((p.name.clone(), self.ty_to_llvm(&p.ty)?)))
                    .collect::<CodegenResult<Vec<_>>>()?;
                let ret_ty = self.ty_to_llvm(&f.return_type)?;
                self.functions.insert(
                    qualified_name.clone(),
                    FnSig {
                        name: qualified_name.clone(),
                        params,
                        ret_ty,
                    },
                );
                self.emit_function_with_name(f, &qualified_name)?;
            }
            Item::ExternDecl(e) => {
                self.emit_extern_decl(e)?;
            }
            Item::EnumDecl(e) => {
                let mut variants = Vec::new();
                let mut payload_ty = None;
                for v in &e.variants {
                    let opt_llvm_ty = if let Some(ref pty) = v.payload {
                        let lty = self.ty_to_llvm(pty)?;
                        if payload_ty.is_none() {
                            payload_ty = Some(lty.clone());
                        }
                        Some(lty)
                    } else {
                        None
                    };
                    variants.push((v.name.clone(), opt_llvm_ty));
                }
                self.enums.insert(
                    e.name.clone(),
                    EnumCodegenDef {
                        name: e.name.clone(),
                        variants,
                        payload_ty,
                    },
                );
            }
            Item::ModuleDecl(m) => {
                self.module_stack.push(m.name.clone());
                for item in &m.items {
                    self.emit_module_item(item)?;
                }
                self.module_stack.pop();
            }
            Item::MachineDecl(_) => {}
            Item::TraitDecl(_) => {}
            Item::ImplDecl(_) => {}
            Item::AsmBlock(asm) => {
                self.emit_asm_block(asm)?;
            }
            Item::ImportDecl(imp) => {
                if imp.path.len() >= 2 && imp.path[0] == "MACHINE" {
                    let full_path = imp.path.join(".");
                    let short_name = imp.path.last().unwrap().clone();
                    self.imported_modules.insert(short_name, full_path.clone());
                    
                    match self.load_machine_module(&imp.path) {
                        Ok(module) => {
                            let saved_stack = self.module_stack.clone();
                            self.module_stack = imp.path.clone();
                            for item in &module.items {
                                self.emit_module_item(item)?;
                            }
                            self.module_stack = saved_stack;
                        }
                        Err(e) => {
                            return Err(CodegenError::Msg(format!("Failed to load module {}: {}", imp.path.join("::"), e)));
                        }
                    }
                }
            }
            Item::ConstDecl(c) => {
                let llvm_ty = self.ty_to_llvm(&c.ty)?;
                let init_val = self.emit_expr_const(&c.init)?;
                let gv_name = format!("@.const.{}", c.name);
                self.global_defs.push(format!(
                    "{} = constant {} {}",
                    gv_name, llvm_ty, init_val
                ));
            }
            Item::VarDecl(VarDecl { name, ty, init, .. }) => {
                let llvm_ty = self.ty_to_llvm(ty)?;
                if let Some(init_expr) = init {
                    let init_val = self.emit_expr_const(init_expr)?;
                    let gv_name = format!("@.var.{}", name);
                    self.global_defs.push(format!(
                        "{} = global {} {}",
                        gv_name, llvm_ty, init_val
                    ));
                } else {
                    let gv_name = format!("@.var.{}", name);
                    self.global_defs.push(format!(
                        "{} = global {} zeroinitializer",
                        gv_name, llvm_ty
                    ));
                }
            }
        }
        Ok(())
    }

    fn add_machine_io_functions(&mut self) {
        self.add_win32_functions();
        self.add_conout_functions();
        self.add_conin_functions();
        self.add_conerr_functions();
        self.add_io_convenience_functions();
    }

    fn add_win32_functions(&mut self) {
        self.functions.insert("GetStdHandle".into(), FnSig {
            name: "GetStdHandle".into(),
            params: vec![("nStdHandle".into(), "i32".into())],
            ret_ty: "i8*".into(),
        });
        self.functions.insert("GetCurrentProcess".into(), FnSig {
            name: "GetCurrentProcess".into(),
            params: vec![],
            ret_ty: "i8*".into(),
        });
        self.functions.insert("WriteConsoleA".into(), FnSig {
            name: "WriteConsoleA".into(),
            params: vec![
                ("hConsoleOutput".into(), "i8*".into()),
                ("lpBuffer".into(), "i8*".into()),
                ("nNumberOfCharsToWrite".into(), "i32".into()),
                ("lpNumberOfCharsWritten".into(), "i32*".into()),
                ("lpReserved".into(), "i8*".into()),
            ],
            ret_ty: "i32".into(),
        });
        self.functions.insert("ReadConsoleA".into(), FnSig {
            name: "ReadConsoleA".into(),
            params: vec![
                ("hConsoleInput".into(), "i8*".into()),
                ("lpBuffer".into(), "i8*".into()),
                ("nNumberOfCharsToRead".into(), "i32".into()),
                ("lpNumberOfCharsRead".into(), "i32*".into()),
                ("pInputControl".into(), "i8*".into()),
            ],
            ret_ty: "i32".into(),
        });
        self.functions.insert("WriteFile".into(), FnSig {
            name: "WriteFile".into(),
            params: vec![
                ("hFile".into(), "i8*".into()),
                ("lpBuffer".into(), "i8*".into()),
                ("nNumberOfBytesToWrite".into(), "i32".into()),
                ("lpNumberOfBytesWritten".into(), "i32*".into()),
                ("lpOverlapped".into(), "i8*".into()),
            ],
            ret_ty: "i32".into(),
        });
        self.functions.insert("ReadFile".into(), FnSig {
            name: "ReadFile".into(),
            params: vec![
                ("hFile".into(), "i8*".into()),
                ("lpBuffer".into(), "i8*".into()),
                ("nNumberOfBytesToRead".into(), "i32".into()),
                ("lpNumberOfBytesRead".into(), "i32*".into()),
                ("lpOverlapped".into(), "i8*".into()),
            ],
            ret_ty: "i32".into(),
        });
        self.functions.insert("FlushFileBuffers".into(), FnSig {
            name: "FlushFileBuffers".into(),
            params: vec![("hFile".into(), "i8*".into())],
            ret_ty: "i32".into(),
        });
        self.functions.insert("ExitProcess".into(), FnSig {
            name: "ExitProcess".into(),
            params: vec![("uExitCode".into(), "i32".into())],
            ret_ty: "void".into(),
        });
        self.functions.insert("GetConsoleMode".into(), FnSig {
            name: "GetConsoleMode".into(),
            params: vec![
                ("hConsoleHandle".into(), "i8*".into()),
                ("lpMode".into(), "i32*".into()),
            ],
            ret_ty: "i32".into(),
        });
        self.functions.insert("SetConsoleMode".into(), FnSig {
            name: "SetConsoleMode".into(),
            params: vec![
                ("hConsoleHandle".into(), "i8*".into()),
                ("dwMode".into(), "i32".into()),
            ],
            ret_ty: "i32".into(),
        });
    }

    fn add_conout_functions(&mut self) {
        self.functions.insert("Conout.write".into(), FnSig {
            name: "Conout.write".into(),
            params: vec![
                ("buf".into(), "i8*".into()),
                ("len".into(), "i64".into()),
            ],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conout.write_str".into(), FnSig {
            name: "Conout.write_str".into(),
            params: vec![("s".into(), "ptr".into())],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conout.writeln".into(), FnSig {
            name: "Conout.writeln".into(),
            params: vec![
                ("buf".into(), "i8*".into()),
                ("len".into(), "i64".into()),
            ],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conout.writeln_str".into(), FnSig {
            name: "Conout.writeln_str".into(),
            params: vec![("s".into(), "ptr".into())],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conout.flush".into(), FnSig {
            name: "Conout.flush".into(),
            params: vec![],
            ret_ty: "void".into(),
        });
        self.functions.insert("Conout.handle".into(), FnSig {
            name: "Conout.handle".into(),
            params: vec![],
            ret_ty: "i8*".into(),
        });
    }

    fn add_conin_functions(&mut self) {
        self.functions.insert("Conin.read".into(), FnSig {
            name: "Conin.read".into(),
            params: vec![
                ("buf".into(), "i8*".into()),
                ("len".into(), "i64".into()),
            ],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conin.scanline".into(), FnSig {
            name: "Conin.scanline".into(),
            params: vec![
                ("buf".into(), "i8*".into()),
                ("max_len".into(), "i64".into()),
            ],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conin.read_char".into(), FnSig {
            name: "Conin.read_char".into(),
            params: vec![],
            ret_ty: "i32".into(),
        });
        self.functions.insert("Conin.read_str".into(), FnSig {
            name: "Conin.read_str".into(),
            params: vec![
                ("buf".into(), "i8*".into()),
                ("len".into(), "i64".into()),
            ],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conin.handle".into(), FnSig {
            name: "Conin.handle".into(),
            params: vec![],
            ret_ty: "i8*".into(),
        });
    }

    fn add_conerr_functions(&mut self) {
        self.functions.insert("Conerr.write".into(), FnSig {
            name: "Conerr.write".into(),
            params: vec![
                ("buf".into(), "i8*".into()),
                ("len".into(), "i64".into()),
            ],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conerr.write_str".into(), FnSig {
            name: "Conerr.write_str".into(),
            params: vec![("s".into(), "ptr".into())],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conerr.writeln".into(), FnSig {
            name: "Conerr.writeln".into(),
            params: vec![
                ("buf".into(), "i8*".into()),
                ("len".into(), "i64".into()),
            ],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conerr.writeln_str".into(), FnSig {
            name: "Conerr.writeln_str".into(),
            params: vec![("s".into(), "ptr".into())],
            ret_ty: "i64".into(),
        });
        self.functions.insert("Conerr.flush".into(), FnSig {
            name: "Conerr.flush".into(),
            params: vec![],
            ret_ty: "void".into(),
        });
        self.functions.insert("Conerr.handle".into(), FnSig {
            name: "Conerr.handle".into(),
            params: vec![],
            ret_ty: "i8*".into(),
        });
    }

    fn add_io_convenience_functions(&mut self) {
        self.functions.insert("print".into(), FnSig {
            name: "print".into(),
            params: vec![("s".into(), "ptr".into())],
            ret_ty: "i64".into(),
        });
        self.functions.insert("println".into(), FnSig {
            name: "println".into(),
            params: vec![("s".into(), "ptr".into())],
            ret_ty: "i64".into(),
        });
        self.functions.insert("eprint".into(), FnSig {
            name: "eprint".into(),
            params: vec![("s".into(), "ptr".into())],
            ret_ty: "i64".into(),
        });
        self.functions.insert("eprintln".into(), FnSig {
            name: "eprintln".into(),
            params: vec![("s".into(), "ptr".into())],
            ret_ty: "i64".into(),
        });
        self.functions.insert("read_line".into(), FnSig {
            name: "read_line".into(),
            params: vec![
                ("buf".into(), "i8*".into()),
                ("len".into(), "i64".into()),
            ],
            ret_ty: "i64".into(),
        });
        self.functions.insert("exit".into(), FnSig {
            name: "exit".into(),
            params: vec![("code".into(), "i32".into())],
            ret_ty: "void".into(),
        });
    }

    fn emit_machine_io_decls(&mut self) {
        self.global_defs.push("declare i8* @GetStdHandle(i32)".into());
        self.global_defs.push("declare i8* @GetCurrentProcess()".into());
        self.global_defs.push("declare i32 @WriteConsoleA(i8*, i8*, i32, i32*, i8*)".into());
        self.global_defs.push("declare i32 @ReadConsoleA(i8*, i8*, i32, i32*, i8*)".into());
        self.global_defs.push("declare i32 @WriteFile(i8*, i8*, i32, i32*, i8*)".into());
        self.global_defs.push("declare i32 @ReadFile(i8*, i8*, i32, i32*, i8*)".into());
        self.global_defs.push("declare i32 @FlushFileBuffers(i8*)".into());
        self.global_defs.push("declare void @ExitProcess(i32)".into());
        self.global_defs.push("declare i32 @GetConsoleMode(i8*, i32*)".into());
        self.global_defs.push("declare i32 @SetConsoleMode(i8*, i32)".into());
        
        // puts/putchar for backwards compatibility
        self.global_defs.push("declare i32 @puts(i8*)".into());
        self.global_defs.push("declare i32 @putchar(i32)".into());
    }

    fn emit_core_intrinsics(&mut self) {
        // LLVM memory intrinsics
        self.global_defs.push("declare i8* @llvm.memcpy.p0.p0.i64(i8*, i8*, i64, i1)".into());
        self.global_defs.push("declare i8* @llvm.memmove.p0.p0.i64(i8*, i8*, i64, i1)".into());
        self.global_defs.push("declare i8* @llvm.memset.p0.i64(i8*, i8, i64, i1)".into());
        self.global_defs.push("declare i32 @llvm.memcmp.p0.p0.i64(i8*, i8*, i64, i1)".into());
    }

    fn emit_extern_decl(&mut self, e: &ExternDecl) -> CodegenResult<()> {
        let mut param_strs = Vec::new();
        for p in &e.params {
            let pty = self.ty_to_llvm(&p.ty)?;
            param_strs.push(format!("{}", pty));
        }
        let ret_ty = self.ty_to_llvm(&e.return_type)?;
        self.global_defs.push(format!(
            "declare {} @{}({})",
            ret_ty,
            e.name,
            param_strs.join(", ")
        ));
        Ok(())
    }

    fn emit_function(&mut self, f: &Function) -> CodegenResult<()> {
        self.reg_counter = 0;
        self.label_counter = 0;
        self.is_terminated = false;
        self.var_alloca.clear();
        self.var_ty.clear();

        let mut param_strs = Vec::new();
        let mut param_types = Vec::new();
        for p in &f.params {
            let pty = self.ty_to_llvm(&p.ty)?;
            param_strs.push(format!("{} %{}", pty, p.name));
            param_types.push((p.name.clone(), pty));
        }
        let ret_ty = self.ty_to_llvm(&f.return_type)?;

        self.line(&format!(
            "define {} @{}({}) {{",
            ret_ty,
            f.name,
            param_strs.join(", ")
        ))?;
        self.line("entry:")?;
        self.indent = 1;

        let is_union_ret = matches!(f.return_type, Type::Union(_));
        self.current_fn = Some(CurrentFn {
            name: f.name.clone(),
            ret_ty: ret_ty.clone(),
            params: param_types.clone(),
            is_union_ret,
        });

        for (pname, pty) in &param_types {
            let alloca = self.new_reg();
            self.line(&format!("{} = alloca {}", alloca, pty))?;
            self.line(&format!("store {} %{}, ptr {}", pty, pname, alloca))?;
            self.var_alloca.insert(pname.clone(), alloca);
            self.var_ty.insert(pname.clone(), pty.clone());
        }

        self.emit_block(&f.body)?;

        if !self.is_terminated {
            if ret_ty == "void" {
                self.line("ret void")?;
            } else {
                self.line(&format!("ret {} zeroinitializer", ret_ty))?;
            }
            self.is_terminated = true;
        }

        self.indent = 0;
        self.line("}")?;
        self.line("")?;

        self.current_fn = None;
        Ok(())
    }

    fn emit_function_with_name(&mut self, f: &Function, qualified_name: &str) -> CodegenResult<()> {
        self.reg_counter = 0;
        self.label_counter = 0;
        self.is_terminated = false;
        self.var_alloca.clear();
        self.var_ty.clear();

        let mut param_strs = Vec::new();
        let mut param_types = Vec::new();
        for p in &f.params {
            let pty = self.ty_to_llvm(&p.ty)?;
            param_strs.push(format!("{} %{}", pty, p.name));
            param_types.push((p.name.clone(), pty));
        }
        let ret_ty = self.ty_to_llvm(&f.return_type)?;

        self.line(&format!(
            "define {} @{}({}) {{",
            ret_ty,
            qualified_name,
            param_strs.join(", ")
        ))?;
        self.line("entry:")?;
        self.indent = 1;

        let is_union_ret = matches!(f.return_type, Type::Union(_));
        self.current_fn = Some(CurrentFn {
            name: qualified_name.to_string(),
            ret_ty: ret_ty.clone(),
            params: param_types.clone(),
            is_union_ret,
        });

        for (pname, pty) in &param_types {
            let alloca = self.new_reg();
            self.line(&format!("{} = alloca {}", alloca, pty))?;
            self.line(&format!("store {} %{}, ptr {}", pty, pname, alloca))?;
            self.var_alloca.insert(pname.clone(), alloca);
            self.var_ty.insert(pname.clone(), pty.clone());
        }

        self.emit_block(&f.body)?;

        if !self.is_terminated {
            if ret_ty == "void" {
                self.line("ret void")?;
            } else {
                self.line(&format!("ret {} zeroinitializer", ret_ty))?;
            }
            self.is_terminated = true;
        }

        self.indent = 0;
        self.line("}")?;
        self.line("")?;

        self.current_fn = None;
        Ok(())
    }

    fn emit_block(&mut self, block: &Block) -> CodegenResult<()> {
        for stmt in &block.statements {
            if self.is_terminated {
                break;
            }
            self.emit_statement(stmt)?;
        }
        Ok(())
    }

    fn emit_statement(&mut self, stmt: &Statement) -> CodegenResult<()> {
        match stmt {
            Statement::VarDecl {
                name,
                ty,
                is_const,
                init,
            } => {
                let llvm_ty = self.ty_to_llvm(ty)?;
                let alloca = self.new_reg();
                self.line(&format!("{} = alloca {}", alloca, llvm_ty))?;
                self.var_alloca.insert(name.clone(), alloca.clone());
                self.var_ty.insert(name.clone(), llvm_ty.clone());

                if let Some(init_expr) = init {
                    let init_val = self.emit_expr(init_expr)?;
                    let (init_ty, init_val_str) = self.split_ty_val(&init_val)?;
                    if init_ty != llvm_ty {
                        return Err(CodegenError::Msg(format!(
                            "type mismatch initializing '{}': expected {}, got {}",
                            name, llvm_ty, init_ty
                        )));
                    }
                    self.line(&format!(
                        "store {} {}, ptr {}",
                        llvm_ty, init_val_str, alloca
                    ))?;
                }
                let _ = is_const;
            }
            Statement::Assignment { target, value } => {
                let val = self.emit_expr(value)?;
                self.emit_assign_target(target, &val)?;
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.emit_expr(condition)?;
                let (cond_ty, cond_val_str) = self.split_ty_val(&cond_val)?;
                if cond_ty != "i1" {
                    return Err(CodegenError::Msg(format!(
                        "if condition must be i1, got {}",
                        cond_ty
                    )));
                }
                let then_label = self.new_label("then");
                let end_label = self.new_label("endif");
                let else_label = if else_branch.is_some() {
                    self.new_label("else")
                } else {
                    end_label.clone()
                };

                self.line(&format!(
                    "br i1 {}, label %{}, label %{}",
                    cond_val_str, then_label, else_label
                ))?;

                // Then branch
                self.line(&format!("{}:", then_label))?;
                self.is_terminated = false;
                self.emit_block(then_branch)?;
                let then_terminated = self.is_terminated;
                if !then_terminated {
                    self.line(&format!("br label %{}", end_label))?;
                }

                // Else branch
                let else_terminated = if let Some(eb) = else_branch {
                    self.line(&format!("{}:", else_label))?;
                    self.is_terminated = false;
                    self.emit_block(eb)?;
                    let et = self.is_terminated;
                    if !et {
                        self.line(&format!("br label %{}", end_label))?;
                    }
                    et
                } else {
                    false
                };

                // Merge block
                if !then_terminated || !else_terminated {
                    self.line(&format!("{}:", end_label))?;
                    self.is_terminated = false;
                } else {
                    self.is_terminated = true;
                }
            }
            Statement::While { condition, body } => {
                let head_label = self.new_label("while.head");
                let body_label = self.new_label("while.body");
                let exit_label = self.new_label("while.exit");

                self.line(&format!("br label %{}", head_label))?;
                self.line(&format!("{}:", head_label))?;
                self.is_terminated = false;
                let cond_val = self.emit_expr(condition)?;
                let (cond_ty, cond_val_str) = self.split_ty_val(&cond_val)?;
                if cond_ty != "i1" {
                    return Err(CodegenError::Msg(format!(
                        "while condition must be i1, got {}",
                        cond_ty
                    )));
                }
                self.line(&format!(
                    "br i1 {}, label %{}, label %{}",
                    cond_val_str, body_label, exit_label
                ))?;
                self.line(&format!("{}:", body_label))?;
                self.is_terminated = false;
                self.emit_block(body)?;
                if !self.is_terminated {
                    self.line(&format!("br label %{}", head_label))?;
                }
                self.line(&format!("{}:", exit_label))?;
                self.is_terminated = false;
            }
            Statement::For {
                variable,
                start,
                end,
                body,
            } => {
                let head_label = self.new_label("for.head");
                let body_label = self.new_label("for.body");
                let exit_label = self.new_label("for.exit");

                let var_alloca = self.new_reg();
                self.line(&format!("{} = alloca i32", var_alloca))?;
                self.var_alloca.insert(variable.clone(), var_alloca.clone());
                self.var_ty.insert(variable.clone(), "i32".to_string());

                let start_val = self.emit_expr(start)?;
                let (start_ty, start_val_str) = self.split_ty_val(&start_val)?;
                if start_ty != "i32" {
                    return Err(CodegenError::Msg(format!(
                        "for start must be i32, got {}",
                        start_ty
                    )));
                }
                self.line(&format!(
                    "store i32 {}, ptr {}",
                    start_val_str, var_alloca
                ))?;
                self.line(&format!("br label %{}", head_label))?;

                self.line(&format!("{}:", head_label))?;
                self.is_terminated = false;
                let var_val = self.emit_expr(&Expr::Ident(variable.clone()))?;
                let (var_ty_s, var_val_str) = self.split_ty_val(&var_val)?;
                if var_ty_s != "i32" {
                    return Err(CodegenError::Msg(format!(
                        "loop variable must be i32, got {}",
                        var_ty_s
                    )));
                }
                let end_val = self.emit_expr(end)?;
                let (end_ty, end_val_str) = self.split_ty_val(&end_val)?;
                if end_ty != "i32" {
                    return Err(CodegenError::Msg(format!(
                        "for end must be i32, got {}",
                        end_ty
                    )));
                }
                let cmp = self.new_reg();
                self.line(&format!(
                    "{} = icmp sle i32 {}, {}",
                    cmp, var_val_str, end_val_str
                ))?;
                self.line(&format!(
                    "br i1 {}, label %{}, label %{}",
                    cmp, body_label, exit_label
                ))?;

                self.line(&format!("{}:", body_label))?;
                self.is_terminated = false;
                self.emit_block(body)?;

                if !self.is_terminated {
                    let var_val2 = self.emit_expr(&Expr::Ident(variable.clone()))?;
                    let (_, var_val2_str) = self.split_ty_val(&var_val2)?;
                    let inc = self.new_reg();
                    self.line(&format!("{} = add i32 {}, 1", inc, var_val2_str))?;
                    self.line(&format!("store i32 {}, ptr {}", inc, var_alloca))?;
                    self.line(&format!("br label %{}", head_label))?;
                }

                self.line(&format!("{}:", exit_label))?;
                self.is_terminated = false;
            }
            Statement::Return { values } => {
                let current_fn = self.current_fn.as_ref().unwrap();
                let ret_ty = current_fn.ret_ty.clone();
                let is_union_ret = current_fn.is_union_ret;
                if ret_ty == "void" {
                    if !values.is_empty() {
                        return Err(CodegenError::Msg(
                            "return with value in void function".to_string(),
                        ));
                    }
                    self.line("ret void")?;
                    self.is_terminated = true;
                } else if is_union_ret {
                    // Union type return: { i32, T1, T2, ... }
                    if values.len() != 1 {
                        return Err(CodegenError::Msg(format!(
                            "union return expects 1 value, got {}",
                            values.len()
                        )));
                    }
                    let val = self.emit_expr(&values[0])?;
                    let (val_ty, val_str) = self.split_ty_val(&val)?;
                    
                    // Find which union variant this value matches
                    let union_payload_tys: Vec<String> = ret_ty
                        .strip_prefix("{ i32, ")
                        .and_then(|s| s.strip_suffix(" }"))
                        .map(|s| s.split(", ").map(|s| s.trim().to_string()).collect())
                        .unwrap_or_default();
                    
                    let variant_idx = union_payload_tys.iter().position(|pty| pty == &val_ty);
                    let variant_idx = match variant_idx {
                        Some(idx) => idx as i64,
                        None => {
                            return Err(CodegenError::Msg(format!(
                                "return value type {} does not match any union variant: {}",
                                val_ty, union_payload_tys.join(", ")
                            )));
                        }
                    };
                    
                    // Construct union: { i32 tag, payload }
                    let tag_reg = self.new_reg();
                    self.line(&format!("{} = insertvalue {} zeroinitializer, i32 {}, 0", tag_reg, ret_ty, variant_idx))?;
                    
                    let payload_reg = self.new_reg();
                    self.line(&format!("{} = insertvalue {} {}, {} {}, 1", payload_reg, ret_ty, tag_reg, val_ty, val_str))?;
                    
                    self.line(&format!("ret {} {}", ret_ty, payload_reg))?;
                    self.is_terminated = true;
                } else if values.len() == 1 {
                    let val = self.emit_expr(&values[0])?;
                    let (val_ty, val_str) = self.split_ty_val(&val)?;
                    if val_ty != ret_ty {
                        return Err(CodegenError::Msg(format!(
                            "return type mismatch: expected {}, got {}",
                            ret_ty, val_ty
                        )));
                    }
                    self.line(&format!("ret {} {}", ret_ty, val_str))?;
                    self.is_terminated = true;
                } else {
                    let mut elems = Vec::new();
                    let mut elem_tys = Vec::new();
                    for v in values {
                        let ev = self.emit_expr(v)?;
                        let (et, ev2) = self.split_ty_val(&ev)?;
                        elem_tys.push(et);
                        elems.push(ev2);
                    }
                    let struct_ty = format!("{{ {} }}", elem_tys.join(", "));
                    if struct_ty != ret_ty {
                        return Err(CodegenError::Msg(format!(
                            "return type mismatch: expected {}, got {}",
                            ret_ty, struct_ty
                        )));
                    }
                    let first = self.new_reg();
                    self.line(&format!(
                        "{} = insertvalue {} zeroinitializer, {} {}, 0",
                        first, struct_ty, elem_tys[0], elems[0]
                    ))?;
                    let mut prev = first;
                    for (i, ev) in elems.iter().enumerate().skip(1) {
                        let new_val = self.new_reg();
                        self.line(&format!(
                            "{} = insertvalue {} {}, {} {}, {}",
                            new_val, struct_ty, prev, elem_tys[i], ev, i
                        ))?;
                        prev = new_val;
                    }
                    self.line(&format!("ret {} {}", struct_ty, prev))?;
                    self.is_terminated = true;
                }
            }
            Statement::Expr(expr) => {
                self.emit_expr(expr)?;
            }
            Statement::Match { expr, arms, else_branch } => {
                let val_str = self.emit_expr(expr)?;
                let (val_ty, val_reg) = self.split_ty_val(&val_str)?;

                let is_enum = val_ty.starts_with("%enum.");
                let (discrim_ty, discrim_reg) = if is_enum {
                    let tag_reg = self.new_reg();
                    self.line(&format!("{} = extractvalue {} {}, 0", tag_reg, val_ty, val_reg))?;
                    ("i32".to_string(), tag_reg)
                } else {
                    (val_ty.clone(), val_reg.clone())
                };

                let mut case_labels = Vec::new();
                for i in 0..arms.len() {
                    case_labels.push(self.new_label(&format!("match.case.{}", i)));
                }
                let else_label = self.new_label("match.else");
                let end_label = self.new_label("match.end");

                let wildcard_idx = arms.iter().position(|arm| matches!(arm.pattern, Pattern::Wildcard));
                let default_target = if let Some(w_idx) = wildcard_idx {
                    case_labels[w_idx].clone()
                } else if else_branch.is_some() {
                    else_label.clone()
                } else {
                    end_label.clone()
                };

                self.line(&format!("switch {} {}, label %{} [", discrim_ty, discrim_reg, default_target))?;
                for (i, arm) in arms.iter().enumerate() {
                    if Some(i) == wildcard_idx {
                        continue;
                    }
                    let match_val: i64 = match &arm.pattern {
                        Pattern::Literal(Literal::Int(n)) => *n,
                        Pattern::Literal(Literal::Bool(b)) => if *b { 1 } else { 0 },
                        Pattern::Variant { enum_name, variant_name }
                        | Pattern::VariantWithPayload { enum_name, variant_name, .. } => {
                            let edef = if let Some(en) = enum_name {
                                self.enums.get(en)
                            } else {
                                let en_from_ty = val_ty.strip_prefix("%enum.").unwrap_or("");
                                self.enums.get(en_from_ty)
                            };
                            if let Some(edef) = edef {
                                let tag_opt = edef.variants.iter().position(|(vn, _)| vn == variant_name);
                                if let Some(tag) = tag_opt {
                                    tag as i64
                                } else {
                                    return Err(CodegenError::Msg(format!("unknown variant '{}'", variant_name)));
                                }
                            } else {
                                return Err(CodegenError::Msg(format!("cannot resolve enum for variant '{}'", variant_name)));
                            }
                        }
                        Pattern::Wildcard => unreachable!(),
                        _ => return Err(CodegenError::Msg("unsupported pattern in match switch".to_string())),
                    };
                    self.line(&format!("    {} {}, label %{}", discrim_ty, match_val, case_labels[i]))?;
                }
                self.line("]")?;
                self.is_terminated = true;

                let mut any_unterminated = false;

                for (i, arm) in arms.iter().enumerate() {
                    self.line(&format!("{}:", case_labels[i]))?;
                    self.is_terminated = false;

                    let prev_var = if let Pattern::VariantWithPayload { binding, .. } = &arm.pattern {
                        let en_from_ty = val_ty.strip_prefix("%enum.").unwrap_or("");
                        let pty = {
                            let edef = self.enums.get(en_from_ty).ok_or_else(|| {
                                CodegenError::Msg(format!("unknown enum '{}'", en_from_ty))
                            })?;
                            edef.payload_ty.clone().ok_or_else(|| {
                                CodegenError::Msg(format!("enum '{}' has no payload", en_from_ty))
                            })?
                        };

                        let payload_reg = self.new_reg();
                        self.line(&format!("{} = extractvalue {} {}, 1", payload_reg, val_ty, val_reg))?;
                        let alloca = self.new_reg();
                        self.line(&format!("{} = alloca {}", alloca, pty))?;
                        self.line(&format!("store {} {}, ptr {}", pty, payload_reg, alloca))?;

                        let prev_a = self.var_alloca.insert(binding.clone(), alloca);
                        let prev_t = self.var_ty.insert(binding.clone(), pty.clone());
                        Some((binding.clone(), prev_a, prev_t))
                    } else {
                        None
                    };

                    self.emit_block(&arm.body)?;

                    if let Some((b, prev_a, prev_t)) = prev_var {
                        self.var_alloca.remove(&b);
                        self.var_ty.remove(&b);
                        if let Some(pa) = prev_a {
                            self.var_alloca.insert(b.clone(), pa);
                        }
                        if let Some(pt) = prev_t {
                            self.var_ty.insert(b, pt);
                        }
                    }

                    if !self.is_terminated {
                        self.line(&format!("br label %{}", end_label))?;
                        any_unterminated = true;
                    }
                }

                if wildcard_idx.is_none() && else_branch.is_some() {
                    self.line(&format!("{}:", else_label))?;
                    self.is_terminated = false;
                    if let Some(eb) = else_branch {
                        self.emit_block(eb)?;
                    }
                    if !self.is_terminated {
                        self.line(&format!("br label %{}", end_label))?;
                        any_unterminated = true;
                    }
                } else if wildcard_idx.is_none() && default_target == end_label {
                    any_unterminated = true;
                }

                if any_unterminated {
                    self.line(&format!("{}:", end_label))?;
                    self.is_terminated = false;
                } else {
                    self.is_terminated = true;
                }
            }
            Statement::AsmBlock(asm) => {
                self.emit_asm_block(asm)?;
            }
            Statement::Unreachable => {
                self.line("unreachable")?;
                self.is_terminated = true;
            }
        }
        Ok(())
    }

fn emit_asm_block(&mut self, asm: &AsmBlock) -> CodegenResult<()> {
        // Combine all instructions into a single template with newlines
        // Replace % placeholders with $0, $1, $2... based on operand order
        let mut combined_template = String::new();
        let mut all_operands = Vec::new();
        
        for instr in &asm.instructions {
            // Replace % placeholders with $0, $1, $2... for this instruction
            let mut instr_template = String::new();
            let operand_base = all_operands.len();
            let mut operand_idx = 0;
            for ch in instr.template.chars() {
                if ch == '%' {
                    instr_template.push('$');
                    instr_template.push_str(&(operand_base + operand_idx).to_string());
                    operand_idx += 1;
                } else {
                    instr_template.push(ch);
                }
            }
            
            if !combined_template.is_empty() {
                combined_template.push_str("\\\\0A");
            }
            combined_template.push_str(&instr_template);
            all_operands.extend(instr.operands.iter().cloned());
        }
        
        let template = combined_template;
        
        // Collect operands
        let mut constraint_strs = Vec::new();
        let mut operand_vals = Vec::new();
        
        for operand in &all_operands {
            let val = self.emit_expr(&operand.expr)?;
            let (ty, val_str) = self.split_ty_val(&val)?;
            
            let constraint = if operand.is_output {
                if operand.is_input {
                    "+r" // read-write
                } else {
                    "=r" // write-only
                }
            } else if operand.is_input {
                "r" // read-only
            } else {
                "r" // default to read-only when direction unknown
            };
            
            // For x86_64, use specific register constraints for better compatibility
            // This is a temporary workaround - ideally we'd infer from the template
            
            constraint_strs.push(constraint.to_string());
            operand_vals.push(format!("{} {}", ty, val_str));
        }
        
        // Build clobber constraints
        let mut clobber_constraints = Vec::new();
        for clobber in &asm.clobbers {
            clobber_constraints.push(format!("~{{{}}}", clobber));
        }
        // Add standard x86_64 clobbers that clang adds automatically
        clobber_constraints.push("~{dirflag}".to_string());
        clobber_constraints.push("~{fpsr}".to_string());
        clobber_constraints.push("~{flags}".to_string());
        
        // Combine all constraints: outputs, inputs, clobbers
        let mut all_constraints: Vec<String> = constraint_strs.into_iter().map(String::from).collect();
        all_constraints.extend(clobber_constraints);
        let constraint_str = all_constraints.join(",");
        
        // Always use Intel syntax (RELIC default)
        let constraint_str = format!("{}#intel", constraint_str);
        
        let operand_str = operand_vals.join(", ");
        
        // Build the full asm call (always sideeffect for inline asm)
        if constraint_str.is_empty() {
            // No operands - still need empty constraint string
            self.line(&format!(
                "call void asm sideeffect \"{}\", \"\"()",
                template
            ))?;
        } else {
            self.line(&format!(
                "call void asm sideeffect \"{}\", \"{}\"({})",
                template,
                constraint_str,
                operand_str
            ))?;
        }
Ok(())
    }

    fn emit_assign_target(
        &mut self,
        target: &AssignmentTarget,
        val: &str,
    ) -> CodegenResult<()> {
        let (val_ty, val_str) = self.split_ty_val(val)?;
        match target {
            AssignmentTarget::Ident(name) => {
                if let Some(alloca) = self.var_alloca.get(name).cloned() {
                    let ty = self
                        .var_ty
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| "i32".to_string());
                    if ty != val_ty {
                        return Err(CodegenError::Msg(format!(
                            "type mismatch assigning to '{}': expected {}, got {}",
                            name, ty, val_ty
                        )));
                    }
                    self.line(&format!("store {} {}, ptr {}", ty, val_str, alloca))?;
                } else {
                    return Err(CodegenError::Msg(format!(
                        "undeclared variable '{}'",
                        name
                    )));
                }
            }
            AssignmentTarget::Member { object, member } => {
                let obj_ptr = self.emit_expr(object)?;
                let (obj_ty, obj_ptr_str) = self.split_ty_val(&obj_ptr)?;
                let struct_ty = self.deref_elem_ty(&obj_ty)?;
                let field_idx = self.field_index_from_ty(&struct_ty, member)?;
                let elem_ty = self.field_ty_from_ty(&struct_ty, member)?;
                let ptr = self.new_reg();
                self.line(&format!(
                    "{} = getelementptr {}, ptr {}, i32 0, i32 {}",
                    ptr, struct_ty, obj_ptr_str, field_idx
                ))?;
                self.line(&format!(
                    "store {} {}, ptr {}",
                    elem_ty, val_str, ptr
                ))?;
            }
            AssignmentTarget::Index { object, index } => {
                let (obj_ty, obj_ptr_str) = if let Expr::Ident(name) = object.as_ref() {
                    if let Some(ty) = self.var_ty.get(name) {
                        if ty.starts_with('[') {
                            (format!("{}*", ty), self.var_alloca.get(name).unwrap().clone())
                        } else {
                            let obj_ptr = self.emit_expr(object)?;
                            self.split_ty_val(&obj_ptr)?
                        }
                    } else {
                        let obj_ptr = self.emit_expr(object)?;
                        self.split_ty_val(&obj_ptr)?
                    }
                } else {
                    let obj_ptr = self.emit_expr(object)?;
                    self.split_ty_val(&obj_ptr)?
                };
                let idx = self.emit_expr(index)?;
                let (idx_ty, idx_str) = self.split_ty_val(&idx)?;
                if idx_ty != "i32" && idx_ty != "i64" {
                    return Err(CodegenError::Msg(format!(
                        "array index must be integer, got {}",
                        idx_ty
                    )));
                }
                let elem_ty = self.index_elem_ty_from_ty(&obj_ty)?;
                let ptr = self.new_reg();
                if let Some(arr_ty) = obj_ty.strip_suffix('*').filter(|s| s.starts_with('[')) {
                    self.line(&format!(
                        "{} = getelementptr {}, ptr {}, i32 0, i32 {}",
                        ptr, arr_ty, obj_ptr_str, idx_str
                    ))?;
                } else {
                    self.line(&format!(
                        "{} = getelementptr {}, ptr {}, i32 {}",
                        ptr, elem_ty, obj_ptr_str, idx_str
                    ))?;
                }
                self.line(&format!(
                    "store {} {}, ptr {}",
                    elem_ty, val_str, ptr
                ))?;
            }
            AssignmentTarget::VolatileDeref { object } => {
                let obj_ptr = self.emit_expr(object)?;
                let (obj_ty, obj_ptr_str) = self.split_ty_val(&obj_ptr)?;
                let elem_ty = self.deref_elem_ty(&obj_ty)?;
                self.line(&format!(
                    "store volatile {} {}, ptr {}",
                    elem_ty, val_str, obj_ptr_str
                ))?;
            }
            AssignmentTarget::Tuple(targets) => {
                let (val_ty, val_str) = self.split_ty_val(val)?;
                let elem_tys = self.struct_elem_tys(&val_ty)?;
                if elem_tys.len() != targets.len() {
                    return Err(CodegenError::Msg(format!(
                        "tuple assignment arity mismatch: expected {} targets, got {} elements",
                        targets.len(),
                        elem_tys.len()
                    )));
                }
                for (i, target) in targets.iter().enumerate() {
                    let elem = self.new_reg();
                    self.line(&format!(
                        "{} = extractvalue {} {}, {}",
                        elem, val_ty, val_str, i
                    ))?;
                    let elem_ty = &elem_tys[i];
                    match target {
                        AssignmentTarget::Ident(name) => {
                            if let Some(alloca) = self.var_alloca.get(name).cloned() {
                                let ty = self
                                    .var_ty
                                    .get(name)
                                    .cloned()
                                    .unwrap_or_else(|| "i32".to_string());
                                if ty != *elem_ty {
                                    return Err(CodegenError::Msg(format!(
                                        "type mismatch assigning to '{}': expected {}, got {}",
                                        name, ty, elem_ty
                                    )));
                                }
                                self.line(&format!(
                                    "store {} {}, ptr {}",
                                    ty, elem, alloca
                                ))?;
                            } else {
                                return Err(CodegenError::Msg(format!(
                                    "undeclared variable '{}'",
                                    name
                                )));
                            }
                        }
                        _ => {
                            return Err(CodegenError::Msg(
                                "nested tuple assignment target not supported".to_string(),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> CodegenResult<String> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(v) => Ok(format!("i32 {}", v)),
                Literal::Float(v) => Ok(format!("double {}", v)),
                Literal::String(s) => {
                    let (gv, len) = self.get_or_emit_string_const(s);
                    let ptr = self.new_reg();
                    self.line(&format!(
                        "{} = getelementptr inbounds [{} x i8], ptr {}, i32 0, i32 0",
                        ptr, len, gv
                    ))?;
                    Ok(format!("i8* {}", ptr))
                }
                Literal::Char(c) => Ok(format!("i8 {}", *c as i32)),
                Literal::Bool(b) => Ok(format!("i1 {}", if *b { 1 } else { 0 })),
                Literal::Null => Ok("i8* null".to_string()),
            },
            Expr::Ident(name) => {
                if let Some(alloca) = self.var_alloca.get(name).cloned() {
                    let ty = self
                        .var_ty
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| "i32".to_string());
                    let loaded = self.new_reg();
                    self.line(&format!("{} = load {}, ptr {}", loaded, ty, alloca))?;
                    Ok(format!("{} {}", ty, loaded))
                } else {
                    let mut found_variant = None;
                    for (en, edef) in &self.enums {
                        if let Some((idx, (_, opt_pty))) = edef.variants.iter().enumerate().find(|(_, (vn, _))| vn == name) {
                            found_variant = Some((en.clone(), idx, opt_pty.clone()));
                            break;
                        }
                    }
                    if let Some((en, idx, opt_pty)) = found_variant {
                        if opt_pty.is_none() {
                            let enum_ty = format!("%enum.{}", en);
                            let r = self.new_reg();
                            self.line(&format!("{} = insertvalue {} zeroinitializer, i32 {}, 0", r, enum_ty, idx))?;
                            Ok(format!("{} {}", enum_ty, r))
                        } else {
                            Err(CodegenError::Msg(format!("variant '{}' requires payload arguments", name)))
                        }
                    } else {
                        Err(CodegenError::Msg(format!(
                            "undeclared variable '{}'",
                            name
                        )))
                    }
                }
            }
            Expr::Binary { left, op, right } => {
                let l = self.emit_expr(left)?;
                let r = self.emit_expr(right)?;
                let (lt, lv) = self.split_ty_val(&l)?;
                let (rt, rv) = self.split_ty_val(&r)?;
                if lt != rt {
                    return Err(CodegenError::Msg(format!(
                        "type mismatch in binary op: {} vs {}",
                        lt, rt
                    )));
                }
                let mut res = self.new_reg();
                match op {
                    BinaryOp::Add => {
                        self.line(&format!("{} = add {} {}, {}", res, lt, lv, rv))?;
                        Ok(format!("{} {}", lt, res))
                    }
                    BinaryOp::Sub => {
                        self.line(&format!("{} = sub {} {}, {}", res, lt, lv, rv))?;
                        Ok(format!("{} {}", lt, res))
                    }
                    BinaryOp::Mul => {
                        self.line(&format!("{} = mul {} {}, {}", res, lt, lv, rv))?;
                        Ok(format!("{} {}", lt, res))
                    }
                    BinaryOp::Div => {
                        if lt.starts_with('i') {
                            self.line(&format!(
                                "{} = sdiv {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else if lt == "double" || lt == "float" {
                            self.line(&format!(
                                "{} = fdiv {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else {
                            return Err(CodegenError::Msg(format!(
                                "div not supported for type {}",
                                lt
                            )));
                        }
                        Ok(format!("{} {}", lt, res))
                    }
                    BinaryOp::Mod => {
                        if lt.starts_with('i') {
                            self.line(&format!(
                                "{} = srem {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else {
                            return Err(CodegenError::Msg(format!(
                                "mod not supported for type {}",
                                lt
                            )));
                        }
                        Ok(format!("{} {}", lt, res))
                    }
                    BinaryOp::Eq => {
                        if lt.starts_with('i') {
                            self.line(&format!(
                                "{} = icmp eq {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else if lt == "double" || lt == "float" {
                            self.line(&format!(
                                "{} = fcmp oeq {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else if lt.starts_with("%enum.") {
                            let tag_a = res;
                            self.line(&format!("{} = extractvalue {} {}, 0", tag_a, lt, lv))?;
                            let tag_b = self.new_reg();
                            self.line(&format!("{} = extractvalue {} {}, 0", tag_b, lt, rv))?;
                            let cmp_res = self.new_reg();
                            self.line(&format!("{} = icmp eq i32 {}, {}", cmp_res, tag_a, tag_b))?;
                            res = cmp_res;
                        } else {
                            return Err(CodegenError::Msg(format!(
                                "eq not supported for type {}",
                                lt
                            )));
                        }
                        Ok(format!("i1 {}", res))
                    }
                    BinaryOp::Neq => {
                        if lt.starts_with('i') {
                            self.line(&format!(
                                "{} = icmp ne {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else if lt == "double" || lt == "float" {
                            self.line(&format!(
                                "{} = fcmp one {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else if lt.starts_with("%enum.") {
                            let tag_a = res;
                            self.line(&format!("{} = extractvalue {} {}, 0", tag_a, lt, lv))?;
                            let tag_b = self.new_reg();
                            self.line(&format!("{} = extractvalue {} {}, 0", tag_b, lt, rv))?;
                            let cmp_res = self.new_reg();
                            self.line(&format!("{} = icmp ne i32 {}, {}", cmp_res, tag_a, tag_b))?;
                            res = cmp_res;
                        } else {
                            return Err(CodegenError::Msg(format!(
                                "ne not supported for type {}",
                                lt
                            )));
                        }
                        Ok(format!("i1 {}", res))
                    }
                    BinaryOp::Lt => {
                        if lt.starts_with('i') {
                            self.line(&format!(
                                "{} = icmp slt {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else if lt == "double" || lt == "float" {
                            self.line(&format!(
                                "{} = fcmp olt {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else {
                            return Err(CodegenError::Msg(format!(
                                "lt not supported for type {}",
                                lt
                            )));
                        }
                        Ok(format!("i1 {}", res))
                    }
                    BinaryOp::Gt => {
                        if lt.starts_with('i') {
                            self.line(&format!(
                                "{} = icmp sgt {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else if lt == "double" || lt == "float" {
                            self.line(&format!(
                                "{} = fcmp ogt {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else {
                            return Err(CodegenError::Msg(format!(
                                "gt not supported for type {}",
                                lt
                            )));
                        }
                        Ok(format!("i1 {}", res))
                    }
                    BinaryOp::Le => {
                        if lt.starts_with('i') {
                            self.line(&format!(
                                "{} = icmp sle {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else if lt == "double" || lt == "float" {
                            self.line(&format!(
                                "{} = fcmp ole {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else {
                            return Err(CodegenError::Msg(format!(
                                "le not supported for type {}",
                                lt
                            )));
                        }
                        Ok(format!("i1 {}", res))
                    }
                    BinaryOp::Ge => {
                        if lt.starts_with('i') {
                            self.line(&format!(
                                "{} = icmp sge {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else if lt == "double" || lt == "float" {
                            self.line(&format!(
                                "{} = fcmp oge {} {}, {}",
                                res, lt, lv, rv
                            ))?;
                        } else {
                            return Err(CodegenError::Msg(format!(
                                "ge not supported for type {}",
                                lt
                            )));
                        }
                        Ok(format!("i1 {}", res))
                    }
                    BinaryOp::And | BinaryOp::Amp => {
                        if lt.starts_with('i') {
                            self.line(&format!("{} = and {} {}, {}", res, lt, lv, rv))?;
                            Ok(format!("{} {}", lt, res))
                        } else {
                            Err(CodegenError::Msg(format!(
                                "and not supported for type {}",
                                lt
                            )))
                        }
                    }
                    BinaryOp::Or | BinaryOp::Pipe => {
                        if lt.starts_with('i') {
                            self.line(&format!("{} = or {} {}, {}", res, lt, lv, rv))?;
                            Ok(format!("{} {}", lt, res))
                        } else {
                            Err(CodegenError::Msg(format!(
                                "or not supported for type {}",
                                lt
                            )))
                        }
                    }
                    BinaryOp::Xor => {
                        if lt.starts_with('i') {
                            self.line(&format!("{} = xor {} {}, {}", res, lt, lv, rv))?;
                            Ok(format!("{} {}", lt, res))
                        } else {
                            Err(CodegenError::Msg(format!(
                                "xor not supported for type {}",
                                lt
                            )))
                        }
                    }
                    BinaryOp::LShift => {
                        if lt.starts_with('i') {
                            self.line(&format!("{} = shl {} {}, {}", res, lt, lv, rv))?;
                            Ok(format!("{} {}", lt, res))
                        } else {
                            Err(CodegenError::Msg(format!(
                                "shl not supported for type {}",
                                lt
                            )))
                        }
                    }
                    BinaryOp::RShift => {
                        if lt.starts_with('i') {
                            self.line(&format!("{} = lshr {} {}, {}", res, lt, lv, rv))?;
                            Ok(format!("{} {}", lt, res))
                        } else {
                            Err(CodegenError::Msg(format!(
                                "lshr not supported for type {}",
                                lt
                            )))
                        }
                    }
                }
            }
            Expr::Unary { op, operand } => {
                let o = self.emit_expr(operand)?;
                let (ot, ov) = self.split_ty_val(&o)?;
                let res = self.new_reg();
                match op {
                    UnaryOp::Neg => {
                        if ot.starts_with('i') {
                            self.line(&format!("{} = sub {} 0, {}", res, ot, ov))?;
                            Ok(format!("{} {}", ot, res))
                        } else if ot == "double" || ot == "float" {
                            self.line(&format!(
                                "{} = fsub {} 0.0, {}",
                                res, ot, ov
                            ))?;
                            Ok(format!("{} {}", ot, res))
                        } else {
                            Err(CodegenError::Msg(format!(
                                "neg not supported for type {}",
                                ot
                            )))
                        }
                    }
                    UnaryOp::Not => {
                        if ot == "i1" {
                            self.line(&format!("{} = xor i1 {}, true", res, ov))?;
                            Ok(format!("i1 {}", res))
                        } else {
                            Err(CodegenError::Msg(format!(
                                "not not supported for type {}",
                                ot
                            )))
                        }
                    }
                    UnaryOp::Amp => {
                        if let Expr::Ident(name) = operand.as_ref() {
                            if let Some(alloca) = self.var_alloca.get(name) {
                                let elem_ty = self
                                    .var_ty
                                    .get(name)
                                    .cloned()
                                    .unwrap_or_else(|| "i32".to_string());
                                Ok(format!("{}* {}", elem_ty, alloca))
                            } else {
                                Err(CodegenError::Msg(format!(
                                    "cannot take address of '{}'",
                                    name
                                )))
                            }
                        } else {
                            Err(CodegenError::Msg(
                                "address-of requires an identifier".to_string(),
                            ))
                        }
                    }
                    UnaryOp::Star => {
                        let elem_ty = self.deref_elem_ty(&ot)?;
                        let res2 = self.new_reg();
                        self.line(&format!(
                            "{} = load {}, ptr {}",
                            res2, elem_ty, ov
                        ))?;
                        Ok(format!("{} {}", elem_ty, res2))
                    }
                    UnaryOp::Volatile => {
                        let elem_ty = self.deref_elem_ty(&ot)?;
                        let res2 = self.new_reg();
                        self.line(&format!(
                            "{} = load volatile {}, ptr {}",
                            res2, elem_ty, ov
                        ))?;
                        Ok(format!("{} {}", elem_ty, res2))
                    }
                    UnaryOp::VolatileDeref => {
                        let elem_ty = self.deref_elem_ty(&ot)?;
                        let res2 = self.new_reg();
                        self.line(&format!(
                            "{} = load volatile {}, ptr {}",
                            res2, elem_ty, ov
                        ))?;
                        Ok(format!("{} {}", elem_ty, res2))
                    }
                    UnaryOp::Tilde => {
                        if ot.starts_with('i') {
                            self.line(&format!(
                                "{} = xor {} {}, -1",
                                res, ot, ov
                            ))?;
                            Ok(format!("{} {}", ot, res))
                        } else {
                            Err(CodegenError::Msg(format!(
                                "complement not supported for type {}",
                                ot
                            )))
                        }
                    }
                }
            }
Expr::Call { callee, args } => {
                // Handle enum variant constructors: Enum.Variant(payload)
                let is_enum_variant = matches!(callee.as_ref(), Expr::Member { object, member } if matches!(object.as_ref(), Expr::Ident(_)));
                
                if is_enum_variant {
                    if let Expr::Member { object, member } = callee.as_ref()
                        && let Expr::Ident(enum_name) = object.as_ref()
                        && let Some(edef) = self.enums.get(enum_name).cloned()
                        && let Some((idx, opt_pty)) = edef.variants.iter().enumerate().find(|(_, (vn, _))| *vn == *member)
                    {
                        // Found the variant in this enum
                        if args.len() != 1 {
                            return Err(CodegenError::Msg(format!("variant '{}.{}' expects 1 argument, got {}", enum_name, member, args.len())));
                        }
                        let arg_val = self.emit_expr(&args[0])?;
                        let (at, av) = self.split_ty_val(&arg_val)?;
                        let enum_ty = format!("%enum.{}", enum_name);
                        let r0 = self.new_reg();
                        self.line(&format!("{} = insertvalue {} zeroinitializer, i32 {}, 0", r0, enum_ty, idx))?;
                        let r1 = self.new_reg();
                        self.line(&format!("{} = insertvalue {} {}, {} {}, 1", r1, enum_ty, r0, at, av))?;
                        return Ok(format!("{} {}", enum_ty, r1));
                    }
                }
                // Not an enum variant constructor, proceed with normal function call
                let func_name = match callee.as_ref() {
                    Expr::Ident(name) => name.clone(),
                    Expr::Member { .. } => {
                        self.extract_qualified_name(callee.as_ref())
                            .ok_or_else(|| CodegenError::Msg("invalid qualified function call".into()))?
                    }
                    _ => return Err(CodegenError::Msg("indirect function calls not yet supported".into())),
                };
                
                // Handle CORE memory intrinsics (map to LLVM intrinsics)
                if matches!(func_name.as_str(), "memcpy" | "memmove" | "memset" | "memcmp") {
                    return self.emit_mem_intrinsic(&func_name, args);
                }
                
                if let Some(sig_info) = self.functions.get(&func_name).cloned() {
                    let mut arg_strs = Vec::new();
                    for arg in args {
                        let a = self.emit_expr(arg)?;
                        let (at, av) = self.split_ty_val(&a)?;
                        arg_strs.push(format!("{} {}", at, av));
                    }
                    if sig_info.ret_ty == "void" {
                        self.line(&format!(
                            "call void @{}({})",
                            func_name,
                            arg_strs.join(", ")
                        ))?;
                        Ok("void".to_string())
                    } else {
                        let res = self.new_reg();
                        self.line(&format!(
                            "{} = call {} @{}({})",
                            res,
                            sig_info.ret_ty,
                            func_name,
                            arg_strs.join(", ")
                        ))?;
                        Ok(format!("{} {}", sig_info.ret_ty, res))
                    }
                } else {
                    // Check if bare variant constructor
                    let mut found = None;
                    for (en, edef) in &self.enums {
                        if let Some((idx, (_, opt_pty))) = edef.variants.iter().enumerate().find(|(_, (vn, _))| *vn == func_name) {
                            found = Some((en.clone(), idx, opt_pty.clone()));
                            break;
                        }
                    }
                    if let Some((en, idx, opt_pty)) = found {
                        let enum_ty = format!("%enum.{}", en);
                        let _pty = opt_pty.ok_or_else(|| CodegenError::Msg(format!("variant '{}' takes no payload", func_name)))?;
                        let arg_val = self.emit_expr(&args[0])?;
                        let (at, av) = self.split_ty_val(&arg_val)?;
                        let r0 = self.new_reg();
                        self.line(&format!("{} = insertvalue {} zeroinitializer, i32 {}, 0", r0, enum_ty, idx))?;
                        let r1 = self.new_reg();
                        self.line(&format!("{} = insertvalue {} {}, {} {}, 1", r1, enum_ty, r0, at, av))?;
                        Ok(format!("{} {}", enum_ty, r1))
                    } else {
                        Err(CodegenError::Msg(format!(
                            "undeclared function '{}'",
                            func_name
                        )))
                    }
                }
            }
            Expr::Member { object, member } => {
                if let Expr::Ident(name) = object.as_ref()
                    && let Some(edef) = self.enums.get(name).cloned()
                {
                    let enum_ty = format!("%enum.{}", name);
                    let idx = edef
                        .variants
                        .iter()
                        .position(|(vn, _)| vn == member)
                        .ok_or_else(|| CodegenError::Msg(format!("unknown variant '{}' in enum '{}'", member, name)))?;
                    let r = self.new_reg();
                    self.line(&format!("{} = insertvalue {} zeroinitializer, i32 {}, 0", r, enum_ty, idx))?;
                    return Ok(format!("{} {}", enum_ty, r));
                }
                let obj_ptr = self.emit_expr(object)?;
                let (obj_ty, obj_ptr_str) = self.split_ty_val(&obj_ptr)?;
                
                // Check if this is a trait object (fat pointer with vtable)
                if obj_ty.starts_with('{') && obj_ty.contains("vtable") {
                    // This is a trait object - extract vtable and do dynamic dispatch
                    // Trait object layout: { i8*, %vtable.Trait* }
                    // Extract vtable pointer (second field)
                    let vtable_ptr = self.new_reg();
                    self.line(&format!(
                        "{} = extractvalue {} {}, 1",
                        vtable_ptr, obj_ty, obj_ptr_str
                    ))?;
                    
                    // Get function pointer from vtable
                    // VTable layout: { method1_ptr, method2_ptr, ... }
                    // We need to find the method index from the trait definition
                    let trait_name = self.extract_trait_name_from_ty(&obj_ty);
                    let method_idx = self.get_method_index(&trait_name, member);
                    
                    let method_ptr = self.new_reg();
                    self.line(&format!(
                        "{} = extractvalue {} {}, {}",
                        method_ptr, obj_ty, vtable_ptr, method_idx
                    ))?;
                    
                    // Return function pointer type
                    // For now, return a function pointer type - actual call will be handled in Call
                    return Ok(format!("ptr {}", method_ptr));
                }
                
                // Regular struct member access
                let struct_ty = self.deref_elem_ty(&obj_ty)?;
                let field_idx = self.field_index_from_ty(&struct_ty, member)?;
                let elem_ty = self.field_ty_from_ty(&struct_ty, member)?;
                let ptr = self.new_reg();
                self.line(&format!(
                    "{} = getelementptr {}, ptr {}, i32 0, i32 {}",
                    ptr, struct_ty, obj_ptr_str, field_idx
                ))?;
                let loaded = self.new_reg();
                self.line(&format!(
                    "{} = load {}, ptr {}",
                    loaded, elem_ty, ptr
                ))?;
                Ok(format!("{} {}", elem_ty, loaded))
            }
            Expr::Index { object, index } => {
                let (obj_ty, obj_ptr_str) = if let Expr::Ident(name) = object.as_ref() {
                    if let Some(ty) = self.var_ty.get(name) {
                        if ty.starts_with('[') {
                            (format!("{}*", ty), self.var_alloca.get(name).unwrap().clone())
                        } else {
                            let obj_val = self.emit_expr(object)?;
                            self.split_ty_val(&obj_val)?
                        }
                    } else {
                        let obj_val = self.emit_expr(object)?;
                        self.split_ty_val(&obj_val)?
                    }
                } else {
                    let obj_val = self.emit_expr(object)?;
                    self.split_ty_val(&obj_val)?
                };
                let idx = self.emit_expr(index)?;
                let (idx_ty, idx_str) = self.split_ty_val(&idx)?;
                if idx_ty != "i32" && idx_ty != "i64" {
                    return Err(CodegenError::Msg(format!(
                        "array index must be integer, got {}",
                        idx_ty
                    )));
                }
                let elem_ty = self.index_elem_ty_from_ty(&obj_ty)?;
                let ptr = self.new_reg();
                if let Some(arr_ty) = obj_ty.strip_suffix('*').filter(|s| s.starts_with('[')) {
                    self.line(&format!(
                        "{} = getelementptr {}, ptr {}, i32 0, i32 {}",
                        ptr, arr_ty, obj_ptr_str, idx_str
                    ))?;
                } else {
                    self.line(&format!(
                        "{} = getelementptr {}, ptr {}, i32 {}",
                        ptr, elem_ty, obj_ptr_str, idx_str
                    ))?;
                }
                let loaded = self.new_reg();
                self.line(&format!(
                    "{} = load {}, ptr {}",
                    loaded, elem_ty, ptr
                ))?;
                Ok(format!("{} {}", elem_ty, loaded))
            }
            Expr::Tuple(elems) => {
                let mut elem_vals = Vec::new();
                let mut elem_tys = Vec::new();
                for e in elems {
                    let ev = self.emit_expr(e)?;
                    let (et, ev2) = self.split_ty_val(&ev)?;
                    elem_tys.push(et);
                    elem_vals.push(ev2);
                }
                let struct_ty = format!("{{ {} }}", elem_tys.join(", "));
                let first = self.new_reg();
                self.line(&format!(
                    "{} = insertvalue {} zeroinitializer, {} {}, 0",
                    first, struct_ty, elem_tys[0], elem_vals[0]
                ))?;
                let mut prev = first;
                for (i, ev) in elem_vals.iter().enumerate().skip(1) {
                    let new_val = self.new_reg();
                    self.line(&format!(
                        "{} = insertvalue {} {}, {} {}, {}",
                        new_val, struct_ty, prev, elem_tys[i], ev, i
                    ))?;
                    prev = new_val;
                }
                Ok(format!("{} {}", struct_ty, prev))
            }
            Expr::GenericInst { callee, type_args: _ } => {
                Err(CodegenError::Msg("generic instantiation should be monomorphized before codegen".into()))
            }
            Expr::Cast { expr, ty } => {
                let val = self.emit_expr(expr)?;
                let (from_ty, val_str) = self.split_ty_val(&val)?;
                let to_ty = self.ty_to_llvm(ty)?;
                
                if from_ty == to_ty {
                    return Ok(val);
                }
                
                let result = self.new_reg();
                let cast_instr = match (from_ty.as_str(), to_ty.as_str()) {
                    ("i32", "i64") => "sext",
                    ("i64", "i32") => "trunc",
                    ("i32", "i16") => "trunc",
                    ("i16", "i32") => "sext",
                    ("i8", "i32") => "sext",
                    ("i8", "i64") => "sext",
                    ("u8", "i32") => "zext",
                    ("u8", "i64") => "zext",
                    ("i8*", "i64") => "ptrtoint",
                    ("i64", "i8*") => "inttoptr",
                    _ => "bitcast",
                };
                self.line(&format!(
                    "{} = {} {} {} to {}",
                    result, cast_instr, from_ty, val_str, to_ty
                ))?;
                Ok(format!("{} {}", to_ty, result))
            }
        }
    }

    fn new_reg(&mut self) -> String {
        let r = self.reg_counter;
        self.reg_counter += 1;
        format!("%{}", r)
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let l = self.label_counter;
        self.label_counter += 1;
        format!("{}.{}", prefix, l)
    }

    fn sanitize_name(&self, name: &str) -> String {
        name.replace("::", "_").replace(".", "_").replace("[", "").replace("]", "").replace("*", "").replace(" ", "")
    }

    fn extract_trait_name_from_ty(&self, ty: &str) -> String {
        // Extract trait name from trait object type like "{ i8*, %vtable.Write* }"
        if let Some(start) = ty.find("%vtable.") {
            let after = &ty[start + 8..]; // skip "%vtable."
            if let Some(end) = after.find('*') {
                return after[..end].to_string();
            }
        }
        "unknown".to_string()
    }

    fn get_method_index(&self, trait_name: &str, method_name: &str) -> usize {
        if let Some(trait_def) = self.traits.get(trait_name) {
            for (idx, method) in trait_def.methods.iter().enumerate() {
                if method.name == method_name {
                    return idx;
                }
            }
        }
        0
    }

    fn get_or_emit_string_const(&mut self, s: &str) -> (String, usize) {
        if let Some(existing) = self.string_constants.get(s) {
            let len = s.len() + 1;
            return (existing.clone(), len);
        }
        let gv = format!("@.str.{}", self.string_constants.len());
        let bytes = s.as_bytes();
        let len = s.len() + 1;
        let mut esc = String::new();
        for &b in bytes {
            match b {
                b'\\' => esc.push_str("\\\\"),
                b'"' => esc.push_str("\\22"),
                32..=126 => esc.push(b as char),
                _ => esc.push_str(&format!("\\{:02X}", b)),
            }
        }
        esc.push_str("\\00");
        let def = format!(
            "{} = private unnamed_addr constant [{} x i8] c\"{}\", align 1",
            gv, len, esc
        );
        self.global_defs.push(def);
        self.string_constants.insert(s.to_string(), gv.clone());
        (gv, len)
    }

    fn ty_to_llvm(&self, ty: &Type) -> CodegenResult<String> {
        match ty {
            Type::Bool => Ok("i1".to_string()),
            Type::I8 => Ok("i8".to_string()),
            Type::I16 => Ok("i16".to_string()),
            Type::I32 => Ok("i32".to_string()),
            Type::I64 => Ok("i64".to_string()),
            Type::U8 => Ok("i8".to_string()),
            Type::U16 => Ok("i16".to_string()),
            Type::U32 => Ok("i32".to_string()),
            Type::U64 => Ok("i64".to_string()),
            Type::F32 => Ok("float".to_string()),
            Type::F64 => Ok("double".to_string()),
            Type::Void => Ok("void".to_string()),
            Type::Ptr(inner) => {
                let inner_llvm = self.ty_to_llvm(inner)?;
                if inner_llvm == "void" {
                    Ok("i8*".to_string())
                } else {
                    Ok(format!("{}*", inner_llvm))
                }
            }
            Type::Own(inner) => Ok(format!("{}*", self.ty_to_llvm(inner)?)),
            Type::Named(n) => {
                if n == "string" {
                    Ok("i8*".to_string())
                } else if self.enums.contains_key(n) {
                    Ok(format!("%enum.{}", n))
                } else {
                    Ok(format!("%struct.{}", n))
                }
            }
            Type::Array(inner, n) => {
                Ok(format!("[{} x {}]", n, self.ty_to_llvm(inner)?))
            }
            Type::Tuple(ts) => {
                let elems: Vec<String> = ts
                    .iter()
                    .map(|t| self.ty_to_llvm(t))
                    .collect::<CodegenResult<Vec<_>>>()?;
                Ok(format!("{{ {} }}", elems.join(", ")))
            }
            Type::Union(ts) => {
                let elems: Vec<String> = ts
                    .iter()
                    .map(|t| self.ty_to_llvm(t))
                    .collect::<CodegenResult<Vec<_>>>()?;
                Ok(format!("{{ i32, {} }}", elems.join(", ")))
            }
            Type::Generic(_) => {
                Err(CodegenError::Msg("generic type should be monomorphized before codegen".into()))
            }
            Type::TraitObject(name) => {
                // Trait objects are fat pointers: { i8*, i8* } (data pointer + vtable pointer)
                Ok(format!("{{ i8*, %vtable.{}* }}", name))
            }
            Type::GenericInst(base, _args) => {
                // For now, use the base type's LLVM representation
                self.ty_to_llvm(base)
            }
        }
    }

    fn extract_qualified_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Ident(name) => Some(name.clone()),
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

    fn split_ty_val(&self, s: &str) -> CodegenResult<(String, String)> {
        if let Some(idx) = s.rfind(' ') {
            let ty = s[..idx].to_string();
            let val = s[idx + 1..].to_string();
            Ok((ty, val))
        } else {
            Err(CodegenError::Msg(format!(
                "cannot split type/value: '{}'",
                s
            )))
        }
    }

    fn deref_elem_ty(&self, ty: &str) -> CodegenResult<String> {
        if let Some(inner) = ty.strip_suffix('*') {
            Ok(inner.trim().to_string())
        } else if ty.starts_with("%struct.") || ty.starts_with("%enum.") {
            Ok(ty.to_string())
        } else {
            Err(CodegenError::Msg(format!(
                "cannot dereference non-pointer type {}",
                ty
            )))
        }
    }

    fn field_index_from_ty(&self, ty: &str, member: &str) -> CodegenResult<usize> {
        let struct_name = ty
            .trim_end_matches('*')
            .strip_prefix("%struct.")
            .unwrap_or(ty)
            .trim_end_matches('*');
        let def = self
            .structs
            .get(struct_name)
            .ok_or_else(|| CodegenError::Msg(format!("unknown struct '{}'", struct_name)))?;
        def.fields
            .iter()
            .enumerate()
            .find(|(_, (n, _))| n == member)
            .map(|(i, _)| i)
            .ok_or_else(|| {
                CodegenError::Msg(format!(
                    "struct '{}' has no field '{}'",
                    struct_name, member
                ))
            })
    }

    fn field_ty_from_ty(&self, ty: &str, member: &str) -> CodegenResult<String> {
        let struct_name = ty
            .trim_end_matches('*')
            .strip_prefix("%struct.")
            .unwrap_or(ty)
            .trim_end_matches('*');
        let def = self
            .structs
            .get(struct_name)
            .ok_or_else(|| CodegenError::Msg(format!("unknown struct '{}'", struct_name)))?;
        def.fields
            .iter()
            .find(|(n, _)| n == member)
            .map(|(_, t)| t.clone())
            .ok_or_else(|| {
                CodegenError::Msg(format!(
                    "struct '{}' has no field '{}'",
                    struct_name, member
                ))
            })
    }

    fn index_elem_ty_from_ty(&self, ty: &str) -> CodegenResult<String> {
        let base = ty.strip_suffix('*').unwrap_or(ty).trim();
        if let Some(rest) = base.strip_prefix('[') {
            let parts: Vec<&str> = rest.splitn(2, " x ").collect();
            if parts.len() == 2 {
                Ok(parts[1].trim_end_matches(']').trim().to_string())
            } else {
                Err(CodegenError::Msg(format!("cannot index type {}", ty)))
            }
        } else {
            Ok(base.to_string())
        }
    }

    /// Parse element types from a struct type string like "{ i32, i1 }"
    fn struct_elem_tys(&self, struct_ty: &str) -> CodegenResult<Vec<String>> {
        let inner = struct_ty
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .ok_or_else(|| CodegenError::Msg(format!("not a struct type: {}", struct_ty)))?;
        if inner.trim().is_empty() {
            return Ok(Vec::new());
        }
        Ok(inner
            .split(',')
            .map(|s| s.trim().to_string())
            .collect())
    }

    fn line(&mut self, s: &str) -> CodegenResult<()> {
        if !s.is_empty() {
            let indent = if s.ends_with(':') || s.starts_with('}') || s.starts_with("define ") {
                0
            } else {
                self.indent
            };
            for _ in 0..indent {
                self.functions_output.push_str("    ");
            }
        }
        self.functions_output.push_str(s);
        self.functions_output.push('\n');
        Ok(())
    }

    fn emit_expr_const(&mut self, expr: &Expr) -> CodegenResult<String> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(v) => Ok(format!("i32 {}", v)),
                Literal::Float(v) => Ok(format!("double {}", v)),
                Literal::String(s) => {
                    let (gv, _len) = self.get_or_emit_string_const(s);
                    Ok(format!("i8* getelementptr inbounds [{} x i8], ptr {}, i32 0, i32 0", s.len() + 1, gv))
                }
                Literal::Char(c) => Ok(format!("i8 {}", *c as i32)),
                Literal::Bool(b) => Ok(format!("i1 {}", if *b { 1 } else { 0 })),
                Literal::Null => Ok("i8* null".to_string()),
            },
            Expr::Cast { expr, ty } => {
                let val = self.emit_expr_const(expr)?;
                let to_ty = self.ty_to_llvm(ty)?;
                // Assume from_ty is i32 for integer literals
                let from_ty = "i32";
                if from_ty == to_ty {
                    return Ok(val);
                }
                let cast_instr = match (from_ty, to_ty.as_str()) {
                    ("i32", "i64") => "sext",
                    ("i64", "i32") => "trunc",
                    ("i8*", "i64") => "ptrtoint",
                    ("i64", "i8*") => "inttoptr",
                    _ => "bitcast",
                };
                Ok(format!("{} {} {} to {}", cast_instr, from_ty, val, to_ty))
            }
            Expr::Unary { op, operand } => {
                if matches!(op, UnaryOp::Neg) {
                    let val = self.emit_expr_const(operand)?;
                    if let Some(int_str) = val.strip_prefix("i32 ") {
                        Ok(format!("i32 -{}", int_str))
                    } else if let Some(int_str) = val.strip_prefix("i64 ") {
                        Ok(format!("i64 -{}", int_str))
                    } else {
                        Err(CodegenError::Msg("non-constant expression in constant declaration".into()))
                    }
                } else {
                    Err(CodegenError::Msg("non-constant expression in constant declaration".into()))
                }
            }
            _ => Err(CodegenError::Msg("non-constant expression in constant declaration".into())),
        }
    }
    
    fn emit_mem_intrinsic(&mut self, name: &str, args: &[Expr]) -> CodegenResult<String> {
        match name {
            "memcpy" => {
                // llvm.memcpy.p0.p0.i64(i8* dest, i8* src, i64 len, i1 isvolatile)
                if args.len() != 3 {
                    return Err(CodegenError::Msg("memcpy expects 3 arguments".into()));
                }
                let dest = self.emit_expr(&args[0])?;
                let src = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                let (dest_ty, dest_val) = self.split_ty_val(&dest)?;
                let (src_ty, src_val) = self.split_ty_val(&src)?;
                let (len_ty, len_val) = self.split_ty_val(&len)?;
                let res = self.new_reg();
                self.line(&format!(
                    "{} = call i8* @llvm.memcpy.p0.p0.i64(i8* {}, i8* {}, i64 {}, i1 false)",
                    res, dest_val, src_val, len_val
                ))?;
                Ok(format!("{} {}", dest_ty, res))
            }
            "memmove" => {
                // llvm.memmove.p0.p0.i64(i8* dest, i8* src, i64 len, i1 isvolatile)
                if args.len() != 3 {
                    return Err(CodegenError::Msg("memmove expects 3 arguments".into()));
                }
                let dest = self.emit_expr(&args[0])?;
                let src = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                let (dest_ty, dest_val) = self.split_ty_val(&dest)?;
                let (src_ty, src_val) = self.split_ty_val(&src)?;
                let (len_ty, len_val) = self.split_ty_val(&len)?;
                let res = self.new_reg();
                self.line(&format!(
                    "{} = call i8* @llvm.memmove.p0.p0.i64(i8* {}, i8* {}, i64 {}, i1 false)",
                    res, dest_val, src_val, len_val
                ))?;
                Ok(format!("{} {}", dest_ty, res))
            }
            "memset" => {
                // llvm.memset.p0.i64(i8* dest, i8 val, i64 len, i1 isvolatile)
                if args.len() != 3 {
                    return Err(CodegenError::Msg("memset expects 3 arguments".into()));
                }
                let dest = self.emit_expr(&args[0])?;
                let val = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                let (dest_ty, dest_val) = self.split_ty_val(&dest)?;
                let (_, val_val) = self.split_ty_val(&val)?;
                let (len_ty, len_val) = self.split_ty_val(&len)?;
                let res = self.new_reg();
                self.line(&format!(
                    "{} = call i8* @llvm.memset.p0.i64(i8* {}, i8 {}, i64 {}, i1 false)",
                    res, dest_val, val_val, len_val
                ))?;
                Ok(format!("{} {}", dest_ty, res))
            }
            "memcmp" => {
                // llvm.memcmp.p0.p0.i64(i8* a, i8* b, i64 len, i1 isvolatile) -> i32
                if args.len() != 3 {
                    return Err(CodegenError::Msg("memcmp expects 3 arguments".into()));
                }
                let a = self.emit_expr(&args[0])?;
                let b = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                let (a_ty, a_val) = self.split_ty_val(&a)?;
                let (b_ty, b_val) = self.split_ty_val(&b)?;
                let (len_ty, len_val) = self.split_ty_val(&len)?;
                let res = self.new_reg();
                self.line(&format!(
                    "{} = call i32 @llvm.memcmp.p0.p0.i64(i8* {}, i8* {}, i64 {}, i1 false)",
                    res, a_val, b_val, len_val
                ))?;
                Ok(format!("i32 {}", res))
            }
            _ => Err(CodegenError::Msg(format!("unknown memory intrinsic: {}", name))),
        }
    }
}

impl Default for Codegen {
    fn default() -> Self {
        Self::new()
    }
}

pub fn emit_ir(program: &Program) -> Result<String, String> {
    Codegen::new()
        .emit(program)
        .map_err(|e| e.to_string())
}
