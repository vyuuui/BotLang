use crate::compile::ast::*;
use crate::compile::lex::{mkident, Lex, Token, TokenDiscriminants};
use crate::compile::parsec::{
    alt, cvt, epsilon, extdbl, extflt, extid, extint, extstr, flagged, icl, ifx, inv, log, mat_tp,
    mat_val, maybeinv, maybeseqlr, pfx, rep, seql, seqlr, seqr, withannot, wrbracket, wrcurly,
    wrparen, Parser, ToOneOf,
};
use crate::compile::{annot, annot_nil, CompileErr, LocationAnnot, SourceLoc, SL_NIL};
use std::cmp::Ordering;
use std::rc::Rc;

struct SharedParsers {
    enum_expr: Rc<dyn Parser<u64, SharedParsers>>,
    fulltype: Rc<dyn Parser<FullType, SharedParsers>>,
    param_list: Rc<dyn Parser<Vec<Var>, SharedParsers>>,
    noblock_pexpr: Rc<dyn Parser<AnnotPExpr, SharedParsers>>,
    block_pexpr: Rc<dyn Parser<AnnotPExpr, SharedParsers>>,
    pexpr_optree: Rc<dyn Parser<AnnotPExpr, SharedParsers>>,
    pexpr_value: Rc<dyn Parser<AnnotPExpr, SharedParsers>>,
}

macro_rules! mk_invoke_pair {
    ($field:ident $ll1_nm:ident $parse_nm:ident $rtp:ty) => {
        fn $ll1_nm(pt: &SharedParsers, lexer: &mut Lex) -> Result<bool, CompileErr> {
            pt.$field.ll1(pt, lexer)
        }
        fn $parse_nm(pt: &SharedParsers, lexer: &mut Lex) -> Result<$rtp, CompileErr> {
            pt.$field.parse(pt, lexer)
        }
    };
}

pub fn parse(script: String) -> Result<Script, CompileErr> {
    let mut lexer = Lex::new(script);
    let mut res = Script::default();
    let parsers: SharedParsers = build_shared_parsers();
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
                res.enums.push(parse_enum(&parsers, &mut lexer)?);
            }
            Token::Identifier(i) if i == "interface" => {}
            Token::Identifier(i) if i == "behavior" => {}
            Token::Identifier(i) if i == "fn" => {
                lexer.eat();
                res.funcs.push(parse_func_def(&parsers, &mut lexer)?);
            }
            Token::Identifier(i) if i == "extern" => {
                lexer.eat();
                res.externs.push(parse_func_decl(&parsers, &mut lexer)?);
            }
            _ => return Err(CompileErr::Message("Expected valid top-level definition")),
        }
    }
    Ok(res)
}

fn build_shared_parsers() -> SharedParsers {
    SharedParsers {
        enum_expr: Rc::new(gen_enum_expr_parser()),
        fulltype: Rc::new(gen_fulltype_parser()),
        param_list: Rc::new(gen_param_list_parser()),
        noblock_pexpr: Rc::new(gen_noblock_pexpr_parser()),
        block_pexpr: Rc::new(gen_block_pexpr_parser()),
        pexpr_optree: Rc::new(gen_pexpr_optree_parser()),
        pexpr_value: Rc::new(gen_pexpr_value_parser()),
    }
}

fn gen_enum_expr_parser() -> impl Parser<u64, SharedParsers> {
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
    ifx(
        ifx(
            ifx(
                ifx(
                    ifx(
                        ifx(
                            pfx(
                                &[Token::Dash, Token::Tilde],
                                eval_unop,
                                (extint(), wrparen(inv(ll1_enum_expr, parse_enum_expr))).oneof(),
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
    )
}

fn ll1_enum_expr(pt: &SharedParsers, lexer: &mut Lex) -> Result<bool, CompileErr> {
    pt.enum_expr.ll1(pt, lexer)
}

fn parse_enum_expr(pt: &SharedParsers, lexer: &mut Lex) -> Result<u64, CompileErr> {
    pt.enum_expr.parse(pt, lexer)
}

fn parse_enum(pt: &SharedParsers, lexer: &mut Lex) -> Result<EnumerationDef, CompileErr> {
    let (nm, mtp) = seqlr(
        withannot(extid()),
        flagged(Token::Colon, withannot(extid())),
    )
    .parse(pt, lexer)?;

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
        flagged(Token::Equal, withannot(inv(ll1_enum_expr, parse_enum_expr))),
    );
    let enum_body = wrcurly(seql(
        icl(enum_ent, Token::Comma),
        flagged(Token::Comma, epsilon()),
    ));

    ret.ents = enum_body
        .parse(pt, lexer)?
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

fn gen_fulltype_parser() -> impl Parser<FullType, SharedParsers> {
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
    let kind_parse = rep(seqlr(
        (
            cvt(|_| TypeKind::List, mat_val(mkident("list"))),
            cvt(|_| TypeKind::Maybe, mat_val(mkident("maybe"))),
            cvt(|_| TypeKind::View, mat_val(mkident("view"))),
        )
            .oneof(),
        cvt(|m| m.is_some(), flagged(Token::Ampersand, epsilon())),
    ));

    let base_parse = seqlr(
        (
            // TODO: check for errors if basetype overlaps with keywords
            cvt(basetype_id, extid()),
            cvt(
                BaseType::Callable,
                wrparen(icl(inv(ll1_fulltype, parse_fulltype), Token::Arrow)),
            ),
        )
            .oneof(),
        cvt(|m| m.is_some(), flagged(Token::Ampersand, epsilon())),
    );

    cvt(
        |(kind, base)| FullType { kind, base },
        seqlr(kind_parse, base_parse),
    )
}
fn ll1_fulltype(pt: &SharedParsers, lexer: &mut Lex) -> Result<bool, CompileErr> {
    pt.fulltype.ll1(pt, lexer)
}
fn parse_fulltype(pt: &SharedParsers, lexer: &mut Lex) -> Result<FullType, CompileErr> {
    pt.fulltype.parse(pt, lexer)
}

fn gen_param_list_parser() -> impl Parser<Vec<Var>, SharedParsers> {
    icl(
        cvt(
            |(name, tp)| Var { name, tp },
            seqlr(
                withannot(extid()),
                seqr(
                    mat_tp(TokenDiscriminants::Colon),
                    withannot(inv(ll1_fulltype, parse_fulltype)),
                ),
            ),
        ),
        Token::Comma,
    )
}
fn ll1_param_list(pt: &SharedParsers, lexer: &mut Lex) -> Result<bool, CompileErr> {
    pt.param_list.ll1(pt, lexer)
}
fn parse_param_list(pt: &SharedParsers, lexer: &mut Lex) -> Result<Vec<Var>, CompileErr> {
    pt.param_list.parse(pt, lexer)
}

fn parse_func_decl(pt: &SharedParsers, lexer: &mut Lex) -> Result<FuncDecl, CompileErr> {
    let (name, params) = seqlr(
        withannot(extid()),
        wrparen(inv(ll1_param_list, parse_param_list)),
    )
    .parse(pt, lexer)?;

    let rtp = seqr(
        mat_tp(TokenDiscriminants::Arrow),
        withannot(inv(ll1_fulltype, parse_fulltype)),
    )
    .parse(pt, lexer)?;

    Ok(FuncDecl { name, params, rtp })
}

fn gen_noblock_pexpr_parser() -> impl Parser<AnnotPExpr, SharedParsers> {
    let guard = withannot(cvt(
        |(cond, mret)| {
            Box::new(PExpr::Guard(
                cond,
                mret.unwrap_or_else(|| annot_nil(Box::new(VOIDEXPR))),
            ))
        },
        seqlr(
            seqr(mat_val(mkident("guard")), inv(ll1_pexpr, parse_pexpr)),
            flagged(Token::Arrow, inv(ll1_pexpr, parse_pexpr)),
        ),
    ));

    let brk = withannot(cvt(|_| Box::new(PExpr::Break), mat_val(mkident("break"))));

    let cont = withannot(cvt(
        |_| Box::new(PExpr::Continue),
        mat_val(mkident("continue")),
    ));

    let ret = withannot(cvt(
        |mexpr| {
            Box::new(PExpr::Return(
                mexpr.unwrap_or_else(|| annot_nil(Box::new(VOIDEXPR))),
            ))
        },
        seqr(mat_val(mkident("return")), maybeinv(ll1_pexpr, parse_pexpr)),
    ));

    let op_tree = inv(ll1_pexpr_optree, parse_pexpr_optree);
    (guard, brk, cont, ret, op_tree).oneof()
}
mk_invoke_pair!(noblock_pexpr ll1_noblock_pexpr parse_noblock_pexpr AnnotPExpr);

fn gen_block_pexpr_parser() -> impl Parser<AnnotPExpr, SharedParsers> {
    let if_e = cvt(
        |ann_pair| {
            if let Some(els) = ann_pair.inner.1 {
                annot(
                    ann_pair.loc,
                    Box::new(PExpr::IfElse(ann_pair.inner.0 .0, ann_pair.inner.0 .1, els)),
                )
            } else {
                annot(
                    ann_pair.loc,
                    Box::new(PExpr::If(ann_pair.inner.0 .0, ann_pair.inner.0 .1)),
                )
            }
        },
        withannot(seqlr(
            seqr(
                mat_val(mkident("if")),
                seqlr(inv(ll1_pexpr, parse_pexpr), inv(ll1_pblock, parse_pblock)),
            ),
            flagged(mkident("else"), inv(ll1_pblock, parse_pblock)),
        )),
    );

    let while_e = cvt(
        |ann| annot(ann.loc, Box::new(PExpr::While(ann.inner.0, ann.inner.1))),
        withannot(seqlr(
            seqr(mat_val(mkident("if")), inv(ll1_pexpr, parse_pexpr)),
            inv(ll1_pblock, parse_pblock),
        )),
    );

    let for_e = cvt(
        |ann| {
            annot(
                ann.loc,
                Box::new(PExpr::For(ann.inner.0 .0, ann.inner.0 .1, ann.inner.1)),
            )
        },
        withannot(seqlr(
            seqlr(
                seqr(mat_val(mkident("for")), withannot(extid())),
                inv(ll1_pexpr, parse_pexpr),
            ),
            inv(ll1_pblock, parse_pblock),
        )),
    );

    let mat_e = cvt(
        |ann| annot(ann.loc, Box::new(PExpr::Match(ann.inner.0, ann.inner.1))),
        withannot(seqlr(
            seqr(mat_val(mkident("match")), inv(ll1_pexpr, parse_pexpr)),
            wrcurly(rep(seqlr(
                inv(ll1_pexpr, parse_pexpr),
                seqr(
                    mat_tp(TokenDiscriminants::Arrow),
                    inv(ll1_pblock, parse_pblock),
                ),
            ))),
        )),
    );

    let case_e = cvt(
        |ann| annot(ann.loc, Box::new(PExpr::Case(ann.inner))),
        withannot(seqr(
            mat_val(mkident("case")),
            wrcurly(rep(seqlr(
                inv(ll1_pexpr, parse_pexpr),
                seqr(
                    mat_tp(TokenDiscriminants::Arrow),
                    inv(ll1_pblock, parse_pblock),
                ),
            ))),
        )),
    );

    (if_e, while_e, for_e, mat_e, case_e).oneof()
}
mk_invoke_pair!(block_pexpr ll1_block_pexpr parse_block_pexpr AnnotPExpr);

fn gen_pexpr_optree_parser() -> impl Parser<AnnotPExpr, SharedParsers> {
    let gen_parser = || {
        let combine_infix: fn(AnnotPExpr, Token, AnnotPExpr) -> AnnotPExpr =
            |lhs, op, rhs| -> AnnotPExpr {
                annot(
                    lhs.loc.to_right(&rhs.loc),
                    Box::new(match op {
                        Token::DoublePipe => PExpr::Binary(lhs, BinaryOp::Or, rhs),
                        Token::DoubleAmpersand => PExpr::Binary(lhs, BinaryOp::And, rhs),
                        Token::Pipe => PExpr::Binary(lhs, BinaryOp::BOr, rhs),
                        Token::Caret => PExpr::Binary(lhs, BinaryOp::BXor, rhs),
                        Token::Ampersand => PExpr::Binary(lhs, BinaryOp::BAnd, rhs),
                        Token::DoubleEqual => PExpr::Binary(lhs, BinaryOp::CmpEq, rhs),
                        Token::BangEqual => PExpr::Binary(lhs, BinaryOp::CmpNe, rhs),
                        Token::RAngle => PExpr::Binary(lhs, BinaryOp::CmpGt, rhs),
                        Token::LAngle => PExpr::Binary(lhs, BinaryOp::CmpLt, rhs),
                        Token::RAngleEqual => PExpr::Binary(lhs, BinaryOp::CmpGe, rhs),
                        Token::LAngleEqual => PExpr::Binary(lhs, BinaryOp::CmpLe, rhs),
                        Token::RRAngle => PExpr::Binary(lhs, BinaryOp::Rsh, rhs),
                        Token::LLAngle => PExpr::Binary(lhs, BinaryOp::Lsh, rhs),
                        Token::Plus => PExpr::Binary(lhs, BinaryOp::Add, rhs),
                        Token::Dash => PExpr::Binary(lhs, BinaryOp::Sub, rhs),
                        Token::Asterisk => PExpr::Binary(lhs, BinaryOp::Mul, rhs),
                        Token::FSlash => PExpr::Binary(lhs, BinaryOp::Div, rhs),
                        Token::Percent => PExpr::Binary(lhs, BinaryOp::Mod, rhs),
                        _ => panic!("Invalid infix operation for procedural expression"),
                    }),
                )
            };
        let combine_prefix: fn(Token, AnnotPExpr) -> AnnotPExpr = |op, sub| -> AnnotPExpr {
            annot(
                // TODO: Correct annotation
                sub.loc,
                Box::new(match op {
                    Token::Dash => PExpr::Unary(UnaryOp::Neg, sub),
                    Token::Tilde => PExpr::Unary(UnaryOp::BNot, sub),
                    Token::Bang => PExpr::Unary(UnaryOp::Not, sub),
                    _ => panic!("Invalid prefix operation for procedural expression"),
                }),
            )
        };

        let base_expr = (
            wrparen(inv(ll1_pexpr, parse_pexpr)),
            inv(ll1_pexpr_value, parse_pexpr_value),
            inv(ll1_block_pexpr, parse_block_pexpr),
        )
            .oneof();

        let memacc = withannot(seqr(
            mat_tp(TokenDiscriminants::Period),
            cvt(
                |id| PExpr::MemAcc(annot_nil(Box::new(PExpr::Nil)), id),
                withannot(extid()),
            ),
        ));
        let arrsub = withannot(wrbracket(cvt(
            |idx_expr| PExpr::Subscr(annot_nil(Box::new(PExpr::Nil)), idx_expr),
            inv(ll1_pexpr, parse_pexpr),
        )));
        let fncall = withannot(wrparen(cvt(
            |call_vec| PExpr::Call(annot_nil(Box::new(PExpr::Nil)), call_vec),
            icl(inv(ll1_pexpr, parse_pexpr), Token::Comma),
        )));
        let indir = cvt(
            |(base_expr, indir_nodes)| {
                let mut ret = base_expr;
                for indir in indir_nodes {
                    ret = annot(
                        ret.loc.to_right(&indir.loc),
                        Box::new(match indir.inner {
                            PExpr::MemAcc(_, ident) => PExpr::MemAcc(ret, ident),
                            PExpr::Subscr(_, idx) => PExpr::Subscr(ret, idx),
                            PExpr::Call(_, param) => PExpr::Call(ret, param),
                            _ => panic!("LValue parse has invalid indirection list"),
                        }),
                    );
                }
                ret
            },
            seqlr(
                base_expr,
                // Wish to do with an infix, but array subscript isn't infix
                rep((memacc, arrsub, fncall).oneof()),
            ),
        );

        let p0 = pfx(
            &[Token::Dash, Token::Tilde, Token::Bang],
            combine_prefix,
            cvt(
                |(bexpr, mconv)| {
                    if let Some(tp) = mconv {
                        annot(
                            bexpr.loc.to_right(&tp.loc),
                            Box::new(PExpr::Cast(bexpr, tp)),
                        )
                    } else {
                        bexpr
                    }
                },
                seqlr(
                    indir,
                    flagged(Token::Colon, withannot(inv(ll1_fulltype, parse_fulltype))),
                ),
            ),
        );

        let i5_10 = ifx(
            ifx(
                ifx(
                    ifx(
                        ifx(
                            ifx(
                                p0,
                                &[Token::Asterisk, Token::FSlash, Token::Percent],
                                combine_infix,
                            ),
                            &[Token::Plus, Token::Dash],
                            combine_infix,
                        ),
                        &[Token::RRAngle, Token::LLAngle],
                        combine_infix,
                    ),
                    &[
                        Token::RAngle,
                        Token::LAngle,
                        Token::RAngleEqual,
                        Token::LAngleEqual,
                    ],
                    combine_infix,
                ),
                &[
                    Token::RAngle,
                    Token::LAngle,
                    Token::RAngleEqual,
                    Token::LAngleEqual,
                ],
                combine_infix,
            ),
            &[Token::DoubleEqual, Token::BangEqual],
            combine_infix,
        );

        ifx(
            ifx(
                ifx(
                    ifx(
                        ifx(i5_10, &[Token::Ampersand], combine_infix),
                        &[Token::Caret],
                        combine_infix,
                    ),
                    &[Token::Pipe],
                    combine_infix,
                ),
                &[Token::DoubleAmpersand],
                combine_infix,
            ),
            &[Token::DoublePipe],
            combine_infix,
        )
    };

    type TD = TokenDiscriminants;
    let assn_ops = (
        mat_tp(TD::Equal),
        mat_tp(TD::PlusEqual),
        mat_tp(TD::DashEqual),
        mat_tp(TD::AsteriskEqual),
        mat_tp(TD::FSlashEqual),
        mat_tp(TD::PercentEqual),
        mat_tp(TD::AmpersandEqual),
        mat_tp(TD::PipeEqual),
        mat_tp(TD::TildeEqual),
        mat_tp(TD::LLAngleEqual),
        mat_tp(TD::RRAngleEqual),
    )
        .oneof();
    cvt(
        |(lhs, mrhs)| {
            if let Some((op, rhs)) = mrhs {
                annot(
                    lhs.loc.to_right(&rhs.loc),
                    Box::new(match op {
                        TD::Equal => PExpr::Assign(lhs, AssignOp::None, rhs),
                        TD::PlusEqual => PExpr::Assign(lhs, AssignOp::Add, rhs),
                        TD::DashEqual => PExpr::Assign(lhs, AssignOp::Sub, rhs),
                        TD::AsteriskEqual => PExpr::Assign(lhs, AssignOp::Mul, rhs),
                        TD::FSlashEqual => PExpr::Assign(lhs, AssignOp::Div, rhs),
                        TD::PercentEqual => PExpr::Assign(lhs, AssignOp::Mod, rhs),
                        TD::AmpersandEqual => PExpr::Assign(lhs, AssignOp::And, rhs),
                        TD::PipeEqual => PExpr::Assign(lhs, AssignOp::Or, rhs),
                        TD::TildeEqual => PExpr::Assign(lhs, AssignOp::Not, rhs),
                        TD::LLAngleEqual => PExpr::Assign(lhs, AssignOp::Lsh, rhs),
                        TD::RRAngleEqual => PExpr::Assign(lhs, AssignOp::Rsh, rhs),
                        _ => panic!("Invalid assignment operation for procedural expression"),
                    }),
                )
            } else {
                lhs
            }
        },
        seqlr(gen_parser(), maybeseqlr(assn_ops, gen_parser())),
    )
}
mk_invoke_pair!(pexpr_optree ll1_pexpr_optree parse_pexpr_optree AnnotPExpr);

fn gen_pexpr_value_parser() -> impl Parser<AnnotPExpr, SharedParsers> {
    // TODO: exclude keyword identifiers (e.g. true, false, if, else, etc...)
    withannot(
        (
            cvt(|id| Box::new(PExpr::Ident(id)), extid()),
            cvt(
                |lit| Box::new(PExpr::Lit(lit)),
                (
                    cvt(Literal::Integral, extint()),
                    cvt(Literal::Flt, extflt()),
                    cvt(Literal::Dbl, extdbl()),
                    cvt(Literal::Str, extstr()),
                )
                    .oneof(),
            ),
        )
            .oneof(),
    )
}
mk_invoke_pair!(pexpr_value ll1_pexpr_value parse_pexpr_value AnnotPExpr);

fn ll1_pexpr(pt: &SharedParsers, lexer: &mut Lex) -> Result<bool, CompileErr> {
    Ok(pt.block_pexpr.ll1(pt, lexer)? || pt.noblock_pexpr.ll1(pt, lexer)?)
}
fn parse_pexpr(pt: &SharedParsers, lexer: &mut Lex) -> Result<AnnotPExpr, CompileErr> {
    if pt.block_pexpr.ll1(pt, lexer)? {
        pt.block_pexpr.parse(pt, lexer)
    } else if pt.noblock_pexpr.ll1(pt, lexer)? {
        pt.noblock_pexpr.parse(pt, lexer)
    } else {
        Err(CompileErr::OneOfFailure(lexer.peek()?.inner.clone()))
    }
}

fn ll1_pblock(pt: &SharedParsers, lexer: &mut Lex) -> Result<bool, CompileErr> {
    mat_tp(TokenDiscriminants::LCurly).ll1(pt, lexer)
}
// parse_pblock requires a handwritten parser due to the ambiguity of the final expression in a
// block being terminated with or without a semicolon
fn parse_pblock(pt: &SharedParsers, lexer: &mut Lex) -> Result<AnnotPExpr, CompileErr> {
    let var_decl = withannot(cvt(
        |(v, e)| Box::new(PExpr::VarDecl(v, e)),
        seql(
            seqlr(
                cvt(
                    |(name, tp)| Var { name, tp },
                    seqlr(
                        seqr(mat_val(mkident("var")), withannot(extid())),
                        seqr(
                            mat_tp(TokenDiscriminants::Colon),
                            withannot(inv(ll1_fulltype, parse_fulltype)),
                        ),
                    ),
                ),
                seqr(
                    mat_tp(TokenDiscriminants::Equal),
                    (
                        inv(ll1_noblock_pexpr, parse_noblock_pexpr),
                        inv(ll1_block_pexpr, parse_block_pexpr),
                    )
                        .oneof(),
                ),
            ),
            mat_tp(TokenDiscriminants::Semicolon),
        ),
    ));

    let semi = withannot(cvt(
        |_| Box::new(VOIDEXPR),
        mat_tp(TokenDiscriminants::Semicolon),
    ));

    let rcurly = mat_tp(TokenDiscriminants::RCurly);

    let mut seq_vec: Vec<AnnotPExpr> = Vec::new();
    let mut res_expr: Option<AnnotPExpr> = None;

    let ann_begin = lexer.head_loc();
    mat_tp(TokenDiscriminants::LCurly).parse(pt, lexer)?;
    loop {
        if var_decl.ll1(pt, lexer)? {
            seq_vec.push(var_decl.parse(pt, lexer)?);
        }
        if semi.ll1(pt, lexer)? {
            seq_vec.push(semi.parse(pt, lexer)?);
        }
        if ll1_noblock_pexpr(pt, lexer)? {
            let result = parse_noblock_pexpr(pt, lexer)?;
            if semi.ll1(pt, lexer)? {
                seq_vec.push(result);
                lexer.eat();
            } else {
                res_expr = Some(result);
                // If we hit a non-block expression without a semicolon, the next token must be the
                // end of the block (RCurly)
                break;
            }
        }
        if ll1_block_pexpr(pt, lexer)? {
            let result = parse_block_pexpr(pt, lexer)?;
            if rcurly.ll1(pt, lexer)? {
                res_expr = Some(result);
                break;
            } else {
                seq_vec.push(result);
                if semi.ll1(pt, lexer)? {
                    lexer.eat();
                }
            }
        } else {
            break;
        }
    }
    mat_tp(TokenDiscriminants::RCurly).parse(pt, lexer)?;
    Ok(annot(
        ann_begin.to_right(&lexer.prev_loc()),
        Box::new(PExpr::Block(seq_vec, res_expr)),
    ))
}

fn parse_func_def(pt: &SharedParsers, lexer: &mut Lex) -> Result<FuncDef, CompileErr> {
    let (name, params) = seqlr(
        withannot(extid()),
        wrparen(inv(ll1_param_list, parse_param_list)),
    )
    .parse(pt, lexer)?;

    let rtp = seqr(
        mat_tp(TokenDiscriminants::Arrow),
        withannot(inv(ll1_fulltype, parse_fulltype)),
    )
    .parse(pt, lexer)?;

    let body = parse_pblock(pt, lexer)?;

    Ok(FuncDef {
        name,
        params,
        rtp,
        body,
    })
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

    fn check_funcdecl(testname: &str, actual: FuncDecl, expect: FuncDecl) {
        assert_eq!(expect.name, actual.name);
        for (e, a) in std::iter::zip(&expect.params, &actual.params) {
            assert_eq!(*e, *a);
        }
        match expect.params.len().cmp(&actual.params.len()) {
            Ordering::Less => panic!(
                "Parameter lists differ for test case '{}': {} additional unexpected parameters",
                testname,
                actual.params.len() - expect.params.len()
            ),
            Ordering::Greater => panic!(
                "Parameter lists differ for test case '{}': {} missing parameters",
                testname,
                expect.params.len() - actual.params.len()
            ),
            _ => (),
        }
        assert_eq!(expect.rtp, actual.rtp);
    }

    #[test]
    fn test_extern() {
        let parsers = build_shared_parsers();
        check_funcdecl(
            "generic",
            parse_func_decl(
                &parsers,
                &mut Lex::new(String::from(
                    "testfunc(a: i32, b:float, c: maybe& u64) -> bool",
                )),
            )
            .expect("Parse failed"),
            FuncDecl {
                name: annot(SourceLoc::new(1, 1, 0, 8), String::from("testfunc")),
                params: vec![
                    Var {
                        name: annot(SourceLoc::new(1, 10, 9, 1), String::from("a")),
                        tp: annot(SourceLoc::new(1, 13, 12, 3), mkfulltype!(i32)),
                    },
                    Var {
                        name: annot(SourceLoc::new(1, 18, 17, 1), String::from("b")),
                        tp: annot(SourceLoc::new(1, 20, 19, 5), mkfulltype!(float)),
                    },
                    Var {
                        name: annot(SourceLoc::new(1, 27, 26, 1), String::from("c")),
                        tp: annot(SourceLoc::new(1, 30, 29, 10), mkfulltype!(maybe & u64)),
                    },
                ],
                rtp: annot(SourceLoc::new(1, 45, 44, 4), mkfulltype!(bool)),
            },
        );
    }

    #[test]
    fn test_fulltype() {
        let parsers = build_shared_parsers();
        assert_eq!(
            parse_fulltype(&parsers, &mut Lex::new(String::from(r"maybe list& i8&")))
                .expect("Parse failed"),
            mkfulltype!(maybe list& i8&),
        );

        assert_eq!(
            parse_fulltype(
                &parsers,
                &mut Lex::new(String::from(r"maybe (maybe list i8 -> float -> bool)"))
            )
            .expect("Parse failed"),
            mkfulltype!(maybe (maybe list i8 -> float -> bool)),
        );
        assert_eq!(
            parse_fulltype(
                &parsers,
                &mut Lex::new(String::from(r"view view& view& ((i32&) -> i32)"))
            )
            .expect("Parse failed"),
            mkfulltype!(view view& view& ((i32&) -> i32)),
        );
        assert_eq!(
            parse_fulltype(
                &parsers,
                &mut Lex::new(String::from(
                    r"((a -> maybe b) -> (b -> maybe c) -> (a -> maybe c))"
                ))
            )
            .expect("Parse failed"),
            mkfulltype!(((a -> maybe b) -> (b -> maybe c) -> (a -> maybe c))),
        );
    }

    #[test]
    fn test_funcparse() {
        let parsers = build_shared_parsers();
        let res = parse_func_def(
            &parsers,
            &mut Lex::new(String::from(
                r"
func(x: i32, y: i32) -> maybe i32 {
    var a: maybe view i32 = mkmaybe(mkview(1 + 6));
    a[1 + a](3, 6) = 3 | 6 + 1

}",
            )),
        )
        .expect("Parse failed");
    }

    #[test]
    fn test_enumparse() {
        let parsers = build_shared_parsers();
        check_enumdef(
            "generic",
            parse_enum(
                &parsers,
                &mut Lex::new(String::from(
                    r"
enum0 {
    Entry0,
    Entry1 = 1 << 8 + 2,
    Entry2 = 5 + 6 * 9 ^ 0,
    Entry3 = 3 * (2 + 3),
}
",
                )),
            )
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
