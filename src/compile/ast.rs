use crate::compile::LocationAnnot;

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

#[derive(PartialEq, Eq, Debug)]
pub enum TypeKind {
    List,
    Maybe,
    View,
}

#[derive(PartialEq, Eq, Debug)]
pub struct FullType {
    pub kind: Vec<(TypeKind, bool)>,
    pub base: (BaseType, bool),
}

/////////////////////
// Procedural lang //
/////////////////////
pub struct Var {
    pub name: LocationAnnot<String>,
    pub tp: LocationAnnot<FullType>,
}

type AnnotPSyn = LocationAnnot<Box<PSyn>>;
type AnnotPExpr = LocationAnnot<Box<PExpr>>;

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

pub enum UnaryOp {
    Neg,
    BNot,
    Not,
}

pub enum PExpr {
    // Conditionals
    If(AnnotPExpr, AnnotPSyn),                         // if Expr { Syn }
    IfElse(AnnotPExpr, AnnotPSyn, AnnotPSyn),          // if Expr { Syn0 } else { Syn1 }
    While(AnnotPExpr, AnnotPSyn),                      // while Expr { Syn }
    For(LocationAnnot<String>, AnnotPExpr, AnnotPSyn), // for Id in Expr { Syn }
    Match(AnnotPExpr, Vec<(AnnotPExpr, AnnotPSyn)>),   // match Expr0 { Expr1 -> { Syn0 }... }
    Case(Vec<(AnnotPExpr, AnnotPSyn)>),                // case { Expr0 -> { Syn0 }... }

    // Expressions
    Assign(AnnotPExpr, AssignOp, AnnotPExpr),          // Expr0 AssnOp Expr1
    Binary(AnnotPExpr, BinaryOp, AnnotPExpr),          // Expr0 BinOp Expr1
    Unary(UnaryOp, AnnotPExpr),                        // UnOp Expr
    Cast(AnnotPExpr, LocationAnnot<FullType>),         // Expr : FullType
    MemAcc(AnnotPExpr, LocationAnnot<String>),         // Expr.Id
    Subscr(AnnotPExpr, AnnotPExpr),                    // Expr0[Expr1]
    Call(AnnotPExpr, Vec<AnnotPExpr>),                 // Expr0(Expr1...)

    // Leaves
    Lit(Literal),                                      // Literal
    Ident(String),                                     // Id
}

// Useful constant for anything with an implied void expression
const VOIDEXPR: PExpr = PExpr::Lit(Literal::Void);

pub enum PSyn {
    Seq(AnnotPSyn, AnnotPSyn),
    VarDecl(Var, Option<AnnotPExpr>), // var Name: FullType = Expr?;
    Guard(AnnotPExpr, AnnotPExpr),    // guard Expr0 -> Expr1?;
    Break,                            // break;
    Continue,                         // continue;
    Return(AnnotPExpr),               // return Expr?;
    Expr(AnnotPExpr),                 // Expr;
}

pub struct FuncDef {
    pub name: LocationAnnot<String>,
    pub params: Vec<Var>,
    pub rtp: LocationAnnot<FullType>,
    pub body: AnnotPSyn,
}


/////////////////////
// Behavioral lang //
/////////////////////
type AnnotBExpr = LocationAnnot<Box<BExpr>>;

pub enum BExpr {
    // Expressions
    Binary(AnnotBExpr, BinaryOp, AnnotBExpr),  // Expr0 BinOp Expr1
    Unary(UnaryOp, AnnotBExpr),                // UnOp Expr
    Cast(AnnotBExpr, LocationAnnot<FullType>), // Expr : FullType
    MemAcc(AnnotBExpr, LocationAnnot<String>), // Expr.Id
    Subscr(AnnotBExpr, AnnotBExpr),            // Expr0[Expr1]
    Call(String, Vec<AnnotBExpr>),             // (#Id Expr0, Expr1...)
    Curry(String, Vec<AnnotBExpr>),            // (@Id Expr0, Expr1...)

    // Literals
    Lit(Literal),                             // Literal
    Ident(String),                            // Id
}

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
    pub ents: Vec<(LocationAnnot<String>, LocationAnnot<u64>)>
}


/////////////
// Externs //
/////////////
pub struct FuncDecl {
    pub name: LocationAnnot<String>,
    pub params: Vec<Var>,
    pub rtp: LocationAnnot<FullType>,
}


////////////////
// Interfaces //
////////////////
pub struct IVarDef {
    name: LocationAnnot<String>,
    tp: LocationAnnot<FullType>,
    off: LocationAnnot<u64>,
}

pub struct InterfaceDef {
    name: LocationAnnot<String>,
    inherit: LocationAnnot<Option<String>>,
    func_list: Vec<FuncDef>,
    var_list: Vec<IVarDef>,
}
