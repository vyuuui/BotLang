use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::str;

#[derive(Copy, Clone)]
pub struct SourceLoc {
    row: i32,
    col: i32,
    off: usize,
    len: usize,
}

const SL_BEGIN: SourceLoc = SourceLoc {
    row: 1,
    col: 1,
    off: 0,
    len: 0,
};

impl SourceLoc {
    pub fn next(&mut self, ch: char) {
        if ch == '\n' {
            self.row += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }

        self.off += ch.len_utf8();
    }

    pub fn to(&self, end: &SourceLoc) -> SourceLoc {
        SourceLoc {
            row: self.row,
            col: self.col,
            off: self.off,
            len: end.off - self.off,
        }
    }
}

pub struct LocationAnnot<T> {
    loc: SourceLoc,
    inner: T,
}

impl<T> LocationAnnot<T> {
    pub fn new(loc: SourceLoc, inner: T) -> Self {
        Self { loc, inner }
    }
}

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

type AnnotTok = LocationAnnot<Token>;

pub struct Lex {
    source: String,

    peek_buf: Vec<AnnotTok>,
    head_loc: usize,

    cursor: SourceLoc,
    seek: SourceLoc,

    err: Option<String>,
}

impl Lex {
    pub fn new(source: String) -> Lex {
        Self {
            source,
            peek_buf: Vec::new(),
            head_loc: 0,
            cursor: SL_BEGIN,
            seek: SL_BEGIN,
            err: None,
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

    fn lex_ident(&mut self, first: char) -> Option<AnnotTok> {
        self.seek.next(first);

        for ch in self.source[self.seek.off..].chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                break;
            }

            self.seek.next(ch);
        }

        Some(AnnotTok::new(
            self.cursor.to(&self.seek),
            Token::Identifier(self.source[self.cursor.off..self.seek.off].to_string()),
        ))
    }

    fn lex_numlit(&mut self, first: char) -> Option<AnnotTok> {
        self.seek.next(first);

        // Instead of doing a dfa here, easier to just write this one out
        if first == '0' {
            let mut val: u64 = 0;
            match self.peek_char() {
                Some('x') => {
                    self.seek.next('x');
                    for ch in self.source[self.seek.off..].chars() {
                        if ch.is_ascii_hexdigit() {
                            // TODO: overflow checks
                            val = val * 16 + x2i(ch as u8);
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
                            // TODO: overflow checks
                            val = val * 2 + d2i(ch as u8);
                        } else {
                            break;
                        }
                        self.seek.next(ch);
                    }
                }
                Some(ch) if ch.is_ascii_digit() => {
                    for ch in self.source[self.seek.off..].chars() {
                        if is_ascii_octal(ch) {
                            // TODO: overflow checks
                            // TODO: overflow checks
                            // TODO: overflow checks
                            // TODO: overflow checks
                            // TODO: overflow checks
                            val = val * 8 + d2i(ch as u8);
                        } else {
                            break;
                        }
                        self.seek.next(ch);
                    }
                }
                // Empty or non-digit means standalone 0, which val already is set with
                _ => (),
            }
            Some(AnnotTok::new(
                self.cursor.to(&self.seek),
                Token::IntegralLiteral(val),
            ))
        } else {
            let mut ival: u64 = d2i(first as u8);
            let mut has_fpart: bool = false;
            for ch in self.source[self.seek.off..].chars() {
                if ch == '.' {
                    has_fpart = true;
                }
                if ch.is_ascii_digit() {
                    // TODO: overflow checks
                    ival = ival * 10 + d2i(ch as u8);
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
                    Some(AnnotTok::new(
                        self.cursor.to(&self.seek),
                        Token::FloatLiteral(fval),
                    ))
                } else {
                    let dval: f64 = self.source[self.cursor.off..self.seek.off]
                        .parse::<f64>()
                        .expect("Failed to parse well-formatted f64");
                    Some(AnnotTok::new(
                        self.cursor.to(&self.seek),
                        Token::DoubleLiteral(dval),
                    ))
                }
            } else {
                Some(AnnotTok::new(
                    self.cursor.to(&self.seek),
                    Token::IntegralLiteral(ival),
                ))
            }
        }
    }

    fn lex_stringlit(&mut self) -> Option<AnnotTok> {
        self.seek.next('"');

        let mut str_build: Vec<u8> = Vec::new();
        let mut iter = self.source[self.seek.off..].chars();
        while let Some(ch) = iter.next() {
            if ch == '\\' {
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
                    Some('x') => {
                        match (iter.next(), iter.next()) {
                            (Some(d0), Some(d1))
                                if d0.is_ascii_hexdigit() && d1.is_ascii_hexdigit() =>
                            {
                                str_build.push(((x2i(d0 as u8) << 4) | x2i(d1 as u8)) as u8);
                            }
                            // Error message needed
                            _ => return None,
                        }
                    }
                    // Error message needed
                    _ => return None,
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

        Some(AnnotTok::new(
            self.cursor.to(&self.seek),
            Token::StringLiteral(str_build),
        ))
    }

    fn lex_sym(&mut self, first: char) -> Option<AnnotTok> {
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

        Some(AnnotTok::new(self.cursor.to(&self.seek), tok))
    }

    fn lex_new(&mut self) -> Option<AnnotTok> {
        self.skip_ws();
        self.cursor = self.seek;

        let new_tok = match self.peek_char() {
            Option::None => Some(AnnotTok::new(self.seek, Token::Eof)),
            Option::Some(ch) if ch.is_alphabetic() || ch == '_' => self.lex_ident(ch),
            Option::Some(ch) if ch.is_ascii_digit() => self.lex_numlit(ch),
            Option::Some('"') => self.lex_stringlit(),
            Option::Some(ch) => self.lex_sym(ch),
        };
        self.cursor = self.seek;

        new_tok
    }

    pub fn peek(&mut self) -> Option<&AnnotTok> {
        if self.head_loc == self.peek_buf.len() {
            if let Some(tok) = self.lex_new() {
                self.peek_buf.push(tok);
            } else {
                self.err = Some(String::from("TODO: Failure reason"));
                return None;
            }
        }
        Some(&self.peek_buf[self.head_loc])
    }

    pub fn eat(&mut self) {
        self.head_loc += 1;
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

    fn is_id(tok: &Token, expect_id: &str) -> bool {
        if let Token::Identifier(id) = tok {
            expect_id == id
        } else {
            false
        }
    }

    fn same_tok(a: &Token, b: Token) -> bool {
        std::mem::discriminant(a) == std::mem::discriminant(&b)
    }

    fn is_int(a: &Token, expect_val: u64) -> bool {
        if let Token::IntegralLiteral(val) = a {
            expect_val == *val
        } else {
            false
        }
    }

    #[test]
    fn test_lex() {
        let mut lexer = Lex::new(String::from("token   another\ntoken+=+352 010 0b1110110 and  0x1f"));
        assert!(is_id(&lexer.peek().expect("Valid token").inner, "token"));
        lexer.eat();
        assert!(is_id(&lexer.peek().expect("Valid token").inner, "another"));
        lexer.eat();
        assert!(is_id(&lexer.peek().expect("Valid token").inner, "token"));
        lexer.eat();
        assert!(same_tok(&lexer.peek().expect("Valid token").inner, Token::PlusEqual));
        lexer.eat();
        assert!(same_tok(&lexer.peek().expect("Valid token").inner, Token::Plus));
        lexer.eat();
        assert!(is_int(&lexer.peek().expect("Valid token").inner, 352));
        lexer.eat();
        assert!(is_int(&lexer.peek().expect("Valid token").inner, 0o10));
        lexer.eat();
        assert!(is_int(&lexer.peek().expect("Valid token").inner, 0b1110110));
        lexer.eat();
        assert!(is_id(&lexer.peek().expect("Valid token").inner, "and"));
        lexer.eat();
        assert!(is_int(&lexer.peek().expect("Valid token").inner, 0x1f));
        lexer.eat();
        assert!(same_tok(&lexer.peek().expect("Valid token").inner, Token::Eof));
        lexer.eat();
        assert!(same_tok(&lexer.peek().expect("Valid token").inner, Token::Eof));
    }
}
