use crate::compile::lex::{Lex, Token, TokenDiscriminants};
use crate::compile::{annot, CompileErr, LocationAnnot};
use std::marker::PhantomData;

pub trait Parser<Out, PT> {
    fn optional(&self) -> bool;
    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr>;
    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr>;
}

pub struct Epsilon<PT> {
    marker: PhantomData<PT>,
}

impl<PT> Parser<(), PT> for Epsilon<PT> {
    fn optional(&self) -> bool {
        true
    }

    fn ll1(&self, _: &PT, _: &mut Lex) -> Result<bool, CompileErr> {
        Ok(true)
    }

    fn parse(&self, _: &PT, _: &mut Lex) -> Result<(), CompileErr> {
        Ok(())
    }
}

pub struct WithAnnot<Out, PT, P: Parser<Out, PT>> {
    sub: P,
    marker: PhantomData<(Out, PT)>,
}

impl<Out, PT, P: Parser<Out, PT>> Parser<LocationAnnot<Out>, PT> for WithAnnot<Out, PT, P> {
    fn optional(&self) -> bool {
        self.sub.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<LocationAnnot<Out>, CompileErr> {
        let ann_begin = l.head_loc();
        let res = self.sub.parse(pt, l)?;
        Ok(annot(ann_begin.to_right(&l.prev_loc()), res))
    }
}

pub struct Log<Out, PT, P: Parser<Out, PT>> {
    sub: P,
    msg_ll1: String,
    msg_parse: String,
    marker: PhantomData<(Out, PT)>,
}

impl<Out, PT, P: Parser<Out, PT>> Parser<Out, PT> for Log<Out, PT, P> {
    fn optional(&self) -> bool {
        self.sub.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        println!("LL1 LOG: {}", self.msg_ll1);
        self.sub.ll1(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr> {
        println!("PARSE LOG: {}", self.msg_parse);
        self.sub.parse(pt, l)
    }
}

pub struct Invoke<Out, PT> {
    l: fn(&PT, &mut Lex) -> Result<bool, CompileErr>,
    f: fn(&PT, &mut Lex) -> Result<Out, CompileErr>,
}

impl<Out, PT> Parser<Out, PT> for Invoke<Out, PT> {
    fn optional(&self) -> bool {
        false
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        (self.l)(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr> {
        (self.f)(pt, l)
    }
}

pub struct MaybeInvoke<Out, PT> {
    l: fn(&PT, &mut Lex) -> Result<bool, CompileErr>,
    f: fn(&PT, &mut Lex) -> Result<Out, CompileErr>,
}

impl<Out, PT> Parser<Option<Out>, PT> for MaybeInvoke<Out, PT> {
    fn optional(&self) -> bool {
        true
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        (self.l)(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Option<Out>, CompileErr> {
        if self.ll1(pt, l)? {
            (self.f)(pt, l).map(Some)
        } else {
            Ok(None)
        }
    }
}

pub struct Convert<In, Out, PT, P: Parser<In, PT>> {
    sub: P,
    f: fn(In) -> Out,
    marker: PhantomData<PT>,
}

impl<In, Out, PT, P: Parser<In, PT>> Parser<Out, PT> for Convert<In, Out, PT, P> {
    fn optional(&self) -> bool {
        self.sub.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr> {
        self.sub.parse(pt, l).map(self.f)
    }
}

pub struct ConvertWithErr<In, Out, PT, P: Parser<In, PT>> {
    sub: P,
    f: fn(In) -> Result<Out, CompileErr>,
    marker: PhantomData<PT>,
}

impl<In, Out, PT, P: Parser<In, PT>> Parser<Out, PT> for ConvertWithErr<In, Out, PT, P> {
    fn optional(&self) -> bool {
        self.sub.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr> {
        self.sub.parse(pt, l).and_then(self.f)
    }
}

pub struct MatchType<PT> {
    t: TokenDiscriminants,
    marker: PhantomData<PT>,
}

impl<PT> Parser<TokenDiscriminants, PT> for MatchType<PT> {
    fn optional(&self) -> bool {
        false
    }

    fn ll1(&self, _: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        Ok(self.t == (&l.peek()?.inner).into())
    }

    fn parse(&self, _: &PT, l: &mut Lex) -> Result<TokenDiscriminants, CompileErr> {
        let tok = l.peek()?;
        if self.t == (&tok.inner).into() {
            (Ok(self.t), l.eat()).0
        } else {
            Err(CompileErr::TypeMismatch(self.t, tok.inner.clone()))
        }
    }
}

pub struct MatchVal<PT> {
    t: Token,
    marker: PhantomData<PT>,
}

impl<PT> Parser<(), PT> for MatchVal<PT> {
    fn optional(&self) -> bool {
        false
    }

    fn ll1(&self, _: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        Ok(l.peek()?.inner == self.t)
    }

    fn parse(&self, _: &PT, l: &mut Lex) -> Result<(), CompileErr> {
        let tok = l.peek()?;
        if tok.inner == self.t {
            (Ok(()), l.eat()).0
        } else {
            Err(CompileErr::ValMismatch(self.t.clone(), tok.inner.clone()))
        }
    }
}

macro_rules! mkextractor {
    ($nm:ident $fnnm:ident $tty:ident $ty:ty) => {
        pub struct $nm<PT> {
            marker: PhantomData<PT>,
        }

        impl<PT> Parser<$ty, PT> for $nm<PT> {
            fn optional(&self) -> bool {
                false
            }

            fn ll1(&self, _: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
                Ok(TokenDiscriminants::$tty == (&l.peek()?.inner).into())
            }

            fn parse(&self, _: &PT, l: &mut Lex) -> Result<$ty, CompileErr> {
                let tok = l.peek()?;
                if let Token::$tty(x) = &tok.inner {
                    Ok((x.clone(), l.eat()).0)
                } else {
                    Err(CompileErr::TypeMismatch(
                        TokenDiscriminants::$tty,
                        tok.inner.clone(),
                    ))
                }
            }
        }

        pub fn $fnnm<PT>() -> impl Parser<$ty, PT> {
            $nm {
                marker: PhantomData,
            }
        }
    };
}

mkextractor!(ExtractIdent extid Identifier String);
mkextractor!(ExtractString extstr StringLiteral Vec<u8>);
mkextractor!(ExtractInt extint IntegralLiteral u64);
mkextractor!(ExtractFloat extflt FloatLiteral f32);
mkextractor!(ExtractDouble extdbl DoubleLiteral f64);

pub struct Repeat<Out, PT, P: Parser<Out, PT>> {
    sub: P,
    marker: PhantomData<(Out, PT)>,
}

impl<Out, PT, P: Parser<Out, PT>> Parser<Vec<Out>, PT> for Repeat<Out, PT, P> {
    fn optional(&self) -> bool {
        true
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Vec<Out>, CompileErr> {
        let mut res = Vec::new();
        while self.ll1(pt, l)? {
            res.push(self.sub.parse(pt, l)?);
        }

        Ok(res)
    }
}

pub struct Intercalate<Out, PT, P: Parser<Out, PT>> {
    sub: P,
    int: Token,
    marker: PhantomData<(Out, PT)>,
}

// TODO: This is fake intercalation for the time being
impl<Out, PT, P: Parser<Out, PT>> Parser<Vec<Out>, PT> for Intercalate<Out, PT, P> {
    fn optional(&self) -> bool {
        true
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Vec<Out>, CompileErr> {
        let mut res = Vec::new();

        while self.ll1(pt, l)? {
            res.push(self.sub.parse(pt, l)?);
            let int = l.peek()?;
            if int.inner != self.int {
                break;
            }
            l.eat();
        }

        Ok(res)
    }
}

pub struct InfixFold<Out, PT, P: Parser<Out, PT>> {
    sub: P,
    int: Vec<Token>,
    combine: fn(Out, Token, Out) -> Out,
    marker: PhantomData<PT>,
}

impl<Out, PT, P: Parser<Out, PT>> Parser<Out, PT> for InfixFold<Out, PT, P> {
    fn optional(&self) -> bool {
        self.sub.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr> {
        let mut res = self.sub.parse(pt, l)?;
        while self.int.contains(&l.peek()?.inner) {
            let inter = l.peek()?.inner.clone();
            l.eat();
            let next = self.sub.parse(pt, l)?;
            res = (self.combine)(res, inter, next);
        }
        Ok(res)
    }
}

pub struct PrefixFold<Out, PT, P: Parser<Out, PT>> {
    sub: P,
    pre: Vec<Token>,
    combine: fn(Token, Out) -> Out,
    marker: PhantomData<PT>,
}

impl<Out, PT, P: Parser<Out, PT>> Parser<Out, PT> for PrefixFold<Out, PT, P> {
    fn optional(&self) -> bool {
        self.sub.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        self.sub.ll1(pt, l)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr> {
        let mut pref_list: Vec<Token> = Vec::new();
        while self.pre.contains(&l.peek()?.inner) {
            pref_list.push(l.peek()?.inner.clone());
            l.eat();
        }
        let mut res = self.sub.parse(pt, l)?;
        for tok in pref_list.into_iter().rev() {
            res = (self.combine)(tok, res);
        }
        Ok(res)
    }
}

pub struct SeqL<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>> {
    lhs: PL,
    rhs: PR,
    marker: PhantomData<(OutL, OutR, PT)>,
}

impl<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>> Parser<OutL, PT>
    for SeqL<OutL, OutR, PT, PL, PR>
{
    fn optional(&self) -> bool {
        self.lhs.optional() && self.rhs.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        if self.lhs.optional() {
            Ok(self.lhs.ll1(pt, l)? || self.rhs.ll1(pt, l)?)
        } else {
            self.lhs.ll1(pt, l)
        }
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<OutL, CompileErr> {
        Ok((self.lhs.parse(pt, l)?, self.rhs.parse(pt, l)?).0)
    }
}

pub struct SeqR<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>> {
    lhs: PL,
    rhs: PR,
    marker: PhantomData<(OutL, OutR, PT)>,
}

impl<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>> Parser<OutR, PT>
    for SeqR<OutL, OutR, PT, PL, PR>
{
    fn optional(&self) -> bool {
        self.lhs.optional() && self.rhs.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        if self.lhs.optional() {
            Ok(self.lhs.ll1(pt, l)? || self.rhs.ll1(pt, l)?)
        } else {
            self.lhs.ll1(pt, l)
        }
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<OutR, CompileErr> {
        Ok((self.lhs.parse(pt, l)?, self.rhs.parse(pt, l)?).1)
    }
}

pub struct SeqLR<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>> {
    lhs: PL,
    rhs: PR,
    marker: PhantomData<(OutL, OutR, PT)>,
}

impl<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>> Parser<(OutL, OutR), PT>
    for SeqLR<OutL, OutR, PT, PL, PR>
{
    fn optional(&self) -> bool {
        self.lhs.optional() && self.rhs.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        if self.lhs.optional() {
            Ok(self.lhs.ll1(pt, l)? || self.rhs.ll1(pt, l)?)
        } else {
            self.lhs.ll1(pt, l)
        }
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<(OutL, OutR), CompileErr> {
        Ok((self.lhs.parse(pt, l)?, self.rhs.parse(pt, l)?))
    }
}

pub struct MaybeSeqLR<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>> {
    lhs: PL,
    rhs: PR,
    marker: PhantomData<(OutL, OutR, PT)>,
}

impl<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>> Parser<Option<(OutL, OutR)>, PT>
    for MaybeSeqLR<OutL, OutR, PT, PL, PR>
{
    fn optional(&self) -> bool {
        self.lhs.optional() && self.rhs.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        if self.lhs.optional() {
            Ok(self.lhs.ll1(pt, l)? || self.rhs.ll1(pt, l)?)
        } else {
            self.lhs.ll1(pt, l)
        }
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Option<(OutL, OutR)>, CompileErr> {
        if self.lhs.ll1(pt, l)? {
            Ok(Some((self.lhs.parse(pt, l)?, self.rhs.parse(pt, l)?)))
        } else {
            Ok(None)
        }
    }
}

pub struct Flagged<Out, PT, P: Parser<Out, PT>> {
    t: Token,
    sub: P,
    marker: PhantomData<(Out, PT)>,
}

impl<Out, PT, P: Parser<Out, PT>> Parser<Option<Out>, PT> for Flagged<Out, PT, P> {
    fn optional(&self) -> bool {
        true
    }

    fn ll1(&self, _: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        Ok(self.t == l.peek()?.inner)
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Option<Out>, CompileErr> {
        if self.ll1(pt, l)? {
            Ok((l.eat(), Some(self.sub.parse(pt, l)?)).1)
        } else {
            Ok(None)
        }
    }
}

macro_rules! mkwrapped {
    ($wrl:path, $wrr:path, $nm:ident, $fnnm:ident) => {
        pub struct $nm<Out, PT, P: Parser<Out, PT>> {
            sub: P,
            marker: PhantomData<(Out, PT)>,
        }

        impl<Out, PT, P: Parser<Out, PT>> Parser<Out, PT> for $nm<Out, PT, P> {
            fn optional(&self) -> bool {
                false
            }

            fn ll1(&self, _: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
                l.peek().map(|v| v.inner == $wrl)
            }

            fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr> {
                let before = l.peek()?;
                if before.inner == $wrl {
                    l.eat();
                    let res = self.sub.parse(pt, l)?;
                    let after = l.peek()?;
                    if after.inner == $wrr {
                        l.eat();
                        Ok(res)
                    } else {
                        Err(CompileErr::ValMismatch($wrr, after.inner.clone()))
                    }
                } else {
                    Err(CompileErr::ValMismatch($wrl, before.inner.clone()))
                }
            }
        }

        pub fn $fnnm<Out, PT, P: Parser<Out, PT>>(sub: P) -> $nm<Out, PT, P> {
            $nm {
                sub,
                marker: PhantomData,
            }
        }
    };
}

mkwrapped!(Token::LParen, Token::RParen, WrapParen, wrparen);
mkwrapped!(Token::LCurly, Token::RCurly, WrapCurly, wrcurly);
mkwrapped!(Token::LBracket, Token::RBracket, WrapBracket, wrbracket);

pub trait ToOneOf<T> {
    fn oneof(self) -> T;
}

macro_rules! mkoneof {
    // Thankfully OneOf is pointless with a single subparser, so this doesn't need repeating for
    // the base case
    ($n0:ident $t0:ident $v0:ident) => {};

    ($n0:ident $t0:ident $v0:ident, $($nk:ident $tk:ident $vk:ident),*) => {
        mkoneof!($($nk $tk $vk),*);

        pub struct $n0<Out, PT, $t0, $($tk),*> {
            tup: ($t0, $($tk),*),
            marker: PhantomData<(Out, PT)>,
        }

        impl<Out, PT, $t0: Parser<Out, PT>, $($tk: Parser<Out, PT>),*> Parser<Out, PT> for $n0<Out, PT, $t0, $($tk,)*> {
            fn optional(&self) -> bool {
                let ($v0, $($vk),*) = &self.tup;
                $v0.optional() $(&& $vk.optional())*
            }

            fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
                let ($v0, $($vk),*) = &self.tup;
                Ok($v0.ll1(pt, l)? $(|| $vk.ll1(pt, l)?)*)
            }

            fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr> {
                let ($v0, $($vk),*) = &self.tup;
                if $v0.ll1(pt, l)? {
                    $v0.parse(pt, l)
                } $(else if $vk.ll1(pt, l)? {
                    $vk.parse(pt, l)
                })* else {
                    Err(CompileErr::OneOfFailure(l.peek()?.inner.clone()))
                }
            }
        }

        impl<Out, PT, $t0: Parser<Out, PT>, $($tk: Parser<Out, PT>),*> ToOneOf<$n0<Out, PT, $t0, $($tk,)*>> for ($t0, $($tk,)*) {
            fn oneof(self) -> $n0<Out, PT, $t0, $($tk,)*> {
                $n0 {
                    tup: self,
                    marker: PhantomData,
                }
            }
        }

    };
}

mkoneof!(OoA A a, OoB B b, OoC C c, OoD D d, OoE E e, OoF F f, OoG G g, OoH H h, OoI I i, OoJ J j, OoK K k, OoL L l, OoM M m, OoN N n, OoO O o, OoP P p, OoQ Q q, OoR R r, OoS S s, OoT T t);

pub struct Alternative<Out, PT, PL: Parser<Out, PT>, PR: Parser<Out, PT>> {
    lhs: PL,
    rhs: PR,
    marker: PhantomData<(Out, PT)>,
}

impl<Out, PT, PL: Parser<Out, PT>, PR: Parser<Out, PT>> Parser<Out, PT>
    for Alternative<Out, PT, PL, PR>
{
    fn optional(&self) -> bool {
        self.lhs.optional() && self.rhs.optional()
    }

    fn ll1(&self, pt: &PT, l: &mut Lex) -> Result<bool, CompileErr> {
        if self.lhs.optional() {
            Ok(self.lhs.ll1(pt, l)? || self.rhs.ll1(pt, l)?)
        } else {
            self.lhs.ll1(pt, l)
        }
    }

    fn parse(&self, pt: &PT, l: &mut Lex) -> Result<Out, CompileErr> {
        l.mark();
        if self.lhs.ll1(pt, l)? {
            let res_l = self.lhs.parse(pt, l);
            if res_l.is_err() {
                l.rewind();
                self.rhs.parse(pt, l)
            } else {
                res_l
            }
        } else {
            self.rhs.parse(pt, l)
        }
    }
}

pub fn epsilon<PT>() -> impl Parser<(), PT> {
    Epsilon {
        marker: PhantomData,
    }
}

pub fn withannot<Out, PT, P: Parser<Out, PT>>(sub: P) -> impl Parser<LocationAnnot<Out>, PT> {
    WithAnnot {
        sub,
        marker: PhantomData,
    }
}

pub fn log<Out, PT, P: Parser<Out, PT>>(
    msg_ll1: &str,
    msg_parse: &str,
    sub: P,
) -> impl Parser<Out, PT> {
    Log {
        msg_ll1: String::from(msg_ll1),
        msg_parse: String::from(msg_parse),
        sub,
        marker: PhantomData,
    }
}

pub fn inv<Out, PT>(
    l: fn(&PT, &mut Lex) -> Result<bool, CompileErr>,
    f: fn(&PT, &mut Lex) -> Result<Out, CompileErr>,
) -> impl Parser<Out, PT> {
    Invoke { l, f }
}

pub fn maybeinv<Out, PT>(
    l: fn(&PT, &mut Lex) -> Result<bool, CompileErr>,
    f: fn(&PT, &mut Lex) -> Result<Out, CompileErr>,
) -> impl Parser<Option<Out>, PT> {
    MaybeInvoke { l, f }
}

pub fn cvt<In, Out, PT, P: Parser<In, PT>>(f: fn(In) -> Out, sub: P) -> impl Parser<Out, PT> {
    Convert {
        f,
        sub,
        marker: PhantomData,
    }
}

pub fn cvt_witherr<In, Out, PT, P: Parser<In, PT>>(
    f: fn(In) -> Result<Out, CompileErr>,
    sub: P,
) -> impl Parser<Out, PT> {
    ConvertWithErr {
        f,
        sub,
        marker: PhantomData,
    }
}

pub fn mat_tp<PT>(tok: TokenDiscriminants) -> impl Parser<TokenDiscriminants, PT> {
    MatchType {
        t: tok,
        marker: PhantomData,
    }
}

pub fn mat_val<PT>(tok: Token) -> impl Parser<(), PT> {
    MatchVal {
        t: tok,
        marker: PhantomData,
    }
}

pub fn rep<Out, PT, P: Parser<Out, PT>>(sub: P) -> impl Parser<Vec<Out>, PT> {
    Repeat {
        sub,
        marker: PhantomData,
    }
}

pub fn icl<Out, PT, P: Parser<Out, PT>>(sub: P, int: Token) -> impl Parser<Vec<Out>, PT> {
    Intercalate {
        sub,
        int,
        marker: PhantomData,
    }
}

pub fn pfx<Out, PT, P: Parser<Out, PT>>(
    pre: &[Token],
    combine: fn(Token, Out) -> Out,
    sub: P,
) -> impl Parser<Out, PT> {
    PrefixFold {
        sub,
        pre: Vec::from(pre),
        combine,
        marker: PhantomData,
    }
}

pub fn ifx<Out, PT, P: Parser<Out, PT>>(
    sub: P,
    int: &[Token],
    combine: fn(Out, Token, Out) -> Out,
) -> impl Parser<Out, PT> {
    InfixFold {
        sub,
        int: Vec::from(int),
        combine,
        marker: PhantomData,
    }
}

pub fn seql<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>>(
    lhs: PL,
    rhs: PR,
) -> impl Parser<OutL, PT> {
    SeqL {
        lhs,
        rhs,
        marker: PhantomData,
    }
}

pub fn seqr<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>>(
    lhs: PL,
    rhs: PR,
) -> impl Parser<OutR, PT> {
    SeqR {
        lhs,
        rhs,
        marker: PhantomData,
    }
}

pub fn seqlr<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>>(
    lhs: PL,
    rhs: PR,
) -> impl Parser<(OutL, OutR), PT> {
    SeqLR {
        lhs,
        rhs,
        marker: PhantomData,
    }
}

pub fn maybeseqlr<OutL, OutR, PT, PL: Parser<OutL, PT>, PR: Parser<OutR, PT>>(
    lhs: PL,
    rhs: PR,
) -> impl Parser<Option<(OutL, OutR)>, PT> {
    MaybeSeqLR {
        lhs,
        rhs,
        marker: PhantomData,
    }
}

pub fn flagged<Out, PT, P: Parser<Out, PT>>(t: Token, sub: P) -> impl Parser<Option<Out>, PT> {
    Flagged {
        t,
        sub,
        marker: PhantomData,
    }
}

pub fn alt<Out, PT, PL: Parser<Out, PT>, PR: Parser<Out, PT>>(
    lhs: PL,
    rhs: PR,
) -> impl Parser<Out, PT> {
    Alternative {
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
            mat_val(Token::IntegralLiteral(3)),
        );
        p0.parse(&(), &mut Lex::new(String::from("++-+3")))
            .expect("Parse failed");
        let p1 = ifx(extint(), &[Token::Plus, Token::Dash], |x, op, y| {
            match (x, op, y) {
                (lhs, Token::Plus, rhs) => lhs + rhs,
                (lhs, Token::Dash, rhs) => lhs - rhs,
                _ => 0,
            }
        });
        assert_eq!(
            p1.parse(&(), &mut Lex::new(String::from("3 + 4")))
                .expect("Parse failed"),
            7
        );
        assert_eq!(
            p1.parse(&(), &mut Lex::new(String::from("3 + 4 + 5")))
                .expect("Parse failed"),
            12
        );
        assert_eq!(
            p1.parse(&(), &mut Lex::new(String::from("3 + 4 - 5")))
                .expect("Parse failed"),
            2
        );

        let p2 = icl(p1, Token::Comma);
        assert_eq!(
            p2.parse(&(), &mut Lex::new(String::from("1 + 2, 5, 30 + 5 - 6 + 1")))
                .expect("Parse failed"),
            vec![3, 5, 30],
        );

        let p3 = seql(
            seql(
                mat_tp(TokenDiscriminants::Plus),
                mat_tp(TokenDiscriminants::Dash),
            ),
            mat_tp(TokenDiscriminants::Plus),
        );

        p3.parse(&(), &mut Lex::new(String::from("+-+")))
            .expect("Parse failed");
    }
}
