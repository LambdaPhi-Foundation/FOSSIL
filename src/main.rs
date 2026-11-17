use carnax::driver::{build_source, find_clang};
use carnax::{emit_ir, parse_source, analyze};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn print_usage() {
    println!(
        "Carnax Compiler (FOSSIL Language) v0.1.0\n\n\
         USAGE:\n    \
             carnax <source.fsl> [output.ll | output.exe]\n    \
             carnax build <source.fsl> [-o <output.exe>]\n    \
             carnax run <source.fsl> [-- <args>...]\n    \
             carnax emit-ir <source.fsl> [-o <output.ll>]\n\n\
         COMMANDS:\n    \
             build      Compile .fsl source into a native binary\n    \
             run        Compile, execute, and return program exit code\n    \
             emit-ir    Emit LLVM IR to stdout or file\n\n\
         OPTIONS:\n    \
             -o <file>  Specify output path\n    \
             -h, --help Show this help information\n    \
             -V, --version Show version"
    );
}

fn main() {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    if raw_args.is_empty() {
        print_usage();
        process::exit(1);
    }

    match raw_args[0].as_str() {
        "-h" | "--help" | "help" => {
            print_usage();
            process::exit(0);
        }
        "-V" | "--version" => {
            println!("carnax 0.1.0");
            if let Ok(clang) = find_clang() {
                println!("LLVM backend: {}", clang.display());
            } else {
                println!("LLVM backend: not found");
            }
            process::exit(0);
        }
        "build" => {
            if raw_args.len() < 2 {
                eprintln!("error: 'build' requires a source file path");
                eprintln!("usage: carnax build <source.fsl> [-o <output.exe>]");
                process::exit(1);
            }
            let source_path = Path::new(&raw_args[1]);
            let mut output_path: Option<PathBuf> = None;

            let mut i = 2;
            while i < raw_args.len() {
                if raw_args[i] == "-o" && i + 1 < raw_args.len() {
                    output_path = Some(PathBuf::from(&raw_args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("error: unexpected argument '{}'", raw_args[i]);
                    process::exit(1);
                }
            }

            match build_source(source_path, output_path.as_deref(), false, &[]) {
                Ok(code) => process::exit(code),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            }
        }
        "run" => {
            if raw_args.len() < 2 {
                eprintln!("error: 'run' requires a source file path");
                eprintln!("usage: carnax run <source.fsl> [-- <args>...]");
                process::exit(1);
            }
            let source_path = Path::new(&raw_args[1]);
            let mut run_args = Vec::new();
            if let Some(pos) = raw_args.iter().position(|a| a == "--") {
                run_args.extend_from_slice(&raw_args[pos + 1..]);
            }

            match build_source(source_path, None, true, &run_args) {
                Ok(code) => process::exit(code),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            }
        }
        "emit-ir" => {
            if raw_args.len() < 2 {
                eprintln!("error: 'emit-ir' requires a source file path");
                eprintln!("usage: carnax emit-ir <source.fsl> [-o <output.ll>]");
                process::exit(1);
            }
            let source_path = Path::new(&raw_args[1]);
            let mut output_path: Option<PathBuf> = None;

            let mut i = 2;
            while i < raw_args.len() {
                if raw_args[i] == "-o" && i + 1 < raw_args.len() {
                    output_path = Some(PathBuf::from(&raw_args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("error: unexpected argument '{}'", raw_args[i]);
                    process::exit(1);
                }
            }

            handle_emit_ir(source_path, output_path.as_deref());
        }
        _ => {
            // Default / direct invocation: carnax <source.fsl> [-o out] [out.ll]
            let source_path = Path::new(&raw_args[0]);
            if raw_args.len() == 1 {
                // Just source: print LLVM IR to stdout
                handle_emit_ir(source_path, None);
            } else if raw_args[1] == "-o" && raw_args.len() >= 3 {
                let out_path = Path::new(&raw_args[2]);
                if out_path.extension().and_then(|s| s.to_str()) == Some("ll") {
                    handle_emit_ir(source_path, Some(out_path));
                } else {
                    match build_source(source_path, Some(out_path), false, &[]) {
                        Ok(code) => process::exit(code),
                        Err(e) => {
                            eprintln!("{}", e);
                            process::exit(1);
                        }
                    }
                }
            } else {
                let out_path = Path::new(&raw_args[1]);
                if out_path.extension().and_then(|s| s.to_str()) == Some("ll") {
                    handle_emit_ir(source_path, Some(out_path));
                } else {
                    match build_source(source_path, Some(out_path), false, &[]) {
                        Ok(code) => process::exit(code),
                        Err(e) => {
                            eprintln!("{}", e);
                            process::exit(1);
                        }
                    }
                }
            }
        }
    }
}

fn handle_emit_ir(source_path: &Path, output_path: Option<&Path>) {
    let source = match fs::read_to_string(source_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading '{}': {}", source_path.display(), e);
            process::exit(1);
        }
    };

    match parse_source(&source) {
        Ok(program) => {
            let result = analyze(&program);
            if result.has_errors {
                for err in &result.errors {
                    eprintln!("semantic error: {}", err);
                }
                process::exit(1);
            }

            match emit_ir(&program) {
                Ok(ir) => {
                    if let Some(out) = output_path {
                        match fs::write(out, &ir) {
                            Ok(_) => {
                                println!("wrote LLVM IR to '{}'", out.display());
                            }
                            Err(e) => {
                                eprintln!("error writing '{}': {}", out.display(), e);
                                process::exit(1);
                            }
                        }
                    } else {
                        print!("{}", ir);
                    }
                }
                Err(e) => {
                    eprintln!("codegen error: {}", e);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("parse error: {}", e);
            process::exit(1);
        }
    }
}
