use crate::ast::*;
use crate::lexer::Lexer;
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    input: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, input: String) -> Self {
        Self { tokens, pos: 0, input }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();
        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    // ---------- helpers ----------

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn peek_char(&self) -> char {
        if self.pos < self.input.len() {
            self.input.chars().nth(self.pos).unwrap_or('\0')
        } else {
            '\0'
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<&Token, String> {
        let tok = self.advance();
        if tok.kind != kind {
            return Err(format!(
                "expected {:?} at line {} col {}, got {:?}",
                kind, tok.line, tok.col, tok.kind
            ));
        }
        Ok(tok)
    }

    fn check(&self, kind: TokenKind) -> bool {
        *self.peek_kind() == kind
    }

    // ---------- top level ----------

    fn parse_item(&mut self) -> Result<Item, String> {
        match self.peek_kind() {
            TokenKind::Machine => Ok(Item::MachineDecl(self.parse_machine_decl()?)),
            TokenKind::NoMachine => Ok(Item::MachineDecl(self.parse_machine_decl()?)),
            TokenKind::Struct => Ok(Item::StructDecl(self.parse_struct_decl()?)),
            TokenKind::Enum => Ok(Item::EnumDecl(self.parse_enum_decl()?)),
            TokenKind::Module => Ok(Item::ModuleDecl(self.parse_module_decl()?)),
            TokenKind::Fn => Ok(Item::Function(self.parse_function()?)),
            TokenKind::Extern => Ok(Item::ExternDecl(self.parse_extern_decl()?)),
            TokenKind::Use => Ok(Item::ImportDecl(self.parse_import_decl()?)),
            TokenKind::Const => Ok(Item::ConstDecl(self.parse_const_decl()?)),
            TokenKind::Var => Ok(Item::VarDecl(self.parse_var_decl_item()?)),
            TokenKind::AsmBlock(_) => Ok(Item::AsmBlock(self.parse_asm_block_from_token()?)),
            _ => Err(format!(
                "expected item declaration at line {} col {}, got {:?}",
                self.peek().line, self.peek().col, self.peek_kind()
            )),
        }
    }

    fn parse_machine_decl(&mut self) -> Result<MachineDecl, String> {
        let is_machine = self.check(TokenKind::Machine);
        self.advance();
        Ok(MachineDecl { is_machine })
    }

    fn parse_module_decl(&mut self) -> Result<ModuleDecl, String> {
        self.expect(TokenKind::Module)?;
        let name = self.parse_ident()?;
        let mut items = Vec::new();
        while !self.check(TokenKind::EndModule) && !self.is_at_end() {
            items.push(self.parse_item()?);
        }
        self.expect(TokenKind::EndModule)?;
        Ok(ModuleDecl { name, items })
    }

    fn parse_const_decl(&mut self) -> Result<ConstDecl, String> {
        self.expect(TokenKind::Const)?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        self.expect(TokenKind::EqSign)?;
        let init = self.parse_expr()?;
        Ok(ConstDecl { name, ty, init })
    }

    fn parse_struct_decl(&mut self) -> Result<StructDecl, String> {
        self.expect(TokenKind::Struct)?;
        let name = self.parse_ident()?;
        let generic_params = self.parse_generic_params()?;
        let mut fields = Vec::new();
        while !self.check(TokenKind::EndStruct) && !self.is_at_end() {
            fields.push(self.parse_struct_field()?);
        }
        self.expect(TokenKind::EndStruct)?;
        Ok(StructDecl { name, generic_params, fields })
    }

    fn parse_struct_field(&mut self) -> Result<StructField, String> {
        let name = self.parse_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        Ok(StructField { name, ty })
    }

    fn parse_enum_decl(&mut self) -> Result<EnumDecl, String> {
        self.expect(TokenKind::Enum)?;
        let name = self.parse_ident()?;
        let generic_params = self.parse_generic_params()?;
        let mut variants = Vec::new();
        while !self.check(TokenKind::EndEnum) && !self.is_at_end() {
            variants.push(self.parse_enum_variant()?);
        }
        self.expect(TokenKind::EndEnum)?;
        Ok(EnumDecl { name, generic_params, variants })
    }

    fn parse_enum_variant(&mut self) -> Result<EnumVariant, String> {
        let name = self.parse_ident()?;
        let payload = if self.check(TokenKind::LParen) {
            self.advance();
            let ty = self.parse_type()?;
            self.expect(TokenKind::RParen)?;
            Some(ty)
        } else {
            None
        };
        Ok(EnumVariant { name, payload })
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        self.expect(TokenKind::Fn)?;
        let name = self.parse_ident()?;
        let generic_params = self.parse_generic_params()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Colon)?;
        let return_type = self.parse_type()?;
        let body = self.parse_block()?;
        self.expect(TokenKind::EndFn)?;
        Ok(Function {
            name,
            generic_params,
            params,
            return_type,
            body,
        })
    }

    fn parse_extern_decl(&mut self) -> Result<ExternDecl, String> {
        self.expect(TokenKind::Extern)?;
        self.expect(TokenKind::Fn)?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Colon)?;
        let return_type = self.parse_type()?;
        self.expect(TokenKind::EndFn)?;
        Ok(ExternDecl {
            name,
            params,
            return_type,
        })
    }

    fn parse_import_decl(&mut self) -> Result<ImportDecl, String> {
        self.expect(TokenKind::Use)?;
        
        // Parse the path: MACHINE::IO or ./local/types
        let mut path = Vec::new();
        let first = self.parse_import_ident()?;
        path.push(first);
        
        while self.check(TokenKind::DColon) {
            self.advance(); // consume ::
            let next = self.parse_import_ident()?;
            path.push(next);
        }
        
        // Optional alias: AS alias
        let alias = if self.check_ident_text("AS") || self.check_ident_text("as") {
            self.advance(); // consume AS
            Some(self.parse_import_ident()?)
        } else {
            None
        };
        
        // Optional specific items: :: { item1, item2 }
        let items = if self.check(TokenKind::DColon) {
            self.advance(); // consume ::
            self.expect(TokenKind::LBrace)?;
            let mut items = Vec::new();
            items.push(self.parse_import_ident()?);
            while self.check(TokenKind::Comma) {
                self.advance();
                items.push(self.parse_import_ident()?);
            }
            self.expect(TokenKind::RBrace)?;
            Some(items)
        } else {
            None
        };
        
        Ok(ImportDecl {
            path,
            alias,
            items,
        })
    }

    fn check_ident_text(&self, text: &str) -> bool {
        if let TokenKind::Ident(s) = self.peek_kind() {
            s.eq_ignore_ascii_case(text)
        } else {
            false
        }
    }

    fn parse_import_ident(&mut self) -> Result<String, String> {
        let tok = self.advance();
        match &tok.kind {
            TokenKind::Ident(name) => Ok(name.clone()),
            TokenKind::Machine => Ok("MACHINE".into()),
            TokenKind::NoMachine => Ok("NO_MACHINE".into()),
            TokenKind::Struct => Ok("STRUCT".into()),
            TokenKind::Enum => Ok("ENUM".into()),
            TokenKind::Fn => Ok("FN".into()),
            TokenKind::Extern => Ok("EXTERN".into()),
            TokenKind::Use => Ok("USE".into()),
            TokenKind::Var => Ok("VAR".into()),
            TokenKind::Const => Ok("CONST".into()),
            TokenKind::If => Ok("IF".into()),
            TokenKind::Else => Ok("ELSE".into()),
            TokenKind::While => Ok("WHILE".into()),
            TokenKind::For => Ok("FOR".into()),
            TokenKind::Match => Ok("MATCH".into()),
            TokenKind::Case => Ok("CASE".into()),
            TokenKind::Return => Ok("RETURN".into()),
            TokenKind::Ptr => Ok("PTR".into()),
            TokenKind::Own => Ok("OWN".into()),
            TokenKind::True => Ok("TRUE".into()),
            TokenKind::False => Ok("FALSE".into()),
            TokenKind::Void => Ok("VOID".into()),
            TokenKind::Bool => Ok("BOOL".into()),
            TokenKind::I8 => Ok("I8".into()),
            TokenKind::I16 => Ok("I16".into()),
            TokenKind::I32 => Ok("I32".into()),
            TokenKind::I64 => Ok("I64".into()),
            TokenKind::U8 => Ok("U8".into()),
            TokenKind::U16 => Ok("U16".into()),
            TokenKind::U32 => Ok("U32".into()),
            TokenKind::U64 => Ok("U64".into()),
            TokenKind::F32 => Ok("F32".into()),
            TokenKind::F64 => Ok("F64".into()),
            TokenKind::Volatile => Ok("VOLATILE".into()),
            _ => Err(format!(
                "expected identifier or keyword in import path at line {} col {}, got {:?}",
                tok.line, tok.col, tok.kind
            )),
        }
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::with_capacity(4);
        if self.check(TokenKind::RParen) {
            return Ok(params);
        }
        loop {
            let is_own = self.check(TokenKind::Own);
            if is_own {
                self.advance();
            }
            let name = self.parse_ident()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            params.push(Param { name, ty, is_own });
            if self.check(TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(params)
    }

    fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>, String> {
        let mut params = Vec::new();
        if !self.check(TokenKind::LBracket) {
            return Ok(params);
        }
        self.advance(); // consume '['
        loop {
            let name = self.parse_ident()?;
            params.push(GenericParam { name });
            if self.check(TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(TokenKind::RBracket)?;
        Ok(params)
    }

    // ---------- types ----------

    fn parse_type(&mut self) -> Result<Type, String> {
        if let Some(ty) = Type::from_keyword(self.peek_kind()) {
            self.advance();
            return Ok(ty);
        }
        match self.peek_kind() {
            TokenKind::Ptr => {
                self.advance();
                self.expect(TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(TokenKind::RBracket)?;
                Ok(Type::Ptr(Box::new(inner)))
            }
            TokenKind::Own => {
                self.advance();
                self.expect(TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(TokenKind::RBracket)?;
                Ok(Type::Own(Box::new(inner)))
            }
            TokenKind::LParen => {
                self.advance();
                let first = self.parse_type()?;
                let mut types = Vec::with_capacity(2);
                types.push(first);
                while self.check(TokenKind::Comma) {
                    self.advance();
                    types.push(self.parse_type()?);
                }
                self.expect(TokenKind::RParen)?;
                Ok(Type::Tuple(types))
            }
            TokenKind::LBracket => {
                self.advance();
                let first = self.parse_type()?;
                if self.check(TokenKind::Semicolon) {
                    self.advance();
                    let tok = self.advance();
                    let size = if let TokenKind::IntLit(n) = tok.kind {
                        n as usize
                    } else {
                        return Err(format!(
                            "expected array length integer at line {} col {}, got {:?}",
                            tok.line, tok.col, tok.kind
                        ));
                    };
                    self.expect(TokenKind::RBracket)?;
                    return Ok(Type::Array(Box::new(first), size));
                }
                let mut types = Vec::with_capacity(2);
                types.push(first);
                while self.check(TokenKind::Comma) {
                    self.advance();
                    types.push(self.parse_type()?);
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Type::Union(types))
            }
            TokenKind::Ident(_) => {
                let name = self.parse_ident()?;
                Ok(Type::Named(name))
            }
            _ => Err(format!(
                "expected type at line {} col {}, got {:?}",
                self.peek().line, self.peek().col, self.peek_kind()
            )),
        }
    }

    // ---------- statements ----------

    fn parse_block(&mut self) -> Result<Block, String> {
        let mut statements = Vec::with_capacity(8);
        while !self.is_block_terminator() && !self.is_at_end() {
            statements.push(self.parse_statement()?);
            while self.check(TokenKind::Semicolon) {
                self.advance();
            }
        }
        Ok(Block { statements })
    }

    fn is_block_terminator(&self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::EndFn
                | TokenKind::EndIf
                | TokenKind::EndWhile
                | TokenKind::EndFor
                | TokenKind::Else
                | TokenKind::EndStruct
                | TokenKind::EndEnum
                | TokenKind::EndMatch
                | TokenKind::Case
                | TokenKind::Eof
        )
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        match self.peek_kind() {
            TokenKind::Var => {
                self.advance();
                return self.parse_var_decl(false);
            }
            TokenKind::Const => {
                self.advance();
                return self.parse_var_decl(true);
            }
            TokenKind::If => return self.parse_if_stmt(),
            TokenKind::While => return self.parse_while_stmt(),
            TokenKind::For => return self.parse_for_stmt(),
            TokenKind::Return => return self.parse_return_stmt(),
            TokenKind::Match => return self.parse_match_stmt(),
            TokenKind::Asm => {
                let asm_block = self.parse_asm_block_from_token()?;
                return Ok(Statement::AsmBlock(asm_block));
            }
            TokenKind::AsmBlock(_) => {
                let asm_block = self.parse_asm_block_from_token()?;
                return Ok(Statement::AsmBlock(asm_block));
            }
            TokenKind::Unreachable => {
                self.advance();
                return Ok(Statement::Unreachable);
            }
            _ => {}
        }
        let mut targets = Vec::new();
        targets.push(self.parse_expr()?);
        while self.check(TokenKind::Comma) {
            self.advance();
            targets.push(self.parse_expr()?);
        }

        if self.check(TokenKind::Assign) {
            self.advance();
            let value = self.parse_expr()?;
            let mut converted: Vec<AssignmentTarget> = targets
                .into_iter()
                .map(|e| self.expr_to_target(e))
                .collect::<Result<Vec<_>, _>>()?;
            if converted.len() == 1 {
                return Ok(Statement::Assignment {
                    target: converted.remove(0),
                    value,
                });
            }
            return Ok(Statement::Assignment {
                target: AssignmentTarget::Tuple(converted),
                value,
            });
        }

        if targets.len() == 1 {
            Ok(Statement::Expr(targets.remove(0)))
        } else {
            Err(format!(
                "expected ':=' at line {} col {}, got {:?}",
                self.peek().line, self.peek().col, self.peek_kind()
            ))
        }
    }

    fn parse_var_decl(&mut self, is_const: bool) -> Result<Statement, String> {
        let name = self.parse_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let init = if self.check(TokenKind::EqSign) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(Statement::VarDecl {
            name,
            ty,
            is_const,
            init,
        })
    }

    fn parse_var_decl_item(&mut self) -> Result<VarDecl, String> {
        let name = self.parse_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let init = if self.check(TokenKind::EqSign) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(VarDecl { name, ty, init })
    }

    fn parse_if_stmt(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::If)?;
        let condition = self.parse_expr()?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.check(TokenKind::Else) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };
        self.expect(TokenKind::EndIf)?;
        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::While)?;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        self.expect(TokenKind::EndWhile)?;
        Ok(Statement::While { condition, body })
    }

    fn parse_for_stmt(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::For)?;
        let variable = self.parse_ident()?;
        self.expect(TokenKind::EqSign)?;
        let start = self.parse_expr()?;
        self.expect_keyword("TO")?;
        let end = self.parse_expr()?;
        let body = self.parse_block()?;
        self.expect(TokenKind::EndFor)?;
        Ok(Statement::For { variable, start, end, body })
    }

    fn parse_return_stmt(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Return)?;
        let mut values = Vec::new();
        if !self.is_block_terminator() && !self.check(TokenKind::Semicolon) {
            values.push(self.parse_expr()?);
            while self.check(TokenKind::Comma) {
                self.advance();
                values.push(self.parse_expr()?);
            }
        }
        Ok(Statement::Return { values })
    }

    fn parse_match_stmt(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Match)?;
        let expr = self.parse_expr()?;
        let mut arms = Vec::new();
        let mut else_branch = None;

        while self.check(TokenKind::Case) {
            self.advance(); // consume CASE
            let pattern = self.parse_pattern()?;
            let body = self.parse_block()?;
            arms.push(MatchArm { pattern, body });
        }

        if self.check(TokenKind::Else) {
            self.advance(); // consume ELSE
            let body = self.parse_block()?;
            else_branch = Some(body);
        }

        self.expect(TokenKind::EndMatch)?;
        Ok(Statement::Match { expr, arms, else_branch })
    }

    fn parse_asm_block_from_token(&mut self) -> Result<AsmBlock, String> {
        // Get the AsmBlock token content
        let tok = self.advance();
        let content = match &tok.kind {
            TokenKind::AsmBlock(content) => content.clone(),
            _ => return Err("expected AsmBlock token".into()),
        };
        
        // Parse the raw assembly content into instructions
        let mut instructions = Vec::new();
        let mut clobbers = Vec::new();
        let mut flags = AsmFlags::default();
        
        // Parse the raw assembly content line by line
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if line.is_empty() || line.starts_with(';') {
                i += 1;
                continue;
            }
            
            // Check for CLobbers or Flags at the end
            if line.starts_with("CLobbers") || line.starts_with("clobbers") {
                // Parse clobbers: CLobbers("rax", "rbx")
                let clobber_content = line.trim_start_matches("CLobbers").trim_start_matches("clobbers").trim();
                if clobber_content.starts_with('(') && clobber_content.ends_with(')') {
                    let regs = &clobber_content[1..clobber_content.len()-1];
                    for reg in regs.split(',') {
                        clobbers.push(reg.trim().trim_matches('"').to_string());
                    }
                }
                i += 1;
                continue;
            }
            
            if line.starts_with("Flags") || line.starts_with("flags") {
                // Parse flags: Flags("volatile", "intel")
                let flag_content = line.trim_start_matches("Flags").trim_start_matches("flags").trim();
                if flag_content.starts_with('(') && flag_content.ends_with(')') {
                    let flags_str = &flag_content[1..flag_content.len()-1];
                    for flag in flags_str.split(',') {
                        let flag = flag.trim().trim_matches('"').to_lowercase();
                        match flag.as_str() {
                            "volatile" => flags.volatile = true,
                            "intel" => flags.intel_syntax = true,
                            "noreturn" => flags.noreturn = true,
                            _ => {}
                        }
                    }
                }
                i += 1;
                continue;
            }
            
            // Parse instruction with backtick operands
            let mut template = String::new();
            let mut operands = Vec::new();
            let line_bytes = line.as_bytes();
            let mut pos = 0;
            
            while pos < line_bytes.len() {
                let ch = line_bytes[pos] as char;
                if ch == '`' {
                    // Parse backtick operand
                    pos += 1;
                    let expr_start = pos;
                    while pos < line_bytes.len() && line_bytes[pos] != b'`' {
                        pos += 1;
                    }
                    if pos >= line_bytes.len() {
                        return Err("unclosed backtick".into());
                    }
                    let expr_str = &line[expr_start..pos];
                    // Parse the expression
                    let expr = self.parse_expr_from_str(expr_str)?;
                    operands.push(AsmOperand {
                        expr,
                        is_output: false,
                        is_input: false,
                    });
                    template.push('%');
                    pos += 1; // skip closing backtick
                } else {
                    template.push(ch);
                    pos += 1;
                }
            }
            
            if !template.trim().is_empty() {
                instructions.push(AsmInstruction {
                    template: template.trim().to_string(),
                    operands,
                });
            }
            i += 1;
        }
        
        Ok(AsmBlock {
            instructions,
            clobbers,
            flags,
            is_expr: false,
            result_type: None,
        })
    }
    
    fn parse_expr_from_str(&mut self, s: &str) -> Result<Expr, String> {
        // Parse expression from string using a temporary lexer/parser
        let mut lexer = crate::lexer::Lexer::new(s);
        let tokens = lexer.tokenize();
        let mut parser = crate::parser::Parser::new(tokens, s.to_string());
        parser.parse_expr()
    }

    fn parse_asm_instruction(&mut self) -> Result<AsmInstruction, String> {
        // Parse assembly instruction: read until end of line or ENDASM
        // Extract backtick operands from the template
        let mut template = String::new();
        let mut operands = Vec::new();
        
        // Read raw characters until end of line or ENDASM
        while !self.is_at_end() {
            let ch = self.peek_char();
            
            // Check for ENDASM keyword
            if ch == 'E' || ch == 'e' {
                let saved_pos = self.pos;
                if self.starts_with("ENDASM") {
                    break;
                }
                // Not ENDASM, continue normally
            }
            
            if ch == '\n' {
                self.advance_char();
                break;
            }
            
            if ch == '`' {
                self.advance_char(); // consume backtick
                let expr = self.parse_expr()?;
                if self.peek_char() != '`' {
                    return Err("expected closing backtick".into());
                }
                self.advance_char(); // consume closing backtick
                operands.push(AsmOperand {
                    expr,
                    is_output: false,
                    is_input: false,
                });
                template.push('%'); // placeholder for operand
            } else {
                template.push(ch);
                self.advance_char();
            }
        }
        
        Ok(AsmInstruction {
            template: template.trim().to_string(),
            operands,
        })
    }
    
    fn starts_with(&self, s: &str) -> bool {
        let bytes = s.as_bytes();
        if self.pos + bytes.len() > self.input.len() {
            return false;
        }
        &self.input[self.pos..self.pos + bytes.len()] == s
    }
    
    fn advance_char(&mut self) {
        if self.pos < self.input.len() {
            self.pos += 1;
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        if let TokenKind::Ident(s) = self.peek_kind()
            && s == "_"
        {
            self.advance();
            return Ok(Pattern::Wildcard);
        }

        match self.peek_kind() {
            TokenKind::IntLit(n) => {
                let n = *n;
                self.advance();
                Ok(Pattern::Literal(Literal::Int(n)))
            }
            TokenKind::Minus => {
                self.advance();
                if let TokenKind::IntLit(n) = self.peek_kind() {
                    let n = *n;
                    self.advance();
                    Ok(Pattern::Literal(Literal::Int(-n)))
                } else if let TokenKind::FloatLit(f) = self.peek_kind() {
                    let f = *f;
                    self.advance();
                    Ok(Pattern::Literal(Literal::Float(-f)))
                } else {
                    Err(format!("expected number following '-' in pattern at line {} col {}", self.peek().line, self.peek().col))
                }
            }
            TokenKind::FloatLit(f) => {
                let f = *f;
                self.advance();
                Ok(Pattern::Literal(Literal::Float(f)))
            }
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::String(s)))
            }
            TokenKind::CharLit(c) => {
                let c = *c;
                self.advance();
                Ok(Pattern::Literal(Literal::Char(c)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(false)))
            }
            TokenKind::Ident(_) => {
                let first = self.parse_ident()?;
                if self.check(TokenKind::Dot) {
                    self.advance(); // consume Dot
                    let variant_name = self.parse_ident()?;
                    if self.check(TokenKind::LParen) {
                        self.advance(); // consume (
                        let binding = self.parse_ident()?;
                        self.expect(TokenKind::RParen)?;
                        Ok(Pattern::VariantWithPayload {
                            enum_name: Some(first),
                            variant_name,
                            binding,
                        })
                    } else {
                        Ok(Pattern::Variant {
                            enum_name: Some(first),
                            variant_name,
                        })
                    }
                } else if self.check(TokenKind::LParen) {
                    self.advance(); // consume (
                    let binding = self.parse_ident()?;
                    self.expect(TokenKind::RParen)?;
                    Ok(Pattern::VariantWithPayload {
                        enum_name: None,
                        variant_name: first,
                        binding,
                    })
                } else {
                    Ok(Pattern::Variant {
                        enum_name: None,
                        variant_name: first,
                    })
                }
            }
            _ => Err(format!(
                "expected pattern at line {} col {}, got {:?}",
                self.peek().line,
                self.peek().col,
                self.peek_kind()
            )),
        }
    }

    fn expect_keyword(&mut self, word: &str) -> Result<&Token, String> {
        let tok = self.advance();
        if let TokenKind::Ident(ref s) = tok.kind
            && s == word
        {
            return Ok(tok);
        }
        Err(format!(
            "expected keyword '{}' at line {} col {}, got {:?}",
            word, tok.line, tok.col, tok.kind
        ))
    }

    // ---------- expressions ----------

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assignment_expr()
    }

    fn parse_assignment_expr(&mut self) -> Result<Expr, String> {
        // assignment is handled at statement level; here just parse logical or
        self.parse_logic_or()
    }

    fn parse_logic_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_logic_and()?;
        while self.check(TokenKind::Or) {
            self.advance();
            let right = self.parse_logic_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logic_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_or()?;
        while self.check(TokenKind::And) {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_xor()?;
        while self.check(TokenKind::Pipe) {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Pipe,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_and()?;
        while self.check(TokenKind::Xor) {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Xor,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while self.check(TokenKind::Amp) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Amp,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        while self.check(TokenKind::Eq) || self.check(TokenKind::Neq) {
            let op = match self.peek_kind() {
                TokenKind::Eq => BinaryOp::Eq,
                TokenKind::Neq => BinaryOp::Neq,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift()?;
        while self.check(TokenKind::Lt)
            || self.check(TokenKind::Gt)
            || self.check(TokenKind::Le)
            || self.check(TokenKind::Ge)
        {
            let op = match self.peek_kind() {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Le => BinaryOp::Le,
                TokenKind::Ge => BinaryOp::Ge,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_shift()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term()?;
        while self.check(TokenKind::LShift) || self.check(TokenKind::RShift) {
            let op = match self.peek_kind() {
                TokenKind::LShift => BinaryOp::LShift,
                TokenKind::RShift => BinaryOp::RShift,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_term()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_factor()?;
        while self.check(TokenKind::Plus) || self.check(TokenKind::Minus) {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        while self.check(TokenKind::Star)
            || self.check(TokenKind::Slash)
            || self.check(TokenKind::Percent)
        {
            let op = match self.peek_kind() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.check(TokenKind::Volatile) {
            self.advance();
            if self.check(TokenKind::Star) {
                self.advance();
                let operand = self.parse_unary()?;
                return Ok(Expr::Unary {
                    op: UnaryOp::VolatileDeref,
                    operand: Box::new(operand),
                });
            }
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Volatile,
                operand: Box::new(operand),
            });
        }
        if self.check(TokenKind::Not)
            || self.check(TokenKind::Minus)
            || self.check(TokenKind::Amp)
            || self.check(TokenKind::Star)
            || self.check(TokenKind::Tilde)
        {
            let op = match self.peek_kind() {
                TokenKind::Not => UnaryOp::Not,
                TokenKind::Minus => UnaryOp::Neg,
                TokenKind::Amp => UnaryOp::Amp,
                TokenKind::Star => UnaryOp::Star,
                TokenKind::Tilde => UnaryOp::Tilde,
                _ => unreachable!(),
            };
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op,
                operand: Box::new(operand),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.check(TokenKind::Dot) {
                self.advance();
                let member = self.parse_ident()?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    member,
                };
            } else if self.check(TokenKind::LBracket) {
                // Check if this is generic instantiation (only on identifiers, and next token looks like a type)
                if let Expr::Ident(_) = &expr {
                    // Peek next token to see if it could be a type
                    let next_token = self.peek_kind();
                    let is_type_start = matches!(next_token,
                        TokenKind::Ident(_) |
                        TokenKind::Ptr |
                        TokenKind::Own |
                        TokenKind::LParen |
                        TokenKind::LBracket
                    );
                    if is_type_start {
                        // Try to parse as generic instantiation
                        let saved_pos = self.pos;
                        self.advance(); // consume '['
                        let mut type_args = Vec::new();
                        match self.parse_type() {
                            Ok(ty) => {
                                type_args.push(ty);
                                while self.check(TokenKind::Comma) {
                                    self.advance();
                                    type_args.push(self.parse_type()?);
                                }
                                if self.check(TokenKind::RBracket) {
                                    self.advance(); // consume ']'
                                    expr = Expr::GenericInst {
                                        callee: Box::new(expr),
                                        type_args,
                                    };
                                    continue;
                                }
                            }
                            Err(_) => {}
                        }
                        // Not generic instantiation, restore and parse as index
                        self.pos = saved_pos;
                    }
                }
                // Parse as array index
                self.advance();
                let index = self.parse_expr()?;
                self.expect(TokenKind::RBracket)?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.check(TokenKind::LParen) {
                self.advance();
                let args = self.parse_arg_list()?;
                self.expect(TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.check(TokenKind::As) {
                self.advance();
                let ty = self.parse_type()?;
                expr = Expr::Cast {
                    expr: Box::new(expr),
                    ty,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::with_capacity(4);
        if self.check(TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr()?);
            if self.check(TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek_kind() {
            TokenKind::IntLit(_) => {
                let tok = self.advance();
                if let TokenKind::IntLit(v) = &tok.kind {
                    Ok(Expr::Literal(Literal::Int(*v)))
                } else {
                    unreachable!()
                }
            }
            TokenKind::FloatLit(_) => {
                let tok = self.advance();
                if let TokenKind::FloatLit(v) = &tok.kind {
                    Ok(Expr::Literal(Literal::Float(*v)))
                } else {
                    unreachable!()
                }
            }
            TokenKind::StringLit(_) => {
                let tok = self.advance();
                if let TokenKind::StringLit(s) = &tok.kind {
                    Ok(Expr::Literal(Literal::String(s.clone())))
                } else {
                    unreachable!()
                }
            }
            TokenKind::CharLit(_) => {
                let tok = self.advance();
                if let TokenKind::CharLit(c) = &tok.kind {
                    Ok(Expr::Literal(Literal::Char(*c)))
                } else {
                    unreachable!()
                }
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Literal(Literal::Null))
            }
            TokenKind::LParen => {
                self.advance();
                let first = self.parse_expr()?;
                if self.check(TokenKind::Comma) {
                    // tuple literal: (expr, expr, ...)
                    let mut elems = Vec::new();
                    elems.push(first);
                    while self.check(TokenKind::Comma) {
                        self.advance();
                        elems.push(self.parse_expr()?);
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Tuple(elems))
                } else {
                    // grouped expression: (expr)
                    self.expect(TokenKind::RParen)?;
                    Ok(first)
                }
            }
            TokenKind::Ident(_) => {
                let name = self.parse_ident()?;
                Ok(Expr::Ident(name))
            }
            _ => Err(format!(
                "expected expression at line {} col {}, got {:?}",
                self.peek().line, self.peek().col, self.peek_kind()
            )),
        }
    }

    fn parse_ident(&mut self) -> Result<String, String> {
        let tok = self.advance();
        match &tok.kind {
            TokenKind::Ident(name) => Ok(name.clone()),
            TokenKind::Ptr => Ok("ptr".into()),
            TokenKind::Own => Ok("own".into()),
            _ => Err(format!(
                "expected identifier at line {} col {}, got {:?}",
                tok.line, tok.col, tok.kind
            )),
        }
    }

    fn expr_to_target(&self, expr: Expr) -> Result<AssignmentTarget, String> {
        match expr {
            Expr::Ident(name) => Ok(AssignmentTarget::Ident(name)),
            Expr::Member { object, member } => Ok(AssignmentTarget::Member { object, member }),
            Expr::Index { object, index } => Ok(AssignmentTarget::Index { object, index }),
            Expr::Unary { op, operand } => {
                match op {
                    UnaryOp::Star => {
                        Ok(AssignmentTarget::Index {
                            object: operand,
                            index: Box::new(Expr::Literal(Literal::Int(0))),
                        })
                    }
                    UnaryOp::VolatileDeref => {
                        Ok(AssignmentTarget::VolatileDeref { object: operand })
                    }
                    UnaryOp::Volatile => {
                        match *operand {
                            Expr::Unary { op: UnaryOp::Star, operand: inner } => {
                                Ok(AssignmentTarget::Index {
                                    object: inner,
                                    index: Box::new(Expr::Literal(Literal::Int(0))),
                                })
                            }
                            _ => Err("VOLATILE assignment target must be a dereference".to_string()),
                        }
                    }
                    _ => Err("invalid assignment target".to_string()),
                }
            }
            Expr::Tuple(elems) => {
                let mut targets = Vec::new();
                for e in elems {
                    targets.push(self.expr_to_target(e)?);
                }
                Ok(AssignmentTarget::Tuple(targets))
            }
            _ => Err("invalid assignment target".to_string()),
        }
    }
}

pub fn parse_source(source: &str) -> Result<Program, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens, source.to_string());
    parser.parse()
}
