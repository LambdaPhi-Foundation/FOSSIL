#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (uppercase, case-sensitive)
    Fn,
    EndFn,
    If,
    Else,
    EndIf,
    While,
    EndWhile,
    For,
    EndFor,
    Struct,
    EndStruct,
    Enum,
    EndEnum,
    Module,
    EndModule,
    Match,
    EndMatch,
    Case,
    Machine,
    NoMachine,
    Var,
    Const,
    Return,
    Ptr,
    Own,
    Extern,
    Volatile,
    Use,
    As,
    Unreachable,
    Asm,
    EndAsm,
    Backtick,
    AsmBlock(String),
    True,
    False,
    Null,
    Void,
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

    // Literals
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    CharLit(char),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,        // ==
    Neq,       // !=
    Lt,
    Gt,
    Le,        // <=
    Ge,        // >=
    Assign,    // :=
    EqSign,    // =
    And,       // &&
    Or,        // ||
    Not,       // !
    Amp,       // &
    Pipe,      // |
    Xor,       // ^
    Tilde,     // ~
    LShift,    // <<
    RShift,    // >>
    Arrow,     // ->
    Dot,       // .
    DColon,  // ::

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,

    Eof,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: (usize, usize), // (start, end) byte offsets
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn new(kind: TokenKind, start: usize, end: usize, line: usize, col: usize) -> Self {
        Self { kind, span: (start, end), line, col }
    }
}

pub fn keyword_or_type(text: &str) -> Option<TokenKind> {
    use TokenKind::*;
    match text {
        "FN" => Some(Fn),
        "ENDFN" => Some(EndFn),
        "IF" => Some(If),
        "ELSE" => Some(Else),
        "ENDIF" => Some(EndIf),
        "WHILE" => Some(While),
        "ENDWHILE" => Some(EndWhile),
        "FOR" => Some(For),
        "ENDFOR" => Some(EndFor),
        "STRUCT" => Some(Struct),
        "ENDSTRUCT" => Some(EndStruct),
        "ENUM" => Some(Enum),
        "ENDENUM" => Some(EndEnum),
        "MODULE" => Some(Module),
        "ENDMODULE" => Some(EndModule),
        "MATCH" => Some(Match),
        "ENDMATCH" => Some(EndMatch),
        "CASE" => Some(Case),
        "MACHINE" => Some(Machine),
        "NO_MACHINE" => Some(NoMachine),
        "VAR" => Some(Var),
        "CONST" => Some(Const),
        "RETURN" => Some(Return),
        "ptr" => Some(Ptr),
        "own" => Some(Own),
        "TRUE" => Some(True),
        "FALSE" => Some(False),
        "void" => Some(Void),
        "bool" => Some(Bool),
        "i8" => Some(I8),
        "i16" => Some(I16),
        "i32" => Some(I32),
        "i64" => Some(I64),
        "u8" => Some(U8),
        "u16" => Some(U16),
        "u32" => Some(U32),
        "u64" => Some(U64),
        "f32" => Some(F32),
        "f64" => Some(F64),
        "EXTERN" => Some(Extern),
        "VOLATILE" => Some(Volatile),
        "USE" => Some(Use),
        "AS" => Some(As),
        "as" => Some(As),
        "UNREACHABLE" => Some(Unreachable),
        "NULL" => Some(Null),
        "null" => Some(Null),
        "ASM" => Some(Asm),
        "ENDASM" => Some(EndAsm),
        _ => None,
    }
}
