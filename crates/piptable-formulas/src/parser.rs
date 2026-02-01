//! Formula parser module

use crate::{BinaryOperator, FormulaError, FormulaExpr, UnaryOperator};
use piptable_primitives::{CellAddress, CellRange, ErrorValue, R1C1Ref, Value};

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Number { value: f64, is_int: bool },
    String(String),
    Identifier(String),
    SheetName(String),
    Error(ErrorValue),
    CellRef(String),
    LParen,
    RParen,
    Comma,
    Semicolon,
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Ampersand,
    Percent,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Colon,
    Bang,
    Eof,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    pos: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedCellRef {
    A1(CellAddress),
    R1C1(R1C1Ref),
}

struct Lexer<'a> {
    input: &'a str,
    chars: Vec<(usize, char)>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().collect(),
            pos: 0,
        }
    }

    fn next_token(&mut self) -> Result<Token, FormulaError> {
        self.skip_whitespace();
        let start = self.pos;
        let ch = match self.peek() {
            Some(ch) => ch,
            None => {
                return Ok(Token {
                    kind: TokenKind::Eof,
                    pos: self.pos,
                })
            }
        };

        let token = match ch {
            '(' => self.simple(TokenKind::LParen),
            ')' => self.simple(TokenKind::RParen),
            ',' => self.simple(TokenKind::Comma),
            ';' => self.simple(TokenKind::Semicolon),
            '+' => self.simple(TokenKind::Plus),
            '-' => self.simple(TokenKind::Minus),
            '*' => self.simple(TokenKind::Star),
            '/' => self.simple(TokenKind::Slash),
            '^' => self.simple(TokenKind::Caret),
            '&' => self.simple(TokenKind::Ampersand),
            '%' => self.simple(TokenKind::Percent),
            ':' => self.simple(TokenKind::Colon),
            '!' => self.simple(TokenKind::Bang),
            '=' => self.simple(TokenKind::Equal),
            '<' => {
                self.advance();
                if self.consume('=') {
                    TokenKind::LessEqual
                } else if self.consume('>') {
                    TokenKind::NotEqual
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                self.advance();
                if self.consume('=') {
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            '"' => self.string_token()?,
            '\'' => self.sheet_name_token()?,
            '#' => self.error_token()?,
            '.' | '0'..='9' => self.number_token()?,
            '$' | 'A'..='Z' | 'a'..='z' | '_' => self.identifier_or_cell_token()?,
            _ => {
                return Err(FormulaError::ParseError(format!(
                    "Unexpected character '{}' at {}",
                    ch, start
                )))
            }
        };

        Ok(Token {
            kind: token,
            pos: start,
        })
    }

    fn simple(&mut self, kind: TokenKind) -> TokenKind {
        self.advance();
        kind
    }

    fn string_token(&mut self) -> Result<TokenKind, FormulaError> {
        self.advance(); // consume opening "
        let mut result = String::new();
        while let Some(ch) = self.peek() {
            self.advance();
            if ch == '"' {
                if self.consume('"') {
                    result.push('"');
                    continue;
                }
                return Ok(TokenKind::String(result));
            }
            result.push(ch);
        }
        Err(FormulaError::ParseError(
            "Unterminated string literal".to_string(),
        ))
    }

    fn sheet_name_token(&mut self) -> Result<TokenKind, FormulaError> {
        self.advance(); // consume opening '
        let mut result = String::new();
        while let Some(ch) = self.peek() {
            self.advance();
            if ch == '\'' {
                if self.consume('\'') {
                    result.push('\'');
                    continue;
                }
                return Ok(TokenKind::SheetName(result));
            }
            result.push(ch);
        }
        Err(FormulaError::ParseError(
            "Unterminated sheet name".to_string(),
        ))
    }

    fn error_token(&mut self) -> Result<TokenKind, FormulaError> {
        self.advance(); // consume '#'
        let mut literal = String::from("#");
        while let Some(ch) = self.peek() {
            if is_error_delimiter(ch) {
                break;
            }
            literal.push(ch);
            self.advance();
        }
        let err = parse_error_literal(&literal)?;
        Ok(TokenKind::Error(err))
    }

    fn number_token(&mut self) -> Result<TokenKind, FormulaError> {
        let start = self.pos;
        let mut seen_dot = false;
        let mut seen_exp = false;

        while let Some(ch) = self.peek() {
            match ch {
                '0'..='9' => {
                    self.advance();
                }
                '.' if !seen_dot && !seen_exp => {
                    seen_dot = true;
                    self.advance();
                }
                'e' | 'E' if !seen_exp => {
                    seen_exp = true;
                    self.advance();
                    if self.consume('+') || self.consume('-') {
                        // optional exponent sign
                    }
                }
                _ => break,
            }
        }

        let text = self.slice(start, self.pos);
        let value: f64 = text
            .parse()
            .map_err(|_| FormulaError::ParseError(format!("Invalid number literal '{}'", text)))?;
        let is_int = !seen_dot && !seen_exp;
        Ok(TokenKind::Number { value, is_int })
    }

    fn identifier_or_cell_token(&mut self) -> Result<TokenKind, FormulaError> {
        let start = self.pos;
        if let Some(token) = self.r1c1_cell_token(start) {
            return Ok(token);
        }
        let _ = self.consume('$');
        let mut col_letters = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphabetic() {
                col_letters.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        if !col_letters.is_empty() {
            let _ = self.consume('$');
            let mut row_digits = String::new();
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    row_digits.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
            if !row_digits.is_empty() && is_boundary(self.peek()) && self.peek() != Some('!') {
                let text = self.slice(start, self.pos);
                return Ok(TokenKind::CellRef(text.to_string()));
            }
            // not a cell ref; fall through to identifier parsing
            self.pos = start;
        } else {
            self.pos = start;
        }

        let mut ident = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        Ok(TokenKind::Identifier(ident))
    }

    fn r1c1_cell_token(&mut self, start: usize) -> Option<TokenKind> {
        if !matches!(self.peek(), Some('R') | Some('r')) {
            return None;
        }

        let mut end = self.pos;
        let mut saw_c = false;
        while let Some(ch) = self.chars.get(end).map(|(_, ch)| *ch) {
            if is_delimiter(ch) && !matches!(ch, '[' | ']' | '+' | '-') {
                break;
            }
            if matches!(ch, 'C' | 'c') {
                saw_c = true;
            }
            if !matches!(
                ch,
                'R' | 'r' | 'C' | 'c' | '[' | ']' | '+' | '-' | '0'..='9'
            ) {
                return None;
            }
            end += 1;
        }

        if !saw_c {
            return None;
        }

        let text = self.slice(start, end).to_string();
        if R1C1Ref::from_r1c1(&text).is_ok() {
            self.pos = end;
            Some(TokenKind::CellRef(text))
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
            self.advance();
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|(_, ch)| *ch)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn byte_pos(&self, idx: usize) -> usize {
        self.chars
            .get(idx)
            .map(|(i, _)| *i)
            .unwrap_or(self.input.len())
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        &self.input[self.byte_pos(start)..self.byte_pos(end)]
    }
}

fn is_delimiter(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '(' | ')'
                | ','
                | ';'
                | '+'
                | '-'
                | '*'
                | '/'
                | '^'
                | '&'
                | '%'
                | ':'
                | '!'
                | '='
                | '<'
                | '>'
        )
}

fn is_error_delimiter(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '(' | ')' | ',' | ';' | '+' | '-' | '*' | '/' | '^' | '&' | '%' | ':' | '=' | '<' | '>'
        )
}

fn is_boundary(ch: Option<char>) -> bool {
    match ch {
        None => true,
        Some(ch) => is_delimiter(ch),
    }
}

struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, idx: 0 }
    }

    fn parse_expression(&mut self) -> Result<FormulaExpr, FormulaError> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expr = self.parse_concat()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Equal => BinaryOperator::Equal,
                TokenKind::NotEqual => BinaryOperator::NotEqual,
                TokenKind::Less => BinaryOperator::LessThan,
                TokenKind::LessEqual => BinaryOperator::LessThanOrEqual,
                TokenKind::Greater => BinaryOperator::GreaterThan,
                TokenKind::GreaterEqual => BinaryOperator::GreaterThanOrEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_concat()?;
            expr = FormulaExpr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_concat(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expr = self.parse_add_sub()?;
        while matches!(self.peek_kind(), TokenKind::Ampersand) {
            self.advance();
            let right = self.parse_add_sub()?;
            expr = FormulaExpr::BinaryOp {
                op: BinaryOperator::Concat,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_add_sub(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expr = self.parse_mul_div()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Subtract,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul_div()?;
            expr = FormulaExpr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_mul_div(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinaryOperator::Multiply,
                TokenKind::Slash => BinaryOperator::Divide,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            expr = FormulaExpr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_power(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expr = self.parse_postfix()?;
        if matches!(self.peek_kind(), TokenKind::Caret) {
            self.advance();
            let right = self.parse_unary()?;
            expr = FormulaExpr::BinaryOp {
                op: BinaryOperator::Power,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<FormulaExpr, FormulaError> {
        match self.peek_kind() {
            TokenKind::Plus => {
                self.advance();
                self.parse_unary()
            }
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(FormulaExpr::UnaryOp {
                    op: UnaryOperator::Negate,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_power(),
        }
    }

    fn parse_postfix(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expr = self.parse_primary()?;
        while matches!(self.peek_kind(), TokenKind::Percent) {
            self.advance();
            expr = FormulaExpr::UnaryOp {
                op: UnaryOperator::Percent,
                expr: Box::new(expr),
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<FormulaExpr, FormulaError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Number { value, is_int } => {
                if is_int {
                    Ok(FormulaExpr::Literal(Value::Int(value as i64)))
                } else {
                    Ok(FormulaExpr::Literal(Value::Float(value)))
                }
            }
            TokenKind::String(value) => Ok(FormulaExpr::Literal(Value::String(value))),
            TokenKind::Error(err) => Ok(FormulaExpr::Literal(Value::Error(err))),
            TokenKind::Identifier(name) => self.parse_identifier_or_ref(name),
            TokenKind::SheetName(name) => self.parse_sheet_ref(name),
            TokenKind::CellRef(text) => self.parse_cell_or_range(text, None),
            TokenKind::LParen => {
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::Eof => Err(FormulaError::ParseError(
                "Unexpected end of input".to_string(),
            )),
            _ => Err(FormulaError::ParseError(format!(
                "Unexpected token at {}",
                token.pos
            ))),
        }
    }

    fn parse_identifier_or_ref(&mut self, name: String) -> Result<FormulaExpr, FormulaError> {
        if matches!(self.peek_kind(), TokenKind::Bang) {
            return self.parse_sheet_ref(name);
        }

        if matches!(self.peek_kind(), TokenKind::LParen) {
            self.advance();
            let args = self.parse_arguments()?;
            return Ok(FormulaExpr::FunctionCall { name, args });
        }

        match name.to_uppercase().as_str() {
            "TRUE" => Ok(FormulaExpr::Literal(Value::Bool(true))),
            "FALSE" => Ok(FormulaExpr::Literal(Value::Bool(false))),
            _ => Err(FormulaError::ParseError(format!(
                "Unexpected identifier '{}'",
                name
            ))),
        }
    }

    fn parse_sheet_ref(&mut self, sheet: String) -> Result<FormulaExpr, FormulaError> {
        if !matches!(self.peek_kind(), TokenKind::Bang) {
            return Err(FormulaError::ParseError(format!(
                "Sheet name '{}' missing '!'.",
                sheet
            )));
        }
        self.advance();
        match self.advance().kind.clone() {
            TokenKind::CellRef(text) => self.parse_cell_or_range(text, Some(sheet)),
            other => Err(FormulaError::ParseError(format!(
                "Expected cell reference after sheet name, got {:?}",
                other
            ))),
        }
    }

    fn parse_cell_or_range(
        &mut self,
        first: String,
        sheet: Option<String>,
    ) -> Result<FormulaExpr, FormulaError> {
        let start = self.parse_cell_ref_text(&first)?;
        if matches!(self.peek_kind(), TokenKind::Colon) {
            self.advance();
            let end_text = match self.advance().kind.clone() {
                TokenKind::CellRef(text) => text,
                _ => {
                    return Err(FormulaError::ParseError(
                        "Expected cell reference after ':'".to_string(),
                    ))
                }
            };
            let end = self.parse_cell_ref_text(&end_text)?;
            match (start, end) {
                (ParsedCellRef::A1(start), ParsedCellRef::A1(end)) => {
                    let range = CellRange::new(start, end);
                    if let Some(sheet) = sheet {
                        Ok(FormulaExpr::SheetRangeRef { sheet, range })
                    } else {
                        Ok(FormulaExpr::RangeRef(range))
                    }
                }
                (ParsedCellRef::R1C1(start), ParsedCellRef::R1C1(end)) => {
                    if start.is_absolute() && end.is_absolute() {
                        let range = CellRange::new(
                            start
                                .resolve(None)
                                .map_err(|e| FormulaError::ParseError(e.to_string()))?,
                            end.resolve(None)
                                .map_err(|e| FormulaError::ParseError(e.to_string()))?,
                        );
                        if let Some(sheet) = sheet {
                            Ok(FormulaExpr::SheetRangeRef { sheet, range })
                        } else {
                            Ok(FormulaExpr::RangeRef(range))
                        }
                    } else if let Some(sheet) = sheet {
                        Ok(FormulaExpr::SheetR1C1RangeRef { sheet, start, end })
                    } else {
                        Ok(FormulaExpr::R1C1RangeRef { start, end })
                    }
                }
                _ => Err(FormulaError::ParseError(
                    "Mixed A1/R1C1 ranges are not supported".to_string(),
                )),
            }
        } else if let Some(sheet) = sheet {
            match start {
                ParsedCellRef::A1(addr) => Ok(FormulaExpr::SheetCellRef { sheet, addr }),
                ParsedCellRef::R1C1(addr) => {
                    if addr.is_absolute() {
                        let resolved = addr
                            .resolve(None)
                            .map_err(|e| FormulaError::ParseError(e.to_string()))?;
                        Ok(FormulaExpr::SheetCellRef {
                            sheet,
                            addr: resolved,
                        })
                    } else {
                        Ok(FormulaExpr::SheetR1C1Ref { sheet, addr })
                    }
                }
            }
        } else {
            match start {
                ParsedCellRef::A1(addr) => Ok(FormulaExpr::CellRef(addr)),
                ParsedCellRef::R1C1(addr) => {
                    if addr.is_absolute() {
                        let resolved = addr
                            .resolve(None)
                            .map_err(|e| FormulaError::ParseError(e.to_string()))?;
                        Ok(FormulaExpr::CellRef(resolved))
                    } else {
                        Ok(FormulaExpr::R1C1Ref(addr))
                    }
                }
            }
        }
    }

    fn parse_cell_ref_text(&self, text: &str) -> Result<ParsedCellRef, FormulaError> {
        if let Ok(addr) = CellAddress::from_a1(text) {
            return Ok(ParsedCellRef::A1(addr));
        }
        let r1c1 = R1C1Ref::from_r1c1(text)
            .map_err(|e| FormulaError::ParseError(format!("Invalid cell ref: {}", e)))?;
        Ok(ParsedCellRef::R1C1(r1c1))
    }

    fn parse_arguments(&mut self) -> Result<Vec<FormulaExpr>, FormulaError> {
        let mut args = Vec::new();
        if matches!(self.peek_kind(), TokenKind::RParen) {
            self.advance();
            return Ok(args);
        }
        loop {
            let expr = self.parse_expression()?;
            args.push(expr);
            match self.peek_kind() {
                TokenKind::Comma | TokenKind::Semicolon => {
                    self.advance();
                    continue;
                }
                TokenKind::RParen => {
                    self.advance();
                    break;
                }
                _ => {
                    return Err(FormulaError::ParseError(
                        "Expected ',' or ')' in argument list".to_string(),
                    ))
                }
            }
        }
        Ok(args)
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), FormulaError> {
        let token = self.advance();
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(&kind) {
            Ok(())
        } else {
            Err(FormulaError::ParseError(format!(
                "Expected {:?}, got {:?}",
                kind, token.kind
            )))
        }
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.idx].kind
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.idx];
        if !matches!(token.kind, TokenKind::Eof) {
            self.idx += 1;
        }
        token
    }
}

fn parse_error_literal(literal: &str) -> Result<ErrorValue, FormulaError> {
    match literal.to_uppercase().as_str() {
        "#DIV/0!" => Ok(ErrorValue::Div0),
        "#NAME?" => Ok(ErrorValue::Name),
        "#VALUE!" => Ok(ErrorValue::Value),
        "#REF!" => Ok(ErrorValue::Ref),
        "#NULL!" => Ok(ErrorValue::Null),
        "#NUM!" => Ok(ErrorValue::Num),
        "#N/A" => Ok(ErrorValue::NA),
        _ => Err(FormulaError::ParseError(format!(
            "Unknown error literal '{}'",
            literal
        ))),
    }
}

/// Parse a formula string into an AST
pub fn parse_formula(formula: &str) -> Result<FormulaExpr, FormulaError> {
    let formula = formula.trim_start();
    let formula = formula.strip_prefix('=').unwrap_or(formula);
    if formula.trim().is_empty() {
        return Err(FormulaError::ParseError("Empty formula".to_string()));
    }

    let mut lexer = Lexer::new(formula);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token()?;
        let is_eof = matches!(token.kind, TokenKind::Eof);
        tokens.push(token);
        if is_eof {
            break;
        }
    }

    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expression()?;
    if !matches!(parser.peek_kind(), TokenKind::Eof) {
        return Err(FormulaError::ParseError(
            "Unexpected trailing input".to_string(),
        ));
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        assert!(parse_formula("").is_err());
        assert!(parse_formula("=").is_err());
    }

    #[test]
    fn test_parse_numbers_and_ops() {
        let expr = parse_formula("=1+2*3").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::Add,
                ..
            }
        ));

        let expr = parse_formula("=6/2").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::Divide,
                ..
            }
        ));

        let expr = parse_formula("=3-1").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::Subtract,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_exponent_precedence() {
        let expr = parse_formula("-2^2").unwrap();
        assert!(
            matches!(expr, FormulaExpr::UnaryOp { op: UnaryOperator::Negate, expr } if matches!(*expr, FormulaExpr::BinaryOp { op: BinaryOperator::Power, .. }))
        );

        let expr = parse_formula("2^-3").unwrap();
        assert!(
            matches!(expr, FormulaExpr::BinaryOp { op: BinaryOperator::Power, right, .. } if matches!(*right, FormulaExpr::UnaryOp { op: UnaryOperator::Negate, .. }))
        );
    }

    #[test]
    fn test_parse_cell_and_range() {
        let expr = parse_formula("A1").unwrap();
        assert!(matches!(expr, FormulaExpr::CellRef(_)));
        let expr = parse_formula("A1:B2").unwrap();
        assert!(matches!(expr, FormulaExpr::RangeRef(_)));
    }

    #[test]
    fn test_parse_sheet_ref() {
        let expr = parse_formula("Sheet1!A1").unwrap();
        assert!(matches!(expr, FormulaExpr::SheetCellRef { .. }));
        let expr = parse_formula("'My Sheet'!A1:B2").unwrap();
        assert!(matches!(expr, FormulaExpr::SheetRangeRef { .. }));
    }

    #[test]
    fn test_parse_r1c1_cell_and_range() {
        let expr = parse_formula("R1C1").unwrap();
        assert!(matches!(expr, FormulaExpr::CellRef(_)));
        let expr = parse_formula("R1C1:R2C2").unwrap();
        assert!(matches!(expr, FormulaExpr::RangeRef(_)));
        let expr = parse_formula("R[1]C[1]").unwrap();
        assert!(matches!(expr, FormulaExpr::R1C1Ref(_)));
        let expr = parse_formula("Sheet1!R[-1]C:R[1]C").unwrap();
        assert!(matches!(expr, FormulaExpr::SheetR1C1RangeRef { .. }));
    }

    #[test]
    fn test_parse_function_call() {
        let expr = parse_formula("SUM(A1,2,3)").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::FunctionCall { ref name, ref args }
                if name == "SUM" && args.len() == 3
        ));

        let expr = parse_formula("SUM()").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::FunctionCall { ref args, .. } if args.is_empty()
        ));
    }

    #[test]
    fn test_parse_literals_and_concat() {
        let expr = parse_formula("\"hello\"").unwrap();
        assert!(matches!(expr, FormulaExpr::Literal(Value::String(s)) if s == "hello"));

        let expr = parse_formula("TRUE").unwrap();
        assert!(matches!(expr, FormulaExpr::Literal(Value::Bool(true))));

        let expr = parse_formula("#VALUE!").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::Literal(Value::Error(ErrorValue::Value))
        ));

        let expr = parse_formula("\"a\"&\"b\"").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::Concat,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_semicolon_args_and_percent() {
        let expr = parse_formula("SUM(1;2;3)").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::FunctionCall { ref args, .. } if args.len() == 3
        ));

        let expr = parse_formula("5%").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::UnaryOp {
                op: UnaryOperator::Percent,
                expr,
            } if matches!(*expr, FormulaExpr::Literal(Value::Int(5)))
        ));

        let expr = parse_formula("10%%").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::UnaryOp {
                op: UnaryOperator::Percent,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_leading_whitespace() {
        let expr = parse_formula(" =1+2").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::Add,
                ..
            }
        ));

        let expr = parse_formula("A1 ").unwrap();
        assert!(matches!(expr, FormulaExpr::CellRef(_)));
    }

    #[test]
    fn test_parse_string_escapes() {
        let expr = parse_formula("\"a\"\"b\"").unwrap();
        assert!(matches!(expr, FormulaExpr::Literal(Value::String(s)) if s == "a\"b"));
    }

    #[test]
    fn test_parse_sheet_name_escape() {
        let expr = parse_formula("'My''Sheet'!A1").unwrap();
        assert!(matches!(expr, FormulaExpr::SheetCellRef { sheet, .. } if sheet == "My'Sheet"));
    }

    #[test]
    fn test_parse_error_literals() {
        assert!(matches!(
            parse_formula("#VALUE!").unwrap(),
            FormulaExpr::Literal(Value::Error(ErrorValue::Value))
        ));

        assert!(matches!(
            parse_error_literal("#DIV/0!"),
            Ok(ErrorValue::Div0)
        ));
        assert!(matches!(
            parse_error_literal("#NAME?"),
            Ok(ErrorValue::Name)
        ));
        assert!(matches!(parse_error_literal("#REF!"), Ok(ErrorValue::Ref)));
        assert!(matches!(
            parse_error_literal("#NULL!"),
            Ok(ErrorValue::Null)
        ));
        assert!(matches!(parse_error_literal("#NUM!"), Ok(ErrorValue::Num)));
        assert!(matches!(parse_error_literal("#N/A"), Ok(ErrorValue::NA)));

        let expr = parse_formula("#VALUE!+1").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::Add,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_comparisons() {
        let expr = parse_formula("1=2").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::Equal,
                ..
            }
        ));

        let expr = parse_formula("1<>2").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::NotEqual,
                ..
            }
        ));

        let expr = parse_formula("1<2").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::LessThan,
                ..
            }
        ));

        let expr = parse_formula("1<=2").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::LessThanOrEqual,
                ..
            }
        ));

        let expr = parse_formula("1>2").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::GreaterThan,
                ..
            }
        ));

        let expr = parse_formula("1>=2").unwrap();
        assert!(matches!(
            expr,
            FormulaExpr::BinaryOp {
                op: BinaryOperator::GreaterThanOrEqual,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_errors() {
        assert!(parse_formula("FOO").is_err());
        assert!(parse_formula("\"unterminated").is_err());
        assert!(parse_formula("'Sheet").is_err());
    }

    #[test]
    fn test_parse_invalid_error_literal() {
        assert!(parse_error_literal("#NOPE").is_err());
        assert!(parse_formula("#NOPE").is_err());
    }

    #[test]
    fn test_parse_sheet_ref_errors() {
        assert!(parse_formula("Sheet1!1").is_err());
        assert!(parse_formula("'Sheet1'").is_err());
    }

    #[test]
    fn test_parse_unexpected_trailing_input() {
        assert!(parse_formula("1 2").is_err());
        assert!(parse_formula("1,,2").is_err());
        assert!(parse_formula("SUM(1 2)").is_err());
        assert!(parse_formula("(1").is_err());
        assert!(parse_formula("()").is_err());
    }

    #[test]
    fn test_parse_unexpected_character() {
        assert!(parse_formula("1@2").is_err());
    }

    #[test]
    fn test_parse_invalid_numeric_exponent() {
        assert!(parse_formula("1e").is_err());
        assert!(parse_formula("1e+").is_err());
    }

    #[test]
    fn test_parse_number_formats() {
        let expr = parse_formula("1.5").unwrap();
        assert!(matches!(expr, FormulaExpr::Literal(Value::Float(f)) if (f - 1.5).abs() < 1e-9));

        let expr = parse_formula("1e-2").unwrap();
        assert!(matches!(expr, FormulaExpr::Literal(Value::Float(f)) if (f - 0.01).abs() < 1e-9));

        let expr = parse_formula("+1").unwrap();
        assert!(matches!(expr, FormulaExpr::Literal(Value::Int(1))));
    }

    #[test]
    fn test_parse_absolute_cell_ref() {
        let expr = parse_formula("$A$1").unwrap();
        assert!(matches!(expr, FormulaExpr::CellRef(_)));
    }

    #[test]
    fn test_parse_range_error() {
        assert!(parse_formula("A1:").is_err());
        assert!(parse_formula(")").is_err());
    }

    #[test]
    fn test_parse_identifier_with_dot() {
        assert!(parse_formula("foo.bar").is_err());
        assert!(parse_formula("_foo").is_err());
        assert!(parse_formula("A1B").is_err());
    }

    #[test]
    fn test_parse_parenthesized() {
        let expr = parse_formula("(1)").unwrap();
        assert!(matches!(expr, FormulaExpr::Literal(Value::Int(1))));
    }

    #[test]
    fn test_lexer_cell_ref_token() {
        let mut lexer = Lexer::new("A1");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.kind, TokenKind::CellRef(_)));
    }

    #[test]
    fn test_parser_eof_error() {
        let tokens = vec![Token {
            kind: TokenKind::Eof,
            pos: 0,
        }];
        let mut parser = Parser::new(tokens);
        assert!(parser.parse_expression().is_err());
    }

    #[test]
    fn test_parse_unicode_literals() {
        let expr = parse_formula("\"你好\"").unwrap();
        assert!(matches!(expr, FormulaExpr::Literal(Value::String(s)) if s == "你好"));

        let expr = parse_formula("'シート'!A1").unwrap();
        assert!(matches!(expr, FormulaExpr::SheetCellRef { sheet, .. } if sheet == "シート"));
    }
}
