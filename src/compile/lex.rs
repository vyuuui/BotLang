use std::fmt;
use std::str;
use strum_macros::{EnumDiscriminants, Display};

use crate::compile::{SourceLoc, LocationAnnot, SL_BEGIN, SL_NIL, CompileErr};

#[derive(Clone, PartialEq, Debug, EnumDiscriminants)]
#[strum_discriminants(derive(Display))]
#[strum_discriminants(vis(pub))]
pub enum Token {
    // Symbolic Tokens
    LParen,          // (
    RParen,          // )
    LCurly,          // {
    RCurly,          // }
    LBracket,        // [
    RBracket,        // ]
    LAngle,          // <
    RAngle,          // >
    Ampersat,        // @
    Ampersand,       // &
    Pipe,            // |
    Plus,            // +
    Dash,            // -
    Asterisk,        // *
    FSlash,          // /
    Percent,         // %
    DoubleAmpersand, // &&
    DoublePipe,      // ||
    Bang,            // !
    Caret,           // ^
    Tilde,           // ~
    LLAngle,         // <<
    RRAngle,         // >>
    DoubleEqual,     // ==
    BangEqual,       // !=
    LAngleEqual,     // <=
    RAngleEqual,     // >=
    Arrow,           // ->
    Semicolon,       // ;
    Colon,           // :
    DoubleColon,     // ::
    Ellipsis,        // ..
    Comma,           // ,
    Period,          // .
    Equal,           // =
    PlusEqual,       // +=
    DashEqual,       // -=
    AsteriskEqual,   // *=
    FSlashEqual,     // /=
    PercentEqual,    // %=
    AmpersandEqual,  // &=
    CaretEqual,      // ^=
    PipeEqual,       // |=
    TildeEqual,      // ~=
    LLAngleEqual,    // <<=
    RRAngleEqual,    // >>=

    // Literal Tokens
    IntegralLiteral(u64),
    FloatLiteral(f32),
    DoubleLiteral(f64),
    StringLiteral(Vec<u8>),
    TimeLiteral(u64),

    // Other Tokens
    Identifier(String),
    Invalid,
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LCurly => write!(f, "{{"),
            Token::RCurly => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LAngle => write!(f, "<"),
            Token::RAngle => write!(f, ">"),
            Token::Ampersat => write!(f, "@"),
            Token::Ampersand => write!(f, "&"),
            Token::Pipe => write!(f, "|"),
            Token::Plus => write!(f, "+"),
            Token::Dash => write!(f, "-"),
            Token::Asterisk => write!(f, "*"),
            Token::FSlash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::DoubleAmpersand => write!(f, "&&"),
            Token::DoublePipe => write!(f, "||"),
            Token::Bang => write!(f, "!"),
            Token::Caret => write!(f, "^"),
            Token::Tilde => write!(f, "~"),
            Token::LLAngle => write!(f, "<<"),
            Token::RRAngle => write!(f, ">>"),
            Token::DoubleEqual => write!(f, "=="),
            Token::BangEqual => write!(f, "!="),
            Token::LAngleEqual => write!(f, "<="),
            Token::RAngleEqual => write!(f, ">="),
            Token::Arrow => write!(f, "->"),
            Token::Semicolon => write!(f, ";"),
            Token::Colon => write!(f, ":"),
            Token::DoubleColon => write!(f, "::"),
            Token::Ellipsis => write!(f, ".."),
            Token::Comma => write!(f, ","),
            Token::Period => write!(f, "."),
            Token::Equal => write!(f, "="),
            Token::PlusEqual => write!(f, "+="),
            Token::DashEqual => write!(f, "-="),
            Token::AsteriskEqual => write!(f, "*="),
            Token::FSlashEqual => write!(f, "/="),
            Token::PercentEqual => write!(f, "%="),
            Token::AmpersandEqual => write!(f, "&="),
            Token::CaretEqual => write!(f, "^="),
            Token::PipeEqual => write!(f, "|="),
            Token::TildeEqual => write!(f, "~="),
            Token::LLAngleEqual => write!(f, "<<="),
            Token::RRAngleEqual => write!(f, ">>="),

            Token::IntegralLiteral(v) => write!(f, "U64 {}", v),
            Token::FloatLiteral(v) => write!(f, "Float {}", v),
            Token::DoubleLiteral(v) => write!(f, "Double {}", v),
            Token::StringLiteral(v) => {
                // Valid UTF-8 strings can just be printed raw
                if let Ok(s) = str::from_utf8(v) {
                    write!(f, "String {}", s)
                } else {
                    // Strings that aren't UTF-8 just print the raw bytes
                    write!(f, "Raw string ")?;
                    v.iter().try_for_each(|b| write!(f, "{:02x} ", *b))
                }
            }
            Token::TimeLiteral(v) => write!(f, "TODO: time {}", v),

            Token::Identifier(v) => write!(f, "Identifier {}", v),
            Token::Invalid => write!(f, "Invalid"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

pub type AnnotTok = LocationAnnot<Token>;

impl Clone for AnnotTok {
    fn clone(&self) -> AnnotTok {
        AnnotTok {
            loc: self.loc,
            inner: self.inner.clone(),
        }
    }
}

pub struct Lex {
    source: String,

    peek_buf: Vec<AnnotTok>,
    head_idx: usize,

    cursor: SourceLoc,
    seek: SourceLoc,

    err: Option<CompileErr>,

    mark_data: (SourceLoc, usize),
}

impl Lex {
    pub fn new(source: String) -> Lex {
        Self {
            source,
            peek_buf: Vec::new(),
            head_idx: 0,
            cursor: SL_BEGIN,
            seek: SL_BEGIN,
            err: None,
            mark_data: (SL_BEGIN, 0),
        }
    }

    fn peek_char(&self) -> Option<char> {
        // off encodes a byte index to the next valid utf8 codepoint, this is fine
        self.source[self.seek.off..].chars().next()
    }

    fn peek_2char(&self) -> (Option<char>, Option<char>) {
        let mut iter = self.source[self.seek.off..].chars();
        (iter.next(), iter.next())
    }

    fn skip_comment(&mut self) {
        for ch in self.source[self.seek.off..].chars() {
            self.seek.next(ch);
            if ch == '\n' {
                break;
            }
        }
    }

    fn skip_ws(&mut self) {
        loop {
            // Actual whitespace
            for ch in self.source[self.seek.off..].chars() {
                if !ch.is_whitespace() {
                    break;
                }

                self.seek.next(ch);
            }

            // Comments
            let done = match self.peek_2char() {
                (Some('/'), Some('/')) => {
                    self.skip_comment();
                    self.peek_char().is_some_and(|x| !x.is_whitespace())
                }
                (Some(ch), _) => !ch.is_whitespace(),
                _ => true,
            };

            if done {
                break;
            }
        }
    }

    fn lex_ident(&mut self, first: char) -> Result<AnnotTok, CompileErr> {
        self.seek.next(first);

        for ch in self.source[self.seek.off..].chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                break;
            }

            self.seek.next(ch);
        }

        Ok(AnnotTok::new(
            self.cursor.to(&self.seek),
            Token::Identifier(self.source[self.cursor.off..self.seek.off].to_string()),
        ))
    }

    fn lex_numlit(&mut self, first: char) -> Result<AnnotTok, CompileErr> {
        self.seek.next(first);

        let safe_accum = |val: u64, num: u64, of: bool, radix: u64| -> (u64, bool) {
            let (val2, of_mul) = val.overflowing_mul(radix);
            let (val3, of_add) = val2.overflowing_add(num);
            (val3, of || of_mul || of_add)
        };

        let mut ival: u64 = 0;
        let mut did_overflow: bool = false;
        // Instead of doing a dfa here, easier to just write this one out
        if first == '0' {
            match self.peek_char() {
                Some('x') => {
                    self.seek.next('x');
                    for ch in self.source[self.seek.off..].chars() {
                        if ch.is_ascii_hexdigit() {
                            (ival, did_overflow) =
                                safe_accum(ival, x2i(ch as u8), did_overflow, 16);
                        } else {
                            break;
                        }
                        self.seek.next(ch);
                    }
                }
                Some('b') => {
                    self.seek.next('b');
                    for ch in self.source[self.seek.off..].chars() {
                        if is_ascii_binary(ch) {
                            (ival, did_overflow) = safe_accum(ival, d2i(ch as u8), did_overflow, 2);
                        } else {
                            break;
                        }
                        self.seek.next(ch);
                    }
                }
                Some(ch) if ch.is_ascii_digit() => {
                    for ch in self.source[self.seek.off..].chars() {
                        if is_ascii_octal(ch) {
                            (ival, did_overflow) = safe_accum(ival, d2i(ch as u8), did_overflow, 8);
                        } else {
                            break;
                        }
                        self.seek.next(ch);
                    }
                }
                // Empty or non-digit means standalone 0, which ival already is set with
                _ => (),
            }
            if did_overflow {
                Err(CompileErr::IntOverflow(self.cursor.to(&self.seek)))
            } else {
                Ok(AnnotTok::new(
                    self.cursor.to(&self.seek),
                    Token::IntegralLiteral(ival),
                ))
            }
        } else {
            // Can't overflow on first digit
            ival = d2i(first as u8);
            let mut has_fpart: bool = false;
            for ch in self.source[self.seek.off..].chars() {
                if ch == '.' {
                    has_fpart = true;
                }
                if ch.is_ascii_digit() {
                    (ival, did_overflow) = safe_accum(ival, d2i(ch as u8), did_overflow, 10);
                } else {
                    break;
                }
                self.seek.next(ch);
            }

            if has_fpart {
                let mut is_float: bool = false;
                self.seek.next('.');
                for ch in self.source[self.seek.off..].chars() {
                    if !ch.is_ascii_digit() {
                        is_float = ch == 'f';
                        break;
                    }
                    self.seek.next(ch);
                }
                if is_float {
                    let fval: f32 = self.source[self.cursor.off..self.seek.off]
                        .parse::<f32>()
                        .expect("Failed to parse well-formatted f32");
                    self.seek.next('f');
                    Ok(AnnotTok::new(
                        self.cursor.to(&self.seek),
                        Token::FloatLiteral(fval),
                    ))
                } else {
                    let dval: f64 = self.source[self.cursor.off..self.seek.off]
                        .parse::<f64>()
                        .expect("Failed to parse well-formatted f64");
                    Ok(AnnotTok::new(
                        self.cursor.to(&self.seek),
                        Token::DoubleLiteral(dval),
                    ))
                }
            } else if did_overflow {
                Err(CompileErr::IntOverflow(self.cursor.to(&self.seek)))
            } else {
                Ok(AnnotTok::new(
                    self.cursor.to(&self.seek),
                    Token::IntegralLiteral(ival),
                ))
            }
        }
    }

    fn lex_stringlit(&mut self) -> Result<AnnotTok, CompileErr> {
        self.seek.next('"');

        let mut str_build: Vec<u8> = Vec::new();
        let mut iter = self.source[self.seek.off..].chars();
        while let Some(ch) = iter.next() {
            if ch == '\\' {
                let escape_start = self.seek;

                self.seek.next(ch);
                let escape = iter.next();
                if let Some(x) = escape {
                    self.seek.next(x);
                }

                match escape {
                    Some('\\') => str_build.push(b'\\'),
                    Some('n') => str_build.push(b'\n'),
                    Some('r') => str_build.push(b'\r'),
                    Some('t') => str_build.push(b'\t'),
                    Some('0') => str_build.push(b'\0'),
                    Some('"') => str_build.push(b'"'),
                    Some('x') => match (iter.next(), iter.next()) {
                        (Some(d0), Some(d1))
                            if d0.is_ascii_hexdigit() && d1.is_ascii_hexdigit() =>
                        {
                            self.seek.next(d0);
                            self.seek.next(d1);
                            str_build.push(((x2i(d0 as u8) << 4) | x2i(d1 as u8)) as u8);
                        }
                        (m0, m1) => {
                            if let Some(d0) = m0 {
                                self.seek.next(d0);
                            }
                            if let Some(d1) = m1 {
                                self.seek.next(d1);
                            }
                            return Err(CompileErr::BadEscape(escape_start.to(&self.seek)));
                        }
                    },
                    _ => return Err(CompileErr::BadEscape(escape_start.to(&self.seek))),
                }
            } else if ch == '"' {
                self.seek.next(ch);
                break;
            } else {
                self.seek.next(ch);
                if ch.len_utf8() == 1 {
                    str_build.push(ch as u8);
                } else {
                    let mut utf8buf: [u8; 4] = [0; 4];
                    ch.encode_utf8(&mut utf8buf);
                    str_build.extend_from_slice(&utf8buf);
                }
            }
        }

        Ok(AnnotTok::new(
            self.cursor.to(&self.seek),
            Token::StringLiteral(str_build),
        ))
    }

    fn lex_sym(&mut self, first: char) -> Result<AnnotTok, CompileErr> {
        self.seek.next(first);

        let tok = match first {
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LCurly,
            '}' => Token::RCurly,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '<' => match self.peek_2char() {
                (Some('<'), Some('=')) => {
                    self.seek.next('<');
                    self.seek.next('=');
                    Token::LLAngleEqual
                }
                (Some('<'), _) => {
                    self.seek.next('<');
                    Token::LLAngle
                }
                _ => Token::LAngle,
            },
            '>' => match self.peek_2char() {
                (Some('>'), Some('=')) => {
                    self.seek.next('>');
                    self.seek.next('=');
                    Token::RRAngleEqual
                }
                (Some('>'), _) => {
                    self.seek.next('>');
                    Token::RRAngle
                }
                _ => Token::RAngle,
            },
            '@' => Token::Ampersat,
            '&' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::AmpersandEqual
                }
                Some('&') => {
                    self.seek.next('&');
                    Token::DoubleAmpersand
                }
                _ => Token::Ampersand,
            },
            '|' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::PipeEqual
                }
                Some('|') => {
                    self.seek.next('|');
                    Token::DoublePipe
                }
                _ => Token::Pipe,
            },
            '+' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::PlusEqual
                }
                _ => Token::Plus,
            },
            '-' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::DashEqual
                }
                Some('>') => {
                    self.seek.next('>');
                    Token::Arrow
                }
                _ => Token::Dash,
            },
            '*' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::AsteriskEqual
                }
                _ => Token::Asterisk,
            },
            '/' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::FSlashEqual
                }
                _ => Token::FSlash,
            },
            '%' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::PercentEqual
                }
                _ => Token::Percent,
            },
            '!' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::BangEqual
                }
                _ => Token::Bang,
            },
            '^' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::CaretEqual
                }
                _ => Token::Caret,
            },
            '~' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::TildeEqual
                }
                _ => Token::Tilde,
            },
            '=' => match self.peek_char() {
                Some('=') => {
                    self.seek.next('=');
                    Token::DoubleEqual
                }
                _ => Token::Equal,
            },
            ';' => Token::Semicolon,
            ':' => match self.peek_char() {
                Some(':') => {
                    self.seek.next(':');
                    Token::DoubleColon
                }
                _ => Token::Colon,
            },
            '.' => match self.peek_char() {
                Some('.') => {
                    self.seek.next('.');
                    Token::Ellipsis
                }
                _ => Token::Period,
            },
            ',' => Token::Comma,
            _ => Token::Invalid,
        };

        Ok(AnnotTok::new(self.cursor.to(&self.seek), tok))
    }

    fn lex_new(&mut self) -> Result<AnnotTok, CompileErr> {
        self.skip_ws();
        self.cursor = self.seek;

        let new_tok = match self.peek_char() {
            Option::None => Ok(AnnotTok::new(self.seek, Token::Eof)),
            Option::Some(ch) if ch.is_alphabetic() || ch == '_' => self.lex_ident(ch),
            Option::Some(ch) if ch.is_ascii_digit() => self.lex_numlit(ch),
            Option::Some('"') => self.lex_stringlit(),
            Option::Some(ch) => self.lex_sym(ch),
        };
        self.cursor = self.seek;

        new_tok
    }

    fn fill_peekbuf(&mut self) {
        while self.head_idx >= self.peek_buf.len() {
            // Doing this to stash the error (instead of ?)
            match self.lex_new() {
                Ok(tok) => self.peek_buf.push(tok),
                Err(err) => {
                    self.err = Some(err);
                    return;
                }
            }
        }
    }

    pub fn peek(&mut self) -> Result<&AnnotTok, CompileErr> {
        if let Some(e) = &self.err {
            return Err(e.clone());
        }

        self.fill_peekbuf();
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        // Catch the peek buffer up to the head pointer
        Ok(&self.peek_buf[self.head_idx])
    }

    pub fn eat(&mut self) {
        if self.err.is_some() {
            return;
        }

        self.head_idx += 1;
    }

    pub fn mark(&mut self) {
        self.mark_data = (self.cursor, self.head_idx);
    }

    pub fn rewind(&mut self) {
        (self.cursor, self.head_idx) = self.mark_data;
        self.seek = self.cursor;
    }

    pub fn head_loc(&mut self) -> SourceLoc {
        if self.err.is_some() {
            return SL_NIL;
        }
        self.fill_peekbuf();
        if self.err.is_some() {
            SL_NIL
        } else {
            self.peek_buf[self.head_idx].loc
        }
    }

    pub fn prev_loc(&mut self) -> SourceLoc {
        if self.err.is_some() {
            return SL_NIL;
        }
        self.fill_peekbuf();
        if self.err.is_some() {
            SL_NIL
        } else if self.head_idx == 0 {
            SL_BEGIN
        } else {
            self.peek_buf[self.head_idx - 1].loc
        }
    }
}

// Some helpers, move these probably
// implied x is hexdigit
fn x2i(x: u8) -> u64 {
    if x.is_ascii_digit() {
        (x - (b'0')) as u64
    } else if (b'a'..=b'f').contains(&x) {
        (x - (b'a') + 10) as u64
    } else {
        (x - (b'A') + 10) as u64
    }
}

// implied d is digit
fn d2i(d: u8) -> u64 {
    (d - (b'0')) as u64
}

fn is_ascii_octal(ch: char) -> bool {
    matches!(ch, '0'..='7')
}

fn is_ascii_binary(ch: char) -> bool {
    ch == '0' || ch == '1'
}

mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut lexer = Lex::new(String::from(
            "token   another\ntoken+=+352 010 0b1110110 and  0x1f",
        ));
        let expect_list = [
            AnnotTok::new(
                SourceLoc::new(1, 1, 0, 5),
                Token::Identifier(String::from("token")),
            ),
            AnnotTok::new(
                SourceLoc::new(1, 9, 8, 7),
                Token::Identifier(String::from("another")),
            ),
            AnnotTok::new(
                SourceLoc::new(2, 1, 16, 5),
                Token::Identifier(String::from("token")),
            ),
            AnnotTok::new(SourceLoc::new(2, 6, 21, 2), Token::PlusEqual),
            AnnotTok::new(SourceLoc::new(2, 8, 23, 1), Token::Plus),
            AnnotTok::new(SourceLoc::new(2, 9, 24, 3), Token::IntegralLiteral(352)),
            AnnotTok::new(SourceLoc::new(2, 13, 28, 3), Token::IntegralLiteral(0o10)),
            AnnotTok::new(
                SourceLoc::new(2, 17, 32, 9),
                Token::IntegralLiteral(0b1110110),
            ),
            AnnotTok::new(
                SourceLoc::new(2, 27, 42, 3),
                Token::Identifier(String::from("and")),
            ),
            AnnotTok::new(SourceLoc::new(2, 32, 47, 4), Token::IntegralLiteral(0x1f)),
            AnnotTok::new(SourceLoc::new(2, 36, 51, 0), Token::Eof),
        ];
        for expect in &expect_list {
            let actual = lexer.peek().expect("Failed to parse token");
            assert_eq!(expect, actual);
            lexer.eat();
        }
    }

    #[test]
    fn test_string() {
        let mut lexer = Lex::new(String::from(
            "\"this is a string\"\n\
             \"now with \\n some escapes\"\n\
             \"\\\\\\n\\r\\t\\0\\\"\"\n\
             \"before\\x00\\x01\\xff\\x80\\x45\\x95\\xeeafter\"",
        ));
        let expect_list = [
            AnnotTok::new(
                SourceLoc::new(1, 1, 0, 18),
                Token::StringLiteral(Vec::from(b"this is a string")),
            ),
            AnnotTok::new(
                SourceLoc::new(2, 1, 19, 26),
                Token::StringLiteral(Vec::from(b"now with \n some escapes")),
            ),
            AnnotTok::new(
                SourceLoc::new(3, 1, 19 + 27, 14),
                Token::StringLiteral(Vec::from(b"\\\n\r\t\0\"")),
            ),
            AnnotTok::new(
                SourceLoc::new(4, 1, 19 + 27 + 15, 41),
                Token::StringLiteral(Vec::from(b"before\x00\x01\xff\x80\x45\x95\xeeafter")),
            ),
            AnnotTok::new(SourceLoc::new(4, 42, 19 + 27 + 15 + 41, 0), Token::Eof),
        ];

        for expect in &expect_list {
            let actual = lexer.peek().expect("Failed to parse token");
            assert_eq!(expect, actual);
            lexer.eat();
        }
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lex::new(String::from("before//after\n1//2\na //\n\"str//a\"//\n"));
        let expect_list = [
            AnnotTok::new(
                SourceLoc::new(1, 1, 0, 6),
                Token::Identifier(String::from("before")),
            ),
            AnnotTok::new(SourceLoc::new(2, 1, 14, 1), Token::IntegralLiteral(1)),
            AnnotTok::new(
                SourceLoc::new(3, 1, 19, 1),
                Token::Identifier(String::from("a")),
            ),
            AnnotTok::new(
                SourceLoc::new(4, 1, 24, 8),
                Token::StringLiteral(Vec::from(b"str//a")),
            ),
            AnnotTok::new(SourceLoc::new(5, 1, 35, 0), Token::Eof),
        ];

        for expect in &expect_list {
            let actual = lexer.peek().expect("Failed to parse token");
            assert_eq!(expect, actual);
            lexer.eat();
        }
    }
}
