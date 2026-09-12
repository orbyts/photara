use super::expression::{Binary, FieldMode, Unary};
use super::value::{Integer, Value};
use super::{AST_DEPTH, AST_NODES, ErrorCode, Result, Span, at};
use crate::contracts::schema::LocalName;
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub(crate) struct Raw {
    pub span: Span,
    pub kind: RawKind,
}
#[derive(Clone, Debug)]
pub(crate) enum RawKind {
    Literal(Value),
    Reference(String),
    Binary(Binary, Box<Raw>, Box<Raw>),
    Unary(Unary, Box<Raw>),
    If(Box<Raw>, Box<Raw>, Box<Raw>),
    Call(String, Vec<Raw>),
    Template(Vec<Raw>),
    List(Vec<Raw>),
    Record(BTreeMap<LocalName, Raw>),
    Field(Box<Raw>, LocalName),
}
fn literal(value: Value, start: usize, end: usize) -> Raw {
    Raw {
        span: Span { start, end },
        kind: RawKind::Literal(value),
    }
}
pub(crate) fn parse(mode: FieldMode, source: &str) -> Result<Raw> {
    if mode == FieldMode::LiteralOnly {
        return Ok(literal(Value::String(source.into()), 0, source.len()));
    }
    if source.contains("```") {
        return Err(at(
            ErrorCode::UnsupportedVersion,
            source.find("```").unwrap_or(0),
            source.find("```").unwrap_or(0) + 3,
        ));
    }
    if mode == FieldMode::Expression {
        let trimmed = source.trim();
        let start = source.len() - source.trim_start().len();
        if !trimmed.starts_with('`') {
            return Err(at(ErrorCode::Syntax, start, start));
        }
        let close = source[start + 1..]
            .find('`')
            .map(|n| n + start + 1)
            .ok_or_else(|| at(ErrorCode::Syntax, start, source.len()))?;
        if !source[close + 1..].trim().is_empty() {
            return Err(at(ErrorCode::Syntax, close + 1, source.len()));
        }
        return Parser::new(source, start + 1, close)?.complete();
    }
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut start = 0;
    let mut i = 0;
    while i < source.len() {
        let c = source[i..].chars().next().ok_or(ErrorCode::Syntax)?;
        match c {
            '\\' => {
                let next = source[i + 1..]
                    .chars()
                    .next()
                    .ok_or_else(|| at(ErrorCode::Syntax, i, i + 1))?;
                if !matches!(next, '\\' | '`') {
                    return Err(at(ErrorCode::Syntax, i, i + 1 + next.len_utf8()));
                }
                text.push(next);
                i += 1 + next.len_utf8();
            }
            '`' => {
                if !text.is_empty() {
                    parts.push(literal(Value::String(std::mem::take(&mut text)), start, i));
                }
                let close = source[i + 1..]
                    .find('`')
                    .map(|n| n + i + 1)
                    .ok_or_else(|| at(ErrorCode::Syntax, i, source.len()))?;
                parts.push(Parser::new(source, i + 1, close)?.complete()?);
                i = close + 1;
                start = i;
            }
            '\n' | '\r' => return Err(at(ErrorCode::UnsupportedVersion, i, i + c.len_utf8())),
            _ => {
                text.push(c);
                i += c.len_utf8();
            }
        }
        if parts.len() > AST_NODES {
            return Err(ErrorCode::LimitExceeded.into());
        }
    }
    if !text.is_empty() {
        parts.push(literal(Value::String(text), start, i));
    }
    Ok(Raw {
        span: Span {
            start: 0,
            end: source.len(),
        },
        kind: RawKind::Template(parts),
    })
}
#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    String(String),
    Number(String),
    Word(String),
    Reference(String),
    Symbol(char),
    Equal,
    NotEqual,
    LessEqual,
    GreaterEqual,
    End,
}
#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    span: Span,
}
struct Parser {
    tokens: Vec<Token>,
    at: usize,
    nodes: usize,
}
impl Parser {
    fn new(source: &str, start: usize, end: usize) -> Result<Self> {
        let mut tokens = Vec::new();
        let mut i = start;
        while i < end {
            let c = source[i..].chars().next().ok_or(ErrorCode::Syntax)?;
            if c.is_ascii_whitespace() {
                i += 1;
                continue;
            }
            let begin = i;
            let kind = if c == '"' {
                i += 1;
                let mut closed = false;
                while i < end {
                    let b = source.as_bytes()[i];
                    if b == b'"' {
                        i += 1;
                        closed = true;
                        break;
                    }
                    if b == b'\\' {
                        i += 1;
                        if i >= end {
                            break;
                        }
                    }
                    i += 1;
                }
                if !closed {
                    return Err(at(ErrorCode::Syntax, begin, end));
                }
                let s = serde_json::from_str::<String>(&source[begin..i])
                    .map_err(|_| at(ErrorCode::Syntax, begin, i))?;
                TokenKind::String(s)
            } else if c.is_ascii_digit() {
                i += 1;
                while i < end && source.as_bytes()[i].is_ascii_digit() {
                    i += 1;
                }
                TokenKind::Number(source[begin..i].into())
            } else if c == '$' {
                i += 1;
                while i < end
                    && (source.as_bytes()[i].is_ascii_alphanumeric()
                        || matches!(source.as_bytes()[i], b'_' | b'.'))
                {
                    i += 1;
                }
                TokenKind::Reference(source[begin..i].into())
            } else if c.is_ascii_lowercase() {
                i += 1;
                while i < end
                    && (source.as_bytes()[i].is_ascii_lowercase()
                        || source.as_bytes()[i].is_ascii_digit()
                        || matches!(source.as_bytes()[i], b'_' | b'.'))
                {
                    i += 1;
                }
                TokenKind::Word(source[begin..i].into())
            } else {
                i += c.len_utf8();
                match c {
                    '=' | '!' | '<' | '>' if i < end && source.as_bytes()[i] == b'=' => {
                        i += 1;
                        match c {
                            '=' => TokenKind::Equal,
                            '!' => TokenKind::NotEqual,
                            '<' => TokenKind::LessEqual,
                            _ => TokenKind::GreaterEqual,
                        }
                    }
                    '(' | ')' | '[' | ']' | '{' | '}' | ',' | ':' | '+' | '-' | '*' | '<' | '>'
                    | '.' => TokenKind::Symbol(c),
                    _ => return Err(at(ErrorCode::Syntax, begin, i)),
                }
            };
            tokens.push(Token {
                kind,
                span: Span {
                    start: begin,
                    end: i,
                },
            });
            if tokens.len() > AST_NODES * 4 {
                return Err(ErrorCode::LimitExceeded.into());
            }
        }
        tokens.push(Token {
            kind: TokenKind::End,
            span: Span { start: end, end },
        });
        Ok(Self {
            tokens,
            at: 0,
            nodes: 0,
        })
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.at]
    }
    fn take(&mut self) -> Token {
        let t = self.peek().clone();
        if self.at + 1 < self.tokens.len() {
            self.at += 1;
        }
        t
    }
    fn expect(&mut self, kind: &TokenKind) -> Result<()> {
        let t = self.take();
        if &t.kind != kind {
            return Err(at(ErrorCode::Syntax, t.span.start, t.span.end));
        }
        Ok(())
    }
    fn complete(mut self) -> Result<Raw> {
        let r = self.expr(0, 0)?;
        self.expect(&TokenKind::End)?;
        Ok(r)
    }
    fn node(&mut self, span: Span, kind: RawKind) -> Result<Raw> {
        self.nodes += 1;
        if self.nodes > AST_NODES {
            return Err(ErrorCode::LimitExceeded.into());
        }
        Ok(Raw { span, kind })
    }
    // Keep the bounded grammar dispatch exhaustive and auditable in one match.
    #[allow(clippy::too_many_lines)]
    fn expr(&mut self, min: u8, depth: usize) -> Result<Raw> {
        if depth >= AST_DEPTH {
            return Err(ErrorCode::LimitExceeded.into());
        }
        let token = self.take();
        let mut left = match token.kind {
            TokenKind::String(s) => self.node(token.span, RawKind::Literal(Value::String(s)))?,
            TokenKind::Number(s) => self.node(
                token.span,
                RawKind::Literal(Value::Integer(Integer::try_from(s)?)),
            )?,
            TokenKind::Reference(s) => self.node(token.span, RawKind::Reference(s))?,
            TokenKind::Word(ref s) if s == "true" || s == "false" => {
                self.node(token.span, RawKind::Literal(Value::Bool(s == "true")))?
            }
            TokenKind::Word(ref s) if s == "null" => {
                self.node(token.span, RawKind::Literal(Value::Null))?
            }
            TokenKind::Word(ref s) if s == "not" => {
                let v = self.expr(6, depth + 1)?;
                self.node(
                    Span {
                        start: token.span.start,
                        end: v.span.end,
                    },
                    RawKind::Unary(Unary::Not, Box::new(v)),
                )?
            }
            TokenKind::Symbol('-') => {
                if let TokenKind::Number(n) = self.peek().kind.clone() {
                    let t = self.take();
                    self.node(
                        Span {
                            start: token.span.start,
                            end: t.span.end,
                        },
                        RawKind::Literal(Value::Integer(Integer::try_from(format!("-{n}"))?)),
                    )?
                } else {
                    let v = self.expr(6, depth + 1)?;
                    self.node(
                        Span {
                            start: token.span.start,
                            end: v.span.end,
                        },
                        RawKind::Unary(Unary::Negate, Box::new(v)),
                    )?
                }
            }
            TokenKind::Symbol('(') => {
                let mut r = self.expr(0, depth + 1)?;
                self.expect(&TokenKind::Symbol(')'))?;
                r.span = Span {
                    start: token.span.start,
                    end: self.tokens[self.at - 1].span.end,
                };
                r
            }
            TokenKind::Word(name) => {
                self.expect(&TokenKind::Symbol('('))?;
                let mut args = Vec::new();
                if self.peek().kind != TokenKind::Symbol(')') {
                    loop {
                        args.push(self.expr(0, depth + 1)?);
                        if self.peek().kind != TokenKind::Symbol(',') {
                            break;
                        }
                        self.take();
                    }
                }
                self.expect(&TokenKind::Symbol(')'))?;
                let span = Span {
                    start: token.span.start,
                    end: self.tokens[self.at - 1].span.end,
                };
                if name == "if" {
                    if args.len() != 3 {
                        return Err(at(ErrorCode::Syntax, span.start, span.end));
                    }
                    let mut i = args.into_iter();
                    self.node(
                        span,
                        RawKind::If(
                            Box::new(i.next().ok_or(ErrorCode::Syntax)?),
                            Box::new(i.next().ok_or(ErrorCode::Syntax)?),
                            Box::new(i.next().ok_or(ErrorCode::Syntax)?),
                        ),
                    )?
                } else {
                    self.node(span, RawKind::Call(name, args))?
                }
            }
            TokenKind::Symbol('[') => {
                let mut items = Vec::new();
                if self.peek().kind != TokenKind::Symbol(']') {
                    loop {
                        items.push(self.expr(0, depth + 1)?);
                        if self.peek().kind != TokenKind::Symbol(',') {
                            break;
                        }
                        self.take();
                    }
                }
                self.expect(&TokenKind::Symbol(']'))?;
                self.node(
                    Span {
                        start: token.span.start,
                        end: self.tokens[self.at - 1].span.end,
                    },
                    RawKind::List(items),
                )?
            }
            TokenKind::Symbol('{') => {
                let mut fields = BTreeMap::new();
                if self.peek().kind != TokenKind::Symbol('}') {
                    loop {
                        let key = self.take();
                        let TokenKind::String(key) = key.kind else {
                            return Err(at(ErrorCode::Syntax, key.span.start, key.span.end));
                        };
                        let key = LocalName::parse(key)?;
                        self.expect(&TokenKind::Symbol(':'))?;
                        if fields.insert(key, self.expr(0, depth + 1)?).is_some() {
                            return Err(ErrorCode::Conflict.into());
                        }
                        if self.peek().kind != TokenKind::Symbol(',') {
                            break;
                        }
                        self.take();
                    }
                }
                self.expect(&TokenKind::Symbol('}'))?;
                self.node(
                    Span {
                        start: token.span.start,
                        end: self.tokens[self.at - 1].span.end,
                    },
                    RawKind::Record(fields),
                )?
            }
            _ => return Err(at(ErrorCode::Syntax, token.span.start, token.span.end)),
        };
        loop {
            if self.peek().kind == TokenKind::Symbol('.') {
                self.take();
                let field = self.take();
                let TokenKind::Word(name) = field.kind else {
                    return Err(at(ErrorCode::Syntax, field.span.start, field.span.end));
                };
                for name in name.split('.') {
                    let span = Span {
                        start: left.span.start,
                        end: field.span.end,
                    };
                    left = self.node(
                        span,
                        RawKind::Field(Box::new(left), LocalName::parse(name)?),
                    )?;
                }
                continue;
            }
            let pair = match &self.peek().kind {
                TokenKind::Word(w) if w == "or" => Some((1, Binary::Or)),
                TokenKind::Word(w) if w == "and" => Some((2, Binary::And)),
                TokenKind::Equal => Some((3, Binary::Equal)),
                TokenKind::NotEqual => Some((3, Binary::NotEqual)),
                TokenKind::Symbol('<') => Some((3, Binary::Less)),
                TokenKind::Symbol('>') => Some((3, Binary::Greater)),
                TokenKind::LessEqual => Some((3, Binary::LessEqual)),
                TokenKind::GreaterEqual => Some((3, Binary::GreaterEqual)),
                TokenKind::Symbol('+') => Some((4, Binary::Add)),
                TokenKind::Symbol('-') => Some((4, Binary::Subtract)),
                TokenKind::Symbol('*') => Some((5, Binary::Multiply)),
                _ => None,
            };
            let Some((precedence, operator)) = pair else {
                break;
            };
            if precedence < min {
                break;
            }
            self.take();
            let right = self.expr(precedence + 1, depth + 1)?;
            let span = Span {
                start: left.span.start,
                end: right.span.end,
            };
            left = self.node(
                span,
                RawKind::Binary(operator, Box::new(left), Box::new(right)),
            )?;
        }
        Ok(left)
    }
}
