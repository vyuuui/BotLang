use crate::compile::ast::*;
use crate::compile::lex::{Lex, Token, TokenDiscriminants};
use crate::compile::parsec::{
    cvt, cvt_witherr, epsilon, extid, extint, icl, ifx, inv, mat, maybe, pfx, rep, seql, seqlr,
    seqr, withannot, Parser, ToOneOf,
};
use crate::compile::{annot, CompileErr, LocationAnnot, SourceLoc, SL_NIL};
use std::cmp::Ordering;

pub fn parse(script: String) -> Result<Script, CompileErr> {
    let mut lexer = Lex::new(script);
    let mut res = Script::default();
    while lexer.peek()?.inner != Token::Eof {
        match &lexer.peek()?.inner {
            Token::Identifier(i) if i == "import" => {
                lexer.eat();
                if let LocationAnnot {
                    loc,
                    inner: Token::StringLiteral(imp),
                } = &lexer.peek()?
                {
                    res.imports
                        .push(annot(*loc, String::from_utf8(imp.clone())?));
                } else {
                    return Err(CompileErr::TypeMismatch(
                        TokenDiscriminants::StringLiteral,
                        lexer.peek()?.inner.clone(),
                    ));
                }
                lexer.eat();
            }
            Token::Identifier(i) if i == "enum" => {
                lexer.eat();
                res.enums.push(parse_enum(&mut lexer)?);
            }
            Token::Identifier(i) if i == "interface" => {}
            Token::Identifier(i) if i == "behavior" => {}
            Token::Identifier(i) if i == "fn" => {}
            Token::Identifier(i) if i == "extern" => {
                lexer.eat();
                res.externs.push(parse_func_decl(&mut lexer)?);
            }
            _ => return Err(CompileErr::Message("Expected valid top-level definition")),
        }
    }
    Ok(res)
}

fn parse_enum_expr(lexer: &mut Lex) -> Result<u64, CompileErr> {
    let eval_binop = |lhs: u64, op: Token, rhs: u64| -> u64 {
        match op {
            Token::Pipe => lhs | rhs,
            Token::Caret => lhs ^ rhs,
            Token::Ampersand => lhs & rhs,
            Token::LLAngle => lhs << rhs,
            Token::RRAngle => lhs >> rhs,
            Token::Plus => lhs + rhs,
            Token::Dash => lhs - rhs,
            Token::Asterisk => lhs * rhs,
            Token::FSlash => lhs / rhs,
            Token::Percent => lhs % rhs,
            _ => panic!("Invalid operation fed to expression computation"),
        }
    };
    let eval_unop = |op: Token, res: u64| -> u64 {
        match op {
            Token::Dash => !res + 1,
            Token::Tilde => !res,
            _ => panic!("Invalid operation def to expression computation"),
        }
    };
    let enum_expr = ifx(
        ifx(
            ifx(
                ifx(
                    ifx(
                        ifx(
                            pfx(
                                &[Token::Dash, Token::Tilde],
                                eval_unop,
                                (
                                    extint(),
                                    seql(
                                        seqr(mat(Token::LParen), inv(parse_enum_expr)),
                                        mat(Token::RParen),
                                    ),
                                )
                                    .oneof(),
                            ),
                            &[Token::Asterisk, Token::FSlash, Token::Percent],
                            eval_binop,
                        ),
                        &[Token::Plus, Token::Dash],
                        eval_binop,
                    ),
                    &[Token::LLAngle, Token::RRAngle],
                    eval_binop,
                ),
                &[Token::Ampersand],
                eval_binop,
            ),
            &[Token::Caret],
            eval_binop,
        ),
        &[Token::Pipe],
        eval_binop,
    );
    enum_expr.parse(lexer)
}

fn parse_enum(lexer: &mut Lex) -> Result<EnumerationDef, CompileErr> {
    let (nm, mtp) =
        seqlr(withannot(extid()), maybe(Token::Colon, withannot(extid()))).parse(lexer)?;

    let parse_type = |tp: LocationAnnot<String>| -> Result<LocationAnnot<BaseType>, CompileErr> {
        match tp.inner.as_str() {
            "i8" => Ok(annot(tp.loc, BaseType::Int8)),
            "i16" => Ok(annot(tp.loc, BaseType::Int16)),
            "i32" => Ok(annot(tp.loc, BaseType::Int32)),
            "i64" => Ok(annot(tp.loc, BaseType::Int64)),
            "u8" => Ok(annot(tp.loc, BaseType::UInt8)),
            "u16" => Ok(annot(tp.loc, BaseType::UInt16)),
            "u32" => Ok(annot(tp.loc, BaseType::UInt32)),
            "u64" => Ok(annot(tp.loc, BaseType::UInt64)),
            _ => Err(CompileErr::InvalidEnumType(tp.inner)),
        }
    };
    let mut ret = EnumerationDef {
        name: nm,
        tp: if let Some(tp) = mtp {
            parse_type(tp)?
        } else {
            annot(SL_NIL, BaseType::Int32)
        },
        ents: Vec::new(),
    };
    let enum_ent = seqlr(
        withannot(extid()),
        maybe(Token::Equal, withannot(inv(parse_enum_expr))),
    );
    let enum_body = seqr(
        mat(Token::LCurly),
        seql(
            seql(icl(enum_ent, Token::Comma), maybe(Token::Comma, epsilon())),
            mat(Token::RCurly),
        ),
    );

    ret.ents = enum_body
        .parse(lexer)?
        .into_iter()
        .fold(Vec::new(), |mut vec, el| {
            vec.push((
                el.0,
                el.1.unwrap_or(annot(
                    SL_NIL,
                    vec.last().map(|x| x.1.inner + 1).unwrap_or(0),
                )),
            ));
            vec
        });
    Ok(ret)
}

fn parse_fulltype(lexer: &mut Lex) -> Result<FullType, CompileErr> {
    let basetype_id = |tn: String| -> BaseType {
        match tn.as_str() {
            "i8" => BaseType::Int8,
            "i16" => BaseType::Int16,
            "i32" => BaseType::Int32,
            "i64" => BaseType::Int64,
            "u8" => BaseType::UInt8,
            "u16" => BaseType::UInt16,
            "u32" => BaseType::UInt32,
            "u64" => BaseType::UInt64,
            "bool" => BaseType::Bool,
            "float" => BaseType::Flt,
            "double" => BaseType::Dbl,
            "vec2" => BaseType::Vec2,
            "vec3" => BaseType::Vec3,
            "string" => BaseType::Str,
            "range" => BaseType::Range,
            "STATUS" => BaseType::Status,
            _ => BaseType::NonPrim(tn),
        }
    };
    let kind = rep(seqlr(
        (
            cvt(
                |_| TypeKind::List,
                mat(Token::Identifier(String::from("list"))),
            ),
            cvt(
                |_| TypeKind::Maybe,
                mat(Token::Identifier(String::from("maybe"))),
            ),
            cvt(
                |_| TypeKind::View,
                mat(Token::Identifier(String::from("view"))),
            ),
        )
            .oneof(),
        cvt(|m| m.is_some(), maybe(Token::Ampersand, epsilon())),
    ))
    .parse(lexer)?;

    let base = seqlr(
        (
            // TODO: check for errors if basetype overlaps with keywords
            cvt(basetype_id, extid()),
            cvt(
                BaseType::Callable,
                seql(
                    seqr(mat(Token::LParen), icl(inv(parse_fulltype), Token::Arrow)),
                    mat(Token::RParen),
                ),
            ),
        )
            .oneof(),
        cvt(|m| m.is_some(), maybe(Token::Ampersand, epsilon())),
    )
    .parse(lexer)?;

    Ok(FullType { kind, base })
}

fn parse_param_list(lexer: &mut Lex) -> Result<Vec<Var>, CompileErr> {
    icl(
        cvt(
            |(name, tp)| Var { name, tp },
            seqlr(
                withannot(extid()),
                seqr(mat(Token::Colon), withannot(inv(parse_fulltype))),
            ),
        ),
        Token::Comma,
    )
    .parse(lexer)
}

fn parse_func_decl(lexer: &mut Lex) -> Result<FuncDecl, CompileErr> {
    let (name, params) = seqlr(
        withannot(extid()),
        seql(
            seqr(mat(Token::LParen), inv(parse_param_list)),
            mat(Token::RParen),
        ),
    )
    .parse(lexer)?;

    let rtp = seqr(mat(Token::Arrow), withannot(inv(parse_fulltype))).parse(lexer)?;

    Ok(FuncDecl { name, params, rtp })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Doing these individually for readability
    fn check_enumdef(testname: &str, actual: EnumerationDef, expect: EnumerationDef) {
        assert_eq!(expect.name, actual.name);
        assert_eq!(expect.tp, actual.tp);
        for (e, a) in std::iter::zip(&expect.ents, &actual.ents) {
            assert_eq!(*e, *a);
        }
        match expect.ents.len().cmp(&actual.ents.len()) {
            Ordering::Less => panic!(
                "Enum declarations differ for test case '{}': {} additional unexpected entries",
                testname,
                actual.ents.len() - expect.ents.len()
            ),
            Ordering::Greater => panic!(
                "Enum declarations differ for test case '{}': {} missing entries",
                testname,
                expect.ents.len() - actual.ents.len()
            ),
            _ => (),
        }
    }

    #[test]
    fn test_fulltype() {
        assert_eq!(
            parse_fulltype(&mut Lex::new(String::from(r"maybe list& i8&"))).expect("Parse failed"),
            FullType {
                kind: vec![(TypeKind::Maybe, false), (TypeKind::List, true)],
                base: (BaseType::Int8, true),
            }
        );

        assert_eq!(
            parse_fulltype(&mut Lex::new(String::from(
                r"(maybe list i8 -> float -> bool)"
            )))
            .expect("Parse failed"),
            FullType {
                kind: vec![],
                base: (
                    BaseType::Callable(vec![
                        FullType {
                            kind: vec![(TypeKind::Maybe, false), (TypeKind::List, false)],
                            base: (BaseType::Int8, false),
                        },
                        FullType {
                            kind: vec![],
                            base: (BaseType::Flt, false),
                        },
                        FullType {
                            kind: vec![],
                            base: (BaseType::Bool, false),
                        }
                    ]),
                    false
                ),
            }
        );
    }

    #[test]
    fn test_enumparse() {
        check_enumdef(
            "generic",
            parse_enum(&mut Lex::new(String::from(
                r"
enum0 {
    Entry0,
    Entry1 = 1 << 8 + 2,
    Entry2 = 5 + 6 * 9 ^ 0,
    Entry3 = 3 * (2 + 3),
}
",
            )))
            .expect("Parse failed"),
            EnumerationDef {
                name: annot(SourceLoc::new(2, 1, 1, 5), String::from("enum0")),
                tp: annot(SL_NIL, BaseType::Int32),
                ents: vec![
                    (
                        annot(SourceLoc::new(3, 5, 13, 6), String::from("Entry0")),
                        annot(SL_NIL, 0),
                    ),
                    (
                        annot(SourceLoc::new(4, 5, 25, 6), String::from("Entry1")),
                        annot(SourceLoc::new(4, 14, 34, 10), 1 << (8 + 2)),
                    ),
                    (
                        annot(SourceLoc::new(5, 5, 50, 6), String::from("Entry2")),
                        annot(SourceLoc::new(5, 14, 59, 13), 5 + (6 * 9)),
                    ),
                    (
                        annot(SourceLoc::new(6, 5, 78, 6), String::from("Entry3")),
                        annot(SourceLoc::new(6, 14, 87, 11), 15),
                    ),
                ],
            },
        );
    }
}
