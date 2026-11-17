pub mod ast;
pub mod codegen;
pub mod driver;
pub mod lexer;
pub mod module_loader;
pub mod parser;
pub mod semantic;
pub mod token;

pub use ast::*;
pub use codegen::{emit_ir, Codegen};
pub use driver::{build_source, compile_ir_to_binary, find_clang, run_binary};
pub use lexer::Lexer;
pub use parser::{parse_source, Parser};
pub use semantic::{analyze, SemanticResult};
pub use token::{Token, TokenKind};
