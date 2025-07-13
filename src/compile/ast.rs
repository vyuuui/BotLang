use crate::compile::LocationAnnot;
use std::fmt;

#[derive(Default)]
pub struct Script {
    pub imports: Vec<LocationAnnot<String>>,
    pub funcs: Vec<FuncDef>,
    pub behaviors: Vec<BehaviorDef>,
    pub enums: Vec<EnumerationDef>,
    pub externs: Vec<FuncDecl>,
    pub interfaces: Vec<InterfaceDef>,
}

////////////
// Shared //
////////////
#[derive(Debug)]
pub enum Literal {
    Void,                 // <>? or E
    Integral(u64),        // Integral
    Bool(bool),           // true | false
    Flt(f32),             // Float
    Dbl(f64),             // Double
    Vec2(f32, f32),       // vec2(x, y)
    Vec3(f32, f32, f32),  // vec3(x, y, z)
    Str(Vec<u8>),         // "string"
    Range(i64, i64, i64), // [begin, end, step]
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Void => write!(f, "()"),
            Self::Integral(i) => write!(f, "{i}"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Flt(flt) => write!(f, "{flt}"),
            Self::Dbl(d) => write!(f, "{d}"),
            Self::Vec2(fx, fy) => write!(f, "<{fx}, {fy}>"),
            Self::Vec3(fx, fy, fz) => write!(f, "<{fx}, {fy}, {fz}>"),
            Self::Str(utf) => write!(
                f,
                "\"{}\"",
                String::from_utf8(utf.clone()).unwrap_or_default()
            ),
            Self::Range(b, e, step) => write!(f, "[{b}..{e}|{step}]"),
        }
    }
}

///////////
// Types //
///////////
#[derive(PartialEq, Eq, Debug)]
pub enum BaseType {
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Bool,
    Flt,
    Dbl,
    Vec2,
    Vec3,
    Str,
    Range,
    Status,
    Callable(Vec<FullType>),
    NonPrim(String),
}

impl fmt::Display for BaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int8 => write!(f, "i8"),
            Self::Int16 => write!(f, "i16"),
            Self::Int32 => write!(f, "i32"),
            Self::Int64 => write!(f, "i64"),
            Self::UInt8 => write!(f, "u8"),
            Self::UInt16 => write!(f, "u16"),
            Self::UInt32 => write!(f, "u32"),
            Self::UInt64 => write!(f, "u64"),
            Self::Bool => write!(f, "bool"),
            Self::Flt => write!(f, "float"),
            Self::Dbl => write!(f, "double"),
            Self::Vec2 => write!(f, "vec2"),
            Self::Vec3 => write!(f, "vec3"),
            Self::Str => write!(f, "string"),
            Self::Range => write!(f, "range"),
            Self::Status => write!(f, "STATUS"),
            Self::Callable(tps) => write!(
                // sorry perf
                f,
                "({})",
                tps.iter()
                    .map(FullType::to_string)
                    .collect::<Vec<String>>()
                    .join(" -> ")
            ),
            Self::NonPrim(nm) => write!(f, "{nm}"),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum TypeKind {
    List,
    Maybe,
    View,
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::List => write!(f, "list"),
            Self::Maybe => write!(f, "maybe"),
            Self::View => write!(f, "view"),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct FullType {
    pub kind: Vec<(TypeKind, bool)>,
    pub base: (BaseType, bool),
}

impl fmt::Display for FullType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (tk, isref) in &self.kind {
            if *isref {
                write!(f, "{tk}& ")?;
            } else {
                write!(f, "{tk} ")?;
            }
        }
        if self.base.1 {
            write!(f, "{}&", self.base.0)
        } else {
            write!(f, "{}", self.base.0)
        }
    }
}

macro_rules! mkbasetype {
    (i8) => {
        BaseType::Int8
    };
    (i16) => {
        BaseType::Int16
    };
    (i32) => {
        BaseType::Int32
    };
    (i64) => {
        BaseType::Int64
    };
    (u8) => {
        BaseType::UInt8
    };
    (u16) => {
        BaseType::UInt16
    };
    (u32) => {
        BaseType::UInt32
    };
    (u64) => {
        BaseType::UInt64
    };
    (bool) => {
        BaseType::Bool
    };
    (float) => {
        BaseType::Flt
    };
    (double) => {
        BaseType::Dbl
    };
    (vec2) => {
        BaseType::Vec2
    };
    (vec3) => {
        BaseType::Vec3
    };
    (string) => {
        BaseType::Str
    };
    (range) => {
        BaseType::Range
    };
    (STATUS) => {
        BaseType::Status
    };
    ($other:ident) => {
        BaseType::NonPrim(String::from(stringify!($other)))
    };
}

macro_rules! mkfunctype {
    ($ptl:tt [$($k:ident $b:expr)*] | list& $($rest:tt)*) => {
        mkfunctype!($ptl [$($k $b)*List true] | $($rest)*)
    };
    ($ptl:tt [$($k:ident $b:expr)*] | maybe& $($rest:tt)*) => {
        mkfunctype!($ptl [$($k $b)*Maybe true] | $($rest)*)
    };
    ($ptl:tt [$($k:ident $b:expr)*] | view& $($rest:tt)*) => {
        mkfunctype!($ptl [$($k $b)*View true] | $($rest)*)
    };
    ($ptl:tt [$($k:ident $b:expr)*] | list $($rest:tt)*) => {
        mkfunctype!($ptl [$($k $b)*List false] | $($rest)*)
    };
    ($ptl:tt [$($k:ident $b:expr)*] | maybe $($rest:tt)*) => {
        mkfunctype!($ptl [$($k $b)*Maybe false] | $($rest)*)
    };
    ($ptl:tt [$($k:ident $b:expr)*] | view $($rest:tt)*) => {
        mkfunctype!($ptl [$($k $b)*View false] | $($rest)*)
    };
    ([$($ptl:expr)*] $kindlist:tt | $bt:ident -> $($rest:tt)*) => {
        mkfunctype!([$($ptl)* mkfulltype_inner!($kindlist | $bt)] [] | $($rest)*)
    };
    ([$($ptl:expr)*] $kindlist:tt | $bt:ident& -> $($rest:tt)*) => {
        mkfunctype!([$($ptl)* mkfulltype_inner!($kindlist | $bt&)] [] | $($rest)*)
    };
    ([$($ptl:expr)*] $kindlist:tt | $func:tt -> $($rest:tt)*) => {
        mkfunctype!([$($ptl)* mkfulltype_inner!($kindlist | $func)] [] | $($rest)*)
    };
    ([$($ptl:expr)*] $kindlist:tt | $bt:ident) => {
        BaseType::Callable(vec![$($ptl,)*mkfulltype_inner!($kindlist | $bt)])
    };
    ([$($ptl:expr)*] $kindlist:tt | $bt:ident&) => {
        BaseType::Callable(vec![$($ptl,)*mkfulltype_inner!($kindlist | $bt&)])
    };
    ([$($ptl:expr)*] $kindlist:tt | $func:tt) => {
        BaseType::Callable(vec![$($ptl,)*mkfulltype_inner!($kindlist | $func)])
    };
}

macro_rules! mkfulltype_inner {
    ([$($k:ident $b:expr)*] | list& $($rest:tt)* ) => {
        mkfulltype_inner!([$($k $b)*List true] | $($rest)*)
    };
    ([$($k:ident $b:expr)*] | maybe& $($rest:tt)* ) => {
        mkfulltype_inner!([$($k $b)*Maybe true] | $($rest)*)
    };
    ([$($k:ident $b:expr)*] | view& $($rest:tt)* ) => {
        mkfulltype_inner!([$($k $b)*View true] | $($rest)*)
    };
    ([$($k:ident $b:expr)*] | list $($rest:tt)* ) => {
        mkfulltype_inner!([$($k $b)*List false] | $($rest)*)
    };
    ([$($k:ident $b:expr)*] | maybe $($rest:tt)* ) => {
        mkfulltype_inner!([$($k $b)*Maybe false] | $($rest)*)
    };
    ([$($k:ident $b:expr)*] | view $($rest:tt)* ) => {
        mkfulltype_inner!([$($k $b)*View false] | $($rest)*)
    };
    ([$($k:ident $b:expr)*] | $bt:ident&) => {
        FullType {
            kind: vec![$((TypeKind::$k, $b)),*],
            base: (mkbasetype!($bt), true),
        }
    };
    ([$($k:ident $b:expr)*] | $bt:ident) => {
        FullType {
            kind: vec![$((TypeKind::$k, $b)),*],
            base: (mkbasetype!($bt), false),
        }
    };
    ([$($k:ident $b:expr)*] | ($($func:tt)*)) => {
        FullType {
            kind: vec![$((TypeKind::$k, $b)),*],
            base: (mkfunctype!([] [] | $($func)*), false),
        }
    };
}

macro_rules! mkfulltype {
    ($($toks:tt)*) => {
        mkfulltype_inner!([] | $($toks)*)
    };
}

pub(crate) use mkbasetype;
pub(crate) use mkfulltype;
pub(crate) use mkfulltype_inner;
pub(crate) use mkfunctype;

/////////////////////
// Procedural lang //
/////////////////////
#[derive(PartialEq, Debug)]
pub struct Var {
    pub name: LocationAnnot<String>,
    pub tp: LocationAnnot<FullType>,
}

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.tp)
    }
}

pub type AnnotPExpr = LocationAnnot<Box<PExpr>>;

#[derive(Debug)]
pub enum AssignOp {
    None,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Xor,
    Or,
    Not,
    Lsh,
    Rsh,
}

impl fmt::Display for AssignOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "="),
            Self::Add => write!(f, "+="),
            Self::Sub => write!(f, "-="),
            Self::Mul => write!(f, "*="),
            Self::Div => write!(f, "/="),
            Self::Mod => write!(f, "%="),
            Self::And => write!(f, "&="),
            Self::Xor => write!(f, "^="),
            Self::Or => write!(f, "|="),
            Self::Not => write!(f, "!="),
            Self::Lsh => write!(f, "<<="),
            Self::Rsh => write!(f, ">>="),
        }
    }
}

#[derive(Debug)]
pub enum BinaryOp {
    Or,
    And,
    BOr,
    BXor,
    BAnd,
    CmpEq,
    CmpNe,
    CmpGt,
    CmpLt,
    CmpGe,
    CmpLe,
    Rsh,
    Lsh,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Or => write!(f, "||"),
            Self::And => write!(f, "&&"),
            Self::BOr => write!(f, "|"),
            Self::BXor => write!(f, "^"),
            Self::BAnd => write!(f, "&"),
            Self::CmpEq => write!(f, "=="),
            Self::CmpNe => write!(f, "!="),
            Self::CmpGt => write!(f, ">"),
            Self::CmpLt => write!(f, "<"),
            Self::CmpGe => write!(f, ">="),
            Self::CmpLe => write!(f, "<="),
            Self::Rsh => write!(f, ">>"),
            Self::Lsh => write!(f, "<<"),
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Mod => write!(f, "%"),
        }
    }
}

#[derive(Debug)]
pub enum UnaryOp {
    Neg,
    BNot,
    Not,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Neg => write!(f, "-"),
            Self::BNot => write!(f, "~"),
            Self::Not => write!(f, "!"),
        }
    }
}

#[derive(Debug)]
pub enum PExpr {
    Block(Vec<AnnotPExpr>, Option<AnnotPExpr>),

    // Conditionals
    If(AnnotPExpr, AnnotPExpr),                         // if Expr { Syn }
    IfElse(AnnotPExpr, AnnotPExpr, AnnotPExpr),         // if Expr { Syn0 } else { Syn1 }
    While(AnnotPExpr, AnnotPExpr),                      // while Expr { Syn }
    For(LocationAnnot<String>, AnnotPExpr, AnnotPExpr), // for Id in Expr { Syn }
    Match(AnnotPExpr, Vec<(AnnotPExpr, AnnotPExpr)>),   // match Expr0 { Expr1 -> { Syn0 }... }
    Case(Vec<(AnnotPExpr, AnnotPExpr)>),                // case { Expr0 -> { Syn0 }... }

    // Expressions
    Assign(AnnotPExpr, AssignOp, AnnotPExpr), // Expr0 AssnOp Expr1
    Binary(AnnotPExpr, BinaryOp, AnnotPExpr), // Expr0 BinOp Expr1
    Unary(UnaryOp, AnnotPExpr),               // UnOp Expr
    Cast(AnnotPExpr, LocationAnnot<FullType>), // Expr : FullType
    MemAcc(AnnotPExpr, LocationAnnot<String>), // Expr.Id
    Subscr(AnnotPExpr, AnnotPExpr),           // Expr0[Expr1]
    Call(AnnotPExpr, Vec<AnnotPExpr>),        // Expr0(Expr1...)

    // Leaves
    Lit(Literal),  // Literal
    Ident(String), // Id

    // Statements evaluating to void
    VarDecl(Var, AnnotPExpr),      // var Name: FullType = Expr;
    Guard(AnnotPExpr, AnnotPExpr), // guard Expr0 -> Expr1?;
    Break,                         // break;
    Continue,                      // continue;
    Return(AnnotPExpr),            // return Expr?;
    Nil,
}

impl fmt::Display for PExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block(subs, mout) => {
                writeln!(f, "{{")?;
                for sub in subs {
                    writeln!(f, "{sub};")?;
                }
                if let Some(out) = mout {
                    writeln!(f, "{out}")?;
                }
                writeln!(f, "}}")
            }

            Self::If(cond, tb) => writeln!(f, "if {cond} {tb}"),
            Self::IfElse(cond, tb, fb) => writeln!(f, "if {cond} {tb} else {fb}"),
            Self::While(cond, body) => writeln!(f, "while {cond} {body}"),
            Self::For(var, cont, body) => writeln!(f, "for {var} in {cont} {body}"),
            Self::Match(cond, arms) => {
                writeln!(f, "match {cond} {{")?;
                for (matcher, body) in arms {
                    writeln!(f, "{matcher} -> {body}")?;
                }
                writeln!(f, "}}")
            }
            Self::Case(arms) => {
                writeln!(f, "case {{")?;
                for (matcher, body) in arms {
                    writeln!(f, "{matcher} -> {body}")?;
                }
                writeln!(f, "}}")
            }

            Self::Assign(lhs, op, rhs) => write!(f, "{lhs} {op} {rhs}"),
            Self::Binary(lhs, op, rhs) => write!(f, "{lhs} {op} {rhs}"),
            Self::Unary(op, sub) => write!(f, "{op}{sub}"),
            Self::Cast(expr, tp) => write!(f, "{expr} : {tp}"),
            Self::MemAcc(expr, memb) => write!(f, "{expr}.{memb}"),
            Self::Subscr(expr, idx) => write!(f, "{expr}[{idx}]"),
            Self::Call(expr, params) => {
                // sorry perf
                write!(
                    f,
                    "{expr}({})",
                    params
                        .iter()
                        .map(AnnotPExpr::to_string)
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            Self::Lit(lit) => write!(f, "{lit}"),
            Self::Ident(id) => write!(f, "{id}"),
            Self::VarDecl(v, expr) => write!(f, "var {v} = {expr}"),
            Self::Guard(cond, ret) => write!(f, "guard {cond} -> {ret}"),
            Self::Break => write!(f, "break"),
            Self::Continue => write!(f, "continue"),
            Self::Return(expr) => write!(f, "return {expr}"),
            Self::Nil => write!(f, "<nil/error>"),
        }
    }
}

// Useful constant for anything with an implied void expression
pub const VOIDEXPR: PExpr = PExpr::Lit(Literal::Void);

#[derive(Debug)]
pub struct FuncDef {
    pub name: LocationAnnot<String>,
    pub params: Vec<Var>,
    pub rtp: LocationAnnot<FullType>,
    pub body: AnnotPExpr,
}

impl fmt::Display for FuncDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Function name: {}\n", self.name)?;
        for (idx, p) in self.params.iter().enumerate() {
            writeln!(f, "\tParameter {idx} = {p}")?;
        }
        writeln!(f, "\tReturns: {}", self.rtp)?;
        writeln!(f, "Body:\n{}", self.body)
    }
}

/////////////////////
// Behavioral lang //
/////////////////////
type AnnotBExpr = LocationAnnot<Box<BExpr>>;

#[derive(Debug)]
pub enum BExpr {
    // Expressions
    Binary(AnnotBExpr, BinaryOp, AnnotBExpr), // Expr0 BinOp Expr1
    Unary(UnaryOp, AnnotBExpr),               // UnOp Expr
    Cast(AnnotBExpr, LocationAnnot<FullType>), // Expr : FullType
    MemAcc(AnnotBExpr, LocationAnnot<String>), // Expr.Id
    Subscr(AnnotBExpr, AnnotBExpr),           // Expr0[Expr1]
    Call(String, Vec<AnnotBExpr>),            // (#Id Expr0, Expr1...)
    Curry(String, Vec<AnnotBExpr>),           // (@Id Expr0, Expr1...)

    // Literals
    Lit(Literal),  // Literal
    Ident(String), // Id
}

#[derive(Debug)]
pub struct BehaviorDef {
    name: LocationAnnot<String>,
    params: Vec<Var>,
    rtp: FullType, // Return type is inferred
    body: AnnotBExpr,
}

//////////////////
// Enumerations //
//////////////////
#[derive(PartialEq, Debug)]
pub struct EnumerationDef {
    pub name: LocationAnnot<String>,
    pub tp: LocationAnnot<BaseType>,
    pub ents: Vec<(LocationAnnot<String>, LocationAnnot<u64>)>,
}

/////////////
// Externs //
/////////////
#[derive(PartialEq, Debug)]
pub struct FuncDecl {
    pub name: LocationAnnot<String>,
    pub params: Vec<Var>,
    pub rtp: LocationAnnot<FullType>,
}

////////////////
// Interfaces //
////////////////
#[derive(Debug)]
pub struct IVarDef {
    name: LocationAnnot<String>,
    tp: LocationAnnot<FullType>,
    off: LocationAnnot<u64>,
}

#[derive(Debug)]
pub struct InterfaceDef {
    name: LocationAnnot<String>,
    inherit: LocationAnnot<Option<String>>,
    func_list: Vec<FuncDef>,
    var_list: Vec<IVarDef>,
}
