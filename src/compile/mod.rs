use std::fmt;

pub mod ast;
pub mod lex;
pub mod parsec;

#[derive(Clone, Debug)]
pub enum CompileErr {
    IntOverflow(SourceLoc),
    BadEscape(SourceLoc),
    Mismatch(lex::Token, lex::Token, SourceLoc),
}

impl fmt::Display for CompileErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntOverflow(_) => write!(f, "Integer too large to fit in u64"),
            Self::BadEscape(_) => write!(f, "Invalid escape sequence"),
            Self::Mismatch(e, a, _) => write!(f, "Expected token {} but got {}", e, a),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SourceLoc {
    pub row: i32,
    pub col: i32,
    pub off: usize,
    pub len: usize,
}

pub const SL_BEGIN: SourceLoc = SourceLoc {
    row: 1,
    col: 1,
    off: 0,
    len: 0,
};

impl SourceLoc {
    pub fn new(row: i32, col: i32, off: usize, len: usize) -> SourceLoc {
        SourceLoc { row, col, off, len }
    }
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

#[derive(PartialEq, Debug)]
pub struct LocationAnnot<T> {
    pub loc: SourceLoc,
    pub inner: T,
}

impl<T> LocationAnnot<T> {
    pub fn new(loc: SourceLoc, inner: T) -> Self {
        Self { loc, inner }
    }
}
