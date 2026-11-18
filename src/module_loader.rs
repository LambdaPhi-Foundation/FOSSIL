use crate::ast::*;
use crate::parser::parse_source;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct ModuleLoader {
    module_cache: HashMap<String, ModuleDecl>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            module_cache: HashMap::new(),
        }
    }

    pub fn load_module(&mut self, path: &[String]) -> Result<ModuleDecl, String> {
        if path.is_empty() {
            return Err("Empty module path".into());
        }

        match path[0].as_str() {
            "MACHINE" => self.load_machine_module(path),
            "CORE" => self.load_core_module(path),
            _ => Err(format!("Unknown module prefix: {}", path[0])),
        }
    }

    pub fn load_machine_module(&mut self, path: &[String]) -> Result<ModuleDecl, String> {
        if path.is_empty() || path[0] != "MACHINE" {
            return Err("Not a MACHINE module path".into());
        }

        let module_key = path.join("::");
        if let Some(cached) = self.module_cache.get(&module_key) {
            return Ok(cached.clone());
        }

        let module = if path.len() == 1 {
            // MACHINE root - create empty module
            ModuleDecl {
                name: "MACHINE".into(),
                items: vec![],
                reexports: vec![],
            }
        } else if path.len() == 2 {
            let module_name = path[1].to_lowercase();
            match module_name.as_str() {
                "io" => self.load_io_module()?,
                "win32" => self.load_win32_module()?,
                "io_handles" => self.load_io_handles_module()?,
                "conout" => self.load_conout_module()?,
                "conin" => self.load_conin_module()?,
                "conerr" => self.load_conerr_module()?,
                _ => return Err(format!("Unknown MACHINE module: {}", path[1])),
            }
        } else {
            return Err(format!("Invalid MACHINE module path: {}", module_key));
        };

        self.module_cache.insert(module_key, module.clone());
        Ok(module)
    }

    pub fn load_core_module(&mut self, path: &[String]) -> Result<ModuleDecl, String> {
        if path.is_empty() || path[0] != "CORE" {
            return Err("Not a CORE module path".into());
        }

        let module_key = path.join("::");
        if let Some(cached) = self.module_cache.get(&module_key) {
            return Ok(cached.clone());
        }

        let module = if path.len() == 1 {
            // CORE root - create empty module
            ModuleDecl {
                name: "CORE".into(),
                items: vec![],
                reexports: vec![],
            }
        } else if path.len() == 2 {
            let module_name = path[1].to_lowercase();
            match module_name.as_str() {
                "mem" => self.load_core_file("mem.fsl", "mem")?,
                "slice" => self.load_core_file("slice.fsl", "slice")?,
                "io" => self.load_core_file("io.fsl", "io")?,
                "fmt" => self.load_core_file("fmt.fsl", "fmt")?,
                "err" => self.load_core_file("err.fsl", "err")?,
                "panic" => self.load_core_file("panic.fsl", "panic")?,
                "alloc" => self.load_core_file("alloc.fsl", "alloc")?,
                "atomic" => self.load_core_file("atomic.fsl", "atomic")?,
                _ => return Err(format!("Unknown CORE module: {}", path[1])),
            }
        } else {
            return Err(format!("Invalid CORE module path: {}", module_key));
        };

        self.module_cache.insert(module_key, module.clone());
        Ok(module)
    }

    fn load_core_file(&self, filename: &str, expected_name: &str) -> Result<ModuleDecl, String> {
        let path = Path::new("src/core").join(filename);
        let source = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read CORE module file {}: {}", filename, e))?;
        self.parse_module(&source, expected_name)
    }

    fn load_file(&self, filename: &str) -> Result<String, String> {
        let path = Path::new("src/machine").join(filename);
        fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read module file {}: {}", filename, e))
    }

    fn parse_module(&self, source: &str, expected_name: &str) -> Result<ModuleDecl, String> {
        let program = parse_source(source)?;
        for item in &program.items {
            if let Item::ModuleDecl(m) = item {
                if m.name == expected_name {
                    return Ok(m.clone());
                }
            }
        }
        Err(format!("Module '{}' not found in parsed file", expected_name))
    }

    fn load_io_module(&self) -> Result<ModuleDecl, String> {
        let source = self.load_file("io.fsl")?;
        self.parse_module(&source, "IO")
    }

    fn load_win32_module(&self) -> Result<ModuleDecl, String> {
        let source = self.load_file("win32.fsl")?;
        self.parse_module(&source, "win32")
    }

    fn load_io_handles_module(&self) -> Result<ModuleDecl, String> {
        let source = self.load_file("io_handles.fsl")?;
        self.parse_module(&source, "io_handles")
    }

    fn load_conout_module(&self) -> Result<ModuleDecl, String> {
        let source = self.load_file("conout.fsl")?;
        self.parse_module(&source, "Conout")
    }

    fn load_conin_module(&self) -> Result<ModuleDecl, String> {
        let source = self.load_file("conin.fsl")?;
        self.parse_module(&source, "Conin")
    }

    fn load_conerr_module(&self) -> Result<ModuleDecl, String> {
        let source = self.load_file("conerr.fsl")?;
        self.parse_module(&source, "Conerr")
    }
}