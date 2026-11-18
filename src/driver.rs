use crate::codegen::emit_ir;
use crate::parser::parse_source;
use crate::semantic::analyze;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Locates a working Clang binary on the system.
pub fn find_clang() -> Result<PathBuf, String> {
    // 1. Explicit CLANG or LLVM_PATH environment variable
    if let Ok(clang_env) = env::var("CLANG") {
        let path = PathBuf::from(&clang_env);
        if path.exists() {
            return Ok(path);
        }
    }
    if let Ok(llvm_path) = env::var("LLVM_PATH") {
        let candidate = PathBuf::from(&llvm_path).join("bin").join("clang.exe");
        if candidate.exists() {
            return Ok(candidate);
        }
        let candidate_nix = PathBuf::from(&llvm_path).join("bin").join("clang");
        if candidate_nix.exists() {
            return Ok(candidate_nix);
        }
    }

    // 2. Check if clang is in PATH
    let probe = if cfg!(windows) { "clang.exe" } else { "clang" };
    if let Ok(status) = Command::new(probe)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        && status.success()
    {
        return Ok(PathBuf::from(probe));
    }

    // 3. Known standard installation paths on Windows
    if cfg!(windows) {
        let candidates = [
            r"C:\Program Files\LLVM\bin\clang.exe",
            r"C:\Program Files (x86)\LLVM\bin\clang.exe",
            r"C:\msys64\ucrt64\bin\clang.exe",
            r"C:\msys64\mingw64\bin\clang.exe",
        ];
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Ok(p);
            }
        }

        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data)
                .join("Programs")
                .join("LLVM")
                .join("bin")
                .join("clang.exe");
            if p.exists() {
                return Ok(p);
            }
        }
    }

    // 4. Known standard paths on Unix
    if cfg!(unix) {
        let candidates = ["/usr/bin/clang", "/usr/local/bin/clang"];
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Ok(p);
            }
        }
    }

    Err("Could not find Clang in PATH or standard installation directories.\n\
         Please install LLVM / Clang or set the CLANG environment variable."
        .to_string())
}

/// Compiles an LLVM IR string to a native binary using Clang.
pub fn compile_ir_to_binary(
    ir: &str,
    output_path: &Path,
    is_machine: bool,
) -> Result<(), String> {
    let clang = find_clang()?;

    // Create a temporary .ll file
    let temp_dir = env::temp_dir();
    let temp_ll_name = format!("carnax_build_{}_{}.ll", std::process::id(), fastrand());
    let temp_ll = temp_dir.join(&temp_ll_name);

    if let Err(e) = fs::write(&temp_ll, ir) {
        return Err(format!("Failed to write temporary LLVM IR file: {}", e));
    }

    let is_obj = output_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext == "o" || ext == "obj")
        .unwrap_or(false);

    // For MACHINE mode executables, use two-step: IR -> .o -> executable
    // This ensures proper CRT linking on Windows MSVC
    if is_machine && !is_obj {
        let temp_obj_name = format!("carnax_build_{}_{}.o", std::process::id(), fastrand());
        let temp_obj = temp_dir.join(&temp_obj_name);

        // Step 1: Compile IR to object file
        let mut cmd1 = Command::new(&clang);
        cmd1.arg(&temp_ll);
        cmd1.arg("-c");
        cmd1.arg("-o").arg(&temp_obj);
        cmd1.arg("-Wno-override-module");

        let output1 = match cmd1.output() {
            Ok(out) => out,
            Err(e) => {
                let _ = fs::remove_file(&temp_ll);
                return Err(format!("Failed to execute clang ({}): {}", clang.display(), e));
            }
        };

        if !output1.status.success() {
            let err_msg = String::from_utf8_lossy(&output1.stderr);
            let _ = fs::remove_file(&temp_ll);
            return Err(format!(
                "Clang compile to object failed (exit code {}):\n{}",
                output1.status.code().unwrap_or(1),
                err_msg.trim()
            ));
        }

        // Step 2: Link object file to executable
        let mut cmd2 = Command::new(&clang);
        cmd2.arg(&temp_obj);
        cmd2.arg("-o").arg(output_path);

        let output2 = match cmd2.output() {
            Ok(out) => out,
            Err(e) => {
                let _ = fs::remove_file(&temp_ll);
                let _ = fs::remove_file(&temp_obj);
                return Err(format!("Failed to execute clang linker ({}): {}", clang.display(), e));
            }
        };

        // Clean up
        let _ = fs::remove_file(&temp_ll);
        let _ = fs::remove_file(&temp_obj);

        if !output2.status.success() {
            let err_msg = String::from_utf8_lossy(&output2.stderr);
            return Err(format!(
                "Clang link failed (exit code {}):\n{}",
                output2.status.code().unwrap_or(1),
                err_msg.trim()
            ));
        }

        return Ok(());
    }

    // For NO_MACHINE or object file output, use single step
    let mut cmd = Command::new(&clang);
    cmd.arg(&temp_ll);
    cmd.arg("-o").arg(output_path);
    cmd.arg("-Wno-override-module");

    if !is_machine {
        if is_obj {
            cmd.arg("-c");
            cmd.arg("-ffreestanding");
        } else {
            cmd.arg("-ffreestanding");
        }
    } else {
        // MACHINE mode object file
        cmd.arg("-c");
    }

    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            let _ = fs::remove_file(&temp_ll);
            return Err(format!("Failed to execute clang ({}): {}", clang.display(), e));
        }
    };

    // Clean up temporary IR file
    let _ = fs::remove_file(&temp_ll);

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Clang compilation failed (exit code {}):\n{}",
            output.status.code().unwrap_or(1),
            err_msg.trim()
        ));
    }

    Ok(())
}

/// Executes a native binary, returning its exit code.
pub fn run_binary(binary_path: &Path, args: &[String]) -> Result<i32, String> {
    let mut cmd = Command::new(binary_path);
    cmd.args(args);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) => Ok(status.code().unwrap_or(0)),
        Err(e) => Err(format!(
            "Failed to execute '{}': {}",
            binary_path.display(),
            e
        )),
    }
}

/// Full end-to-end build pipeline for a RELIC source file.
pub fn build_source(
    source_path: &Path,
    output_path: Option<&Path>,
    run_after_build: bool,
    run_args: &[String],
) -> Result<i32, String> {
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read '{}': {}", source_path.display(), e))?;

    let program = parse_source(&source).map_err(|e| format!("Parse error: {}", e))?;

    let result = analyze(&program);
    if result.has_errors {
        let mut msg = String::new();
        for err in &result.errors {
            msg.push_str(&format!("Semantic error: {}\n", err));
        }
        return Err(msg.trim_end().to_string());
    }

    let is_machine = program
        .items
        .iter()
        .find_map(|item| {
            if let crate::ast::Item::MachineDecl(m) = item {
                Some(m.is_machine)
            } else {
                None
            }
        })
        .unwrap_or(true);

    let ir = emit_ir(&program).map_err(|e| format!("Codegen error: {}", e))?;

    // Case 1: Just running temporarily
    if run_after_build {
        let temp_dir = env::temp_dir();
        let ext = if cfg!(windows) { "exe" } else { "out" };
        let temp_exe = temp_dir.join(format!(
            "carnax_run_{}_{}.{}",
            std::process::id(),
            fastrand(),
            ext
        ));

        compile_ir_to_binary(&ir, &temp_exe, is_machine)?;
        let exit_code = run_binary(&temp_exe, run_args);
        let _ = fs::remove_file(&temp_exe);
        return exit_code;
    }

    // Case 2: Writing to specified or default output
    let default_output = if cfg!(windows) {
        source_path.with_extension("exe")
    } else {
        source_path.with_extension("")
    };
    let target_output = output_path.unwrap_or(&default_output);

    let is_ll = target_output
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext == "ll")
        .unwrap_or(false);

    if is_ll {
        fs::write(target_output, &ir)
            .map_err(|e| format!("Error writing '{}': {}", target_output.display(), e))?;
        println!("Wrote LLVM IR to '{}'", target_output.display());
    } else {
        compile_ir_to_binary(&ir, target_output, is_machine)?;
        println!("Built native binary '{}'", target_output.display());
    }

    Ok(0)
}

/// A lightweight non-cryptographic random token generator for unique temp files.
fn fastrand() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    now as u64
}
