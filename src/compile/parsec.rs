use crate::compile::lex::{Lex, Token, TokenDiscriminants};
use crate::compile::{annot, CompileErr, LocationAnnot};
use std::marker::PhantomData;

pub trait Parser<Out> {
    const SKIP: bool;
    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr>;
    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr>;
}

pub struct Epsilon;

impl Parser<()> for Epsilon {
    const SKIP: bool = true;

    fn ll1(&self, _: &mut Lex) -> Result<bool, CompileErr> {
        Ok(true)
    }

    fn parse(&self, _: &mut Lex) -> Result<(), CompileErr> {
        Ok(())
    }
}

pub struct WithAnnot<Out, P: Parser<Out>> {
    sub: P,
    marker: PhantomData<Out>,
}

impl<Out, P: Parser<Out>> Parser<LocationAnnot<Out>> for WithAnnot<Out, P> {
    const SKIP: bool = P::SKIP;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(l)
    }

    fn parse(&self, l: &mut Lex) -> Result<LocationAnnot<Out>, CompileErr> {
        let ann_begin = l.head_loc();
        let res = self.sub.parse(l)?;
        Ok(annot(ann_begin.to_right(&l.prev_loc()), res))
    }
}

pub struct Invoke<Out> {
    f: fn(&mut Lex) -> Result<Out, CompileErr>,
}

impl<Out> Parser<Out> for Invoke<Out> {
    const SKIP: bool = false;

    fn ll1(&self, _: &mut Lex) -> Result<bool, CompileErr> {
        Ok(true)
    }

    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr> {
        (self.f)(l)
    }
}

pub struct Convert<In, Out, P: Parser<In>> {
    sub: P,
    f: fn(In) -> Out,
}

impl<In, Out, P: Parser<In>> Parser<Out> for Convert<In, Out, P> {
    const SKIP: bool = P::SKIP;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(l)
    }

    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr> {
        self.sub.parse(l).map(self.f)
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

    fn parse(&self, l: &mut Lex) -> Result<(), CompileErr> {
        let tok = l.peek()?;
        if tok.inner == self.t {
            (Ok(()), l.eat()).0
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

    fn parse(&self, l: &mut Lex) -> Result<String, CompileErr> {
        let tok = l.peek()?;
        if let Token::Identifier(s) = &tok.inner {
            Ok((s.clone(), l.eat()).0)
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

    fn parse(&self, l: &mut Lex) -> Result<u64, CompileErr> {
        let tok = l.peek()?;
        if let Token::IntegralLiteral(i) = &tok.inner {
            Ok((*i, l.eat()).0)
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

impl<Out, P: Parser<Out>> Parser<Vec<Out>> for Repeat<Out, P> {
    const SKIP: bool = true;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(l)
    }

    fn parse(&self, l: &mut Lex) -> Result<Vec<Out>, CompileErr> {
        let mut res = Vec::new();
        while self.ll1(l)? {
            res.push(self.sub.parse(l)?);
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
impl<Out, P: Parser<Out>> Parser<Vec<Out>> for Intercalate<Out, P> {
    const SKIP: bool = true;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(l)
    }

    fn parse(&self, l: &mut Lex) -> Result<Vec<Out>, CompileErr> {
        let mut res = Vec::new();

        while self.ll1(l)? {
            res.push(self.sub.parse(l)?);
            let int = l.peek()?;
            if int.inner != self.int {
                break;
            }
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

    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr> {
        let mut res = self.sub.parse(l)?;
        while self.int.contains(&l.peek()?.inner) {
            let inter = l.peek()?.inner.clone();
            l.eat();
            let next = self.sub.parse(l)?;
            res = (self.combine)(res, inter, next);
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

    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr> {
        let mut pref_list: Vec<Token> = Vec::new();
        while self.pre.contains(&l.peek()?.inner) {
            pref_list.push(l.peek()?.inner.clone());
            l.eat();
        }
        let mut res = self.sub.parse(l)?;
        for tok in pref_list.into_iter().rev() {
            res = (self.combine)(tok, res);
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

    fn parse(&self, l: &mut Lex) -> Result<OutL, CompileErr> {
        Ok((self.lhs.parse(l)?, self.rhs.parse(l)?).0)
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

    fn parse(&self, l: &mut Lex) -> Result<OutR, CompileErr> {
        Ok((self.lhs.parse(l)?, self.rhs.parse(l)?).1)
    }
}

pub struct SeqLR<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>> {
    lhs: PL,
    rhs: PR,
    markl: PhantomData<OutL>,
    markr: PhantomData<OutR>,
}

impl<OutL, OutR, PL: Parser<OutL>, PR: Parser<OutR>> Parser<(OutL, OutR)>
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

    fn parse(&self, l: &mut Lex) -> Result<(OutL, OutR), CompileErr> {
        Ok((self.lhs.parse(l)?, self.rhs.parse(l)?))
    }
}

pub struct Maybe<Out, P: Parser<Out>> {
    t: Token,
    sub: P,
    marker: PhantomData<Out>,
}

impl<Out, P: Parser<Out>> Parser<Option<Out>> for Maybe<Out, P> {
    const SKIP: bool = true;

    fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
        Ok(self.t == l.peek()?.inner)
    }

    fn parse(&self, l: &mut Lex) -> Result<Option<Out>, CompileErr> {
        if self.ll1(l)? {
            Ok((l.eat(), Some(self.sub.parse(l)?)).1)
        } else {
            Ok(None)
        }
    }
}

pub trait ToOneOf<T> {
    fn oneof(self) -> T;
}

macro_rules! mkoneof {
    // Thankfully OneOf is pointless with a single subparser, so this doesn't need repeating for
    // the base case
    ($n0:ident $t0:ident $v0:ident) => {};

    ($n0:ident $t0:ident $v0:ident, $($nk:ident $tk:ident $vk:ident),*) => {
        mkoneof!($($nk $tk $vk),*);

        pub struct $n0<Out, $t0, $($tk),*> {
            tup: ($t0, $($tk),*),
            marker: PhantomData<Out>,
        }

        impl<Out, $t0: Parser<Out>, $($tk: Parser<Out>),*> Parser<Out> for $n0<Out, $t0, $($tk,)*> {
            const SKIP: bool = $t0::SKIP $(&& $tk::SKIP)*;

            fn ll1(&self, l: &mut Lex) -> Result<bool, CompileErr> {
                let ($v0, $($vk),*) = &self.tup;
                Ok($v0.ll1(l)? $(|| $vk.ll1(l)?)*)
            }

            fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr> {
                let ($v0, $($vk),*) = &self.tup;
                if $v0.ll1(l)? {
                    $v0.parse(l)
                } $(else if $vk.ll1(l)? {
                    $vk.parse(l)
                })* else {
                    Err(CompileErr::OneOfFailure(l.peek()?.inner.clone()))
                }
            }
        }

        impl<Out, $t0: Parser<Out>, $($tk: Parser<Out>),*> ToOneOf<$n0<Out, $t0, $($tk,)*>> for ($t0, $($tk,)*) {
            fn oneof(self) -> $n0<Out, $t0, $($tk,)*> {
                $n0 {
                    tup: self,
                    marker: PhantomData,
                }
            }
        }

    };
}

mkoneof!(OoA A a, OoB B b, OoC C c, OoD D d, OoE E e, OoF F f, OoG G g, OoH H h, OoI I i, OoJ J j, OoK K k, OoL L l, OoM M m, OoN N n, OoO O o, OoP P p, OoQ Q q, OoR R r, OoS S s, OoT T t);

pub fn epsilon() -> Epsilon {
    Epsilon
}

pub fn withannot<Out, P: Parser<Out>>(sub: P) -> WithAnnot<Out, P> {
    WithAnnot {
        sub,
        marker: PhantomData,
    }
}

pub fn inv<Out>(f: fn(&mut Lex) -> Result<Out, CompileErr>) -> Invoke<Out> {
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
        let p1 = ifx(extint(), &[Token::Plus, Token::Dash], |x, op, y| {
            match (x, op, y) {
                (lhs, Token::Plus, rhs) => lhs + rhs,
                (lhs, Token::Dash, rhs) => lhs - rhs,
                _ => 0,
            }
        });
        assert_eq!(
            p1.parse(&mut Lex::new(String::from("3 + 4")))
                .expect("Parse failed"),
            7
        );
        assert_eq!(
            p1.parse(&mut Lex::new(String::from("3 + 4 + 5")))
                .expect("Parse failed"),
            12
        );
        assert_eq!(
            p1.parse(&mut Lex::new(String::from("3 + 4 - 5")))
                .expect("Parse failed"),
            2
        );

        let p2 = icl(p1, Token::Comma);
        assert_eq!(
            p2.parse(&mut Lex::new(String::from("1 + 2, 5, 30 + 5 - 6 + 1")))
                .expect("Parse failed"),
            vec![3, 5, 30],
        );

        let p3 = seql(seql(mat(Token::Plus), mat(Token::Dash)), mat(Token::Plus));

        p3.parse(&mut Lex::new(String::from("+-+")))
            .expect("Parse failed");
    }
}
