use crate::compile::lex::{AnnotTok, Lex, Token, TokenDiscriminants};
use crate::compile::{annot, CompileErr, LocationAnnot, SL_NIL};
use std::marker::PhantomData;

pub trait Parser<Out> {
    const SKIP: bool;
    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr>;
    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Out>, CompileErr>;
}

pub struct Epsilon;

impl Parser<()> for Epsilon {
    const SKIP: bool = true;

    fn ll1(&self, _: &mut Lex) -> Result<bool, CompileErr> {
        Ok(true)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<()>, CompileErr> {
        Ok(annot(l.cursor, ()))
    }
}

pub struct Invoke<Out> {
    f: fn(&mut Lex) -> Result<LocationAnnot<Out>, CompileErr>,
}

impl<Out> Parser<Out> for Invoke<Out> {
    const SKIP: bool = false;

    fn ll1(&self, _: &mut Lex) -> Result<bool, CompileErr> {
        Ok(true)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Out>, CompileErr> {
        (self.f)(l)
    }
}

pub struct Convert<In, Out, P: Parser<In>> {
    sub: P,
    f: fn(In) -> Out,
}

impl<In, Out, P: Parser<In>> Parser<Out> for Convert<In, Out, P> {
    const SKIP: bool = false;

    fn ll1(&self, _: &mut Lex) -> Result<bool, CompileErr> {
        Ok(true)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Out>, CompileErr> {
        let res = self.sub.parse(l)?;
        Ok(annot(res.loc, (self.f)(res.inner)))
    }
}

pub struct Match<Out> {
    t: Token,
    marker: PhantomData<Out>,
}

impl Parser<()> for Match<()> {
    const SKIP: bool = false;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        Ok(l.peek()?.inner == self.t)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<()>, CompileErr> {
        let tok = l.peek()?;
        if tok.inner == self.t {
            (Ok(annot(tok.loc, ())), l.eat()).0
        } else {
            Err(CompileErr::ValMismatch(self.t.clone(), tok.inner.clone()))
        }
    }
}

pub struct ExtractIdent;

impl Parser<String> for ExtractIdent {
    const SKIP: bool = false;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        Ok(TokenDiscriminants::Identifier == (&l.peek()?.inner).into())
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<String>, CompileErr> {
        let tok = l.peek()?;
        if let AnnotTok {
            loc: sl,
            inner: Token::Identifier(s),
        } = tok
        {
            let ret = LocationAnnot::new(*sl, s.clone());
            l.eat();
            Ok(ret)
        } else {
            Err(CompileErr::TypeMismatch(
                TokenDiscriminants::Identifier,
                tok.inner.clone(),
            ))
        }
    }
}

pub struct ExtractInt;

impl Parser<u64> for ExtractInt {
    const SKIP: bool = false;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        Ok(TokenDiscriminants::IntegralLiteral == (&l.peek()?.inner).into())
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<u64>, CompileErr> {
        let tok = l.peek()?;
        if let AnnotTok {
            loc: sl,
            inner: Token::IntegralLiteral(i),
        } = tok
        {
            let ret = LocationAnnot::new(*sl, *i);
            l.eat();
            Ok(ret)
        } else {
            Err(CompileErr::TypeMismatch(
                TokenDiscriminants::Identifier,
                tok.inner.clone(),
            ))
        }
    }
}

pub struct Repeat<Out, P: Parser<Out>> {
    sub: P,
    marker: PhantomData<Out>,
}

impl<Out, P: Parser<Out>> Parser<Vec<LocationAnnot<Out>>> for Repeat<Out, P> {
    const SKIP: bool = true;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(l)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Vec<LocationAnnot<Out>>>, CompileErr> {
        let mut res = annot(SL_NIL, Vec::new());
        while self.ll1(l)? {
            let nelem = self.sub.parse(l)?;
            res.loc.accum(&nelem.loc);
            res.inner.push(nelem);
        }

        Ok(res)
    }
}

pub struct Intercalate<Out, P: Parser<Out>> {
    sub: P,
    int: Token,
    marker: PhantomData<Out>,
}

// TODO: This is fake intercalation for the time being
impl<Out, P: Parser<Out>> Parser<Vec<LocationAnnot<Out>>> for Intercalate<Out, P> {
    const SKIP: bool = true;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(l)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Vec<LocationAnnot<Out>>>, CompileErr> {
        let mut res = annot(SL_NIL, Vec::new());

        while self.ll1(l)? {
            let nelem = self.sub.parse(l)?;
            res.loc.accum(&nelem.loc);
            res.inner.push(nelem);
            let int = l.peek()?;
            if int.inner != self.int {
                break;
            }
            res.loc.accum(&int.loc);
            l.eat();
        }

        Ok(res)
    }
}

pub struct InfixFold<Out, P: Parser<Out>> {
    sub: P,
    int: Vec<Token>,
    combine: fn(Out, Token, Out) -> Out,
}

impl<Out, P: Parser<Out>> Parser<Out> for InfixFold<Out, P> {
    const SKIP: bool = P::SKIP;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(l)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Out>, CompileErr> {
        let mut res = self.sub.parse(l)?;
        while self.int.contains(&l.peek()?.inner) {
            let inter = l.peek()?.inner.clone();
            l.eat();
            let next = self.sub.parse(l)?;
            res.loc.accum(&next.loc);
            res.inner = (self.combine)(res.inner, inter, next.inner);
        }
        Ok(res)
    }
}

pub struct PrefixFold<Out, P: Parser<Out>> {
    sub: P,
    pre: Vec<Token>,
    combine: fn(Token, Out) -> Out,
}

impl<Out, P: Parser<Out>> Parser<Out> for PrefixFold<Out, P> {
    const SKIP: bool = P::SKIP;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(l)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Out>, CompileErr> {
        let mut pref_list: Vec<AnnotTok> = Vec::new();
        while self.pre.contains(&l.peek()?.inner) {
            pref_list.push(l.peek()?.clone());
            l.eat();
        }
        let mut res = self.sub.parse(l)?;
        for tok in pref_list.into_iter().rev() {
            res.loc.accum(&tok.loc);
            res.inner = (self.combine)(tok.inner, res.inner);
        }
        Ok(res)
    }
}

pub struct SeqL<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>> {
    lhs: PL,
    rhs: PR,
    markl: PhantomData<OutL>,
    markr: PhantomData<OutR>,
}

impl<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>> Parser<OutL> for SeqL<OutL, OutR, PL, PR> {
    const SKIP: bool = PL::SKIP && PR::SKIP;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        if PL::SKIP {
            Ok(self.lhs.ll1(l)? || self.rhs.ll1(l)?)
        } else {
            self.lhs.ll1(l)
        }
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<OutL>, CompileErr> {
        let mut res = self.lhs.parse(l)?;
        res.loc.accum(&self.rhs.parse(l)?.loc);
        Ok(res)
    }
}

pub struct SeqR<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>> {
    lhs: PL,
    rhs: PR,
    markl: PhantomData<OutL>,
    markr: PhantomData<OutR>,
}

impl<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>> Parser<OutR> for SeqR<OutL, OutR, PL, PR> {
    const SKIP: bool = PL::SKIP && PR::SKIP;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        if PL::SKIP {
            Ok(self.lhs.ll1(l)? || self.rhs.ll1(l)?)
        } else {
            self.lhs.ll1(l)
        }
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<OutR>, CompileErr> {
        let lhs_loc = self.lhs.parse(l)?.loc;
        let mut res = self.rhs.parse(l)?;
        res.loc.accum(&lhs_loc);
        Ok(res)
    }
}

pub struct SeqLR<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>> {
    lhs: PL,
    rhs: PR,
    markl: PhantomData<OutL>,
    markr: PhantomData<OutR>,
}

impl<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>> Parser<(LocationAnnot<OutL>, LocationAnnot<OutR>)>
    for SeqLR<OutL, OutR, PL, PR>
{
    const SKIP: bool = PL::SKIP && PR::SKIP;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        if PL::SKIP {
            Ok(self.lhs.ll1(l)? || self.rhs.ll1(l)?)
        } else {
            self.lhs.ll1(l)
        }
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<(LocationAnnot<OutL>, LocationAnnot<OutR>)>, CompileErr> {
        let left = self.lhs.parse(l)?;
        let right = self.rhs.parse(l)?;
        let mut acc_loc = left.loc;
        acc_loc.accum(&right.loc);
        Ok(annot(acc_loc, (left, right)))
    }
}

pub struct Maybe<Out, P: Parser<Out>> {
    t: Token,
    sub: P,
    marker: PhantomData<Out>,
}

impl<Out, P: Parser<Out>> Parser<Option<LocationAnnot<Out>>> for Maybe<Out, P> {
    const SKIP: bool = true;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        Ok(self.t == l.peek()?.inner)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Option<LocationAnnot<Out>>>, CompileErr> {
        if self.ll1(l)? {
            let mut m_loc = l.peek()?.loc;
            l.eat();
            let ret = self.sub.parse(l)?;
            m_loc.accum(&ret.loc);
            Ok(annot(m_loc, Some(ret)))
        } else {
            Ok(annot(l.cursor, None))
        }
    }
}

pub struct OneOf<Out, PL: Parser<Out>, PR: Parser<Out>> {
    lhs: PL,
    rhs: PR,
    marker: PhantomData<Out>,
}

impl<Out, PL: Parser<Out>, PR: Parser<Out>> Parser<Out> for OneOf<Out, PL, PR> {
    const SKIP: bool = PL::SKIP && PR::SKIP;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        Ok(self.lhs.ll1(l)? || self.rhs.ll1(l)?)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Out>, CompileErr> {
        if self.lhs.ll1(l)? {
            self.lhs.parse(l)
        } else if self.rhs.ll1(l)? {
            self.rhs.parse(l)
        } else {
            Err(CompileErr::OneOfFailure(l.peek()?.inner.clone()))
        }
    }
}

pub fn epsilon() -> Epsilon {
    Epsilon
}

pub fn inv<Out>(f: fn(&mut Lex) -> Result<LocationAnnot<Out>, CompileErr>) -> Invoke<Out> {
    Invoke { f }
}

pub fn cvt<In, Out, P: Parser<In>>(f: fn(In) -> Out, sub: P) -> Convert<In, Out, P> {
    Convert { f, sub }
}

pub fn mat(tok: Token) -> Match<()> {
    Match {
        t: tok,
        marker: PhantomData,
    }
}

pub fn extid() -> ExtractIdent {
    ExtractIdent
}

pub fn extint() -> ExtractInt {
    ExtractInt
}

pub fn rep<Out, P: Parser<Out>>(sub: P) -> Repeat<Out, P> {
    Repeat {
        sub,
        marker: PhantomData,
    }
}

pub fn icl<Out, P: Parser<Out>>(sub: P, int: Token) -> Intercalate<Out, P> {
    Intercalate {
        sub,
        int,
        marker: PhantomData,
    }
}

pub fn pfx<Out, P: Parser<Out>>(
    pre: &[Token],
    combine: fn(Token, Out) -> Out,
    sub: P,
) -> PrefixFold<Out, P> {
    PrefixFold {
        sub,
        pre: Vec::from(pre),
        combine,
    }
}

pub fn ifx<Out, P: Parser<Out>>(
    sub: P,
    int: &[Token],
    combine: fn(Out, Token, Out) -> Out,
) -> InfixFold<Out, P> {
    InfixFold {
        sub,
        int: Vec::from(int),
        combine,
    }
}

pub fn seql<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>>(
    lhs: PL,
    rhs: PR,
) -> SeqL<OutL, OutR, PL, PR> {
    SeqL {
        lhs,
        rhs,
        markl: PhantomData,
        markr: PhantomData,
    }
}

pub fn seqr<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>>(
    lhs: PL,
    rhs: PR,
) -> SeqR<OutL, OutR, PL, PR> {
    SeqR {
        lhs,
        rhs,
        markl: PhantomData,
        markr: PhantomData,
    }
}

pub fn seqlr<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>>(
    lhs: PL,
    rhs: PR,
) -> SeqLR<OutL, OutR, PL, PR> {
    SeqLR {
        lhs,
        rhs,
        markl: PhantomData,
        markr: PhantomData,
    }
}

pub fn maybe<Out, P: Parser<Out>>(t: Token, sub: P) -> Maybe<Out, P> {
    Maybe {
        t,
        sub,
        marker: PhantomData,
    }
}

pub fn oneof<Out, PL: Parser<Out>, PR: Parser<Out>>(lhs: PL, rhs: PR) -> OneOf<Out, PL, PR> {
    OneOf {
        lhs,
        rhs,
        marker: PhantomData,
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_lang() {}

    #[test]
    fn test_basic() {
        let p0 = pfx(
            &[Token::Plus, Token::Dash],
            |_, x| x,
            mat(Token::IntegralLiteral(3)),
        );
        p0.parse(&mut Lex::new(String::from("++-+3")))
            .expect("Parse failed");
        let p1 = ifx(
            extint(),
            &[Token::Plus, Token::Dash],
            |x, op, y| match (x, op, y) {
                (lhs, Token::Plus, rhs) => lhs + rhs,
                (lhs, Token::Dash, rhs) => lhs - rhs,
                _ => 0,
            },
        );
        assert_eq!(
            p1.parse(&mut Lex::new(String::from("3 + 4")))
                .expect("Parse failed").inner,
            7
        );
        assert_eq!(
            p1.parse(&mut Lex::new(String::from("3 + 4 + 5")))
                .expect("Parse failed").inner,
            12
        );
        assert_eq!(
            p1.parse(&mut Lex::new(String::from("3 + 4 - 5")))
                .expect("Parse failed").inner,
            2
        );

        let p2 = icl(p1, Token::Comma);
        assert_eq!(
            p2.parse(&mut Lex::new(String::from("1 + 2, 5, 30 + 5 - 6 + 1")))
                .expect("Parse failed").inner.into_iter().map(|x| x.inner).collect::<Vec<u64>>(),
            vec![3, 5, 30],
        );

        let p3 = seql(seql(mat(Token::Plus), mat(Token::Dash)), mat(Token::Plus));

        p3.parse(&mut Lex::new(String::from("+-+")))
            .expect("Parse failed");
    }
}
