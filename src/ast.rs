use crate::token::TokenKind;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    StructDecl(StructDecl),
    EnumDecl(EnumDecl),
    ModuleDecl(ModuleDecl),
    MachineDecl(MachineDecl),
    ExternDecl(ExternDecl),
    ImportDecl(ImportDecl),
    ConstDecl(ConstDecl),
    VarDecl(VarDecl),
    AsmBlock(AsmBlock),
}

#[derive(Debug, Clone)]
pub struct MachineDecl {
    pub is_machine: bool, // true = MACHINE, false = NO_MACHINE
}

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub ty: Type,
    pub init: Expr,
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub ty: Type,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct ExternDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: Vec<String>,  // e.g., ["MACHINE", "IO"] or ["local", "types"]
    pub alias: Option<String>,  // optional: USE ... AS alias
    pub items: Option<Vec<String>>,  // optional: USE ... :: { item1, item2 }
}

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub is_own: bool,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub payload: Option<Type>,
}

#[derive(Debug, Clone)]
pub enum Type {
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
    Ptr(Box<Type>),
    Own(Box<Type>),
    Named(String),
    Array(Box<Type>, usize),
    Tuple(Vec<Type>),       // (T1, T2, ...) - product type, all values
    Union(Vec<Type>),       // [T1, T2, ...] - sum type, any one value
    Generic(String),        // Generic type parameter (T, U, etc.)
}

impl Type {
    pub fn from_keyword(kind: &TokenKind) -> Option<Type> {
        use TokenKind::*;
        Some(match kind {
            Bool => Type::Bool,
            I8 => Type::I8,
            I16 => Type::I16,
            I32 => Type::I32,
            I64 => Type::I64,
            U8 => Type::U8,
            U16 => Type::U16,
            U32 => Type::U32,
            U64 => Type::U64,
            F32 => Type::F32,
            F64 => Type::F64,
            Void => Type::Void,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    VarDecl {
        name: String,
        ty: Type,
        is_const: bool,
        init: Option<Expr>,
    },
    Assignment {
        target: AssignmentTarget,
        value: Expr,
    },
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    While {
        condition: Expr,
        body: Block,
    },
    For {
        variable: String,
        start: Expr,
        end: Expr,
        body: Block,
    },
    Return {
        values: Vec<Expr>,
    },
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
        else_branch: Option<Block>,
    },
    Expr(Expr),
    AsmBlock(AsmBlock),
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Literal),
    Variant {
        enum_name: Option<String>,
        variant_name: String,
    },
    VariantWithPayload {
        enum_name: Option<String>,
        variant_name: String,
        binding: String,
    },
    Wildcard,
}

#[derive(Debug, Clone)]
pub enum AssignmentTarget {
    Ident(String),
    Member { object: Box<Expr>, member: String },
    Index { object: Box<Expr>, index: Box<Expr> },
    VolatileDeref { object: Box<Expr> }, // VOLATILE *ptr
    Tuple(Vec<AssignmentTarget>), // a, b := f()
}

#[derive(Debug, Clone, Default)]
pub struct AsmFlags {
    pub volatile: bool,
    pub intel_syntax: bool,
    pub noreturn: bool,
}

#[derive(Debug, Clone)]
pub struct AsmOperand {
    pub expr: Expr,
    pub is_output: bool,
    pub is_input: bool,
}

#[derive(Debug, Clone)]
pub struct AsmInstruction {
    pub template: String,
    pub operands: Vec<AsmOperand>,
}

#[derive(Debug, Clone)]
pub struct AsmBlock {
    pub instructions: Vec<AsmInstruction>,
    pub clobbers: Vec<String>,
    pub flags: AsmFlags,
    pub is_expr: bool,
    pub result_type: Option<Type>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Member {
        object: Box<Expr>,
        member: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    GenericInst {
        callee: Box<Expr>,
        type_args: Vec<Type>,
    },
    Cast {
        expr: Box<Expr>,
        ty: Type,
    },
    Tuple(Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Amp,
    Pipe,
    Xor,
    LShift,
    RShift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    Amp,
    Star, // deref
    Volatile, // volatile load/store (prefix to deref)
    VolatileDeref, // VOLATILE *ptr combined
    Tilde,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Null,
}
