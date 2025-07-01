use crate::compile::lex::{AnnotTok, Lex, Token, TokenDiscriminants};
use crate::compile::CompileErr;
use std::marker::PhantomData;

pub trait Parser<Out> {
    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr>;
}

pub struct Convert<In, Out, P: Parser<In>> {
    sub: P,
    f: fn(In) -> Out,
}

impl<In, Out, P: Parser<In>> Parser<Out> for Convert<In, Out, P> {
    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr> {
        Ok((self.f)(self.sub.parse(l)?))
    }
}

pub struct Match<Out> {
    t: Token,
    marker: PhantomData<Out>,
}

impl Parser<()> for Match<()> {
    fn parse(&self, l: &mut Lex) -> Result<(), CompileErr> {
        let tok = l.peek()?;
        if tok.inner == self.t {
            l.eat();
            Ok(())
        } else {
            Err(CompileErr::ValMismatch(
                self.t.clone(),
                tok.inner.clone(),
                tok.loc,
            ))
        }
    }
}

pub struct Extract {
    tp: TokenDiscriminants,
}

impl Parser<AnnotTok> for Extract {
    fn parse(&self, l: &mut Lex) -> Result<AnnotTok, CompileErr> {
        let tok = l.peek()?;
        if self.tp == (&tok.inner).into() {
            let tok_copy = tok.clone();
            l.eat();
            Ok(tok_copy)
        } else {
            Err(CompileErr::TypeMismatch(
                self.tp,
                tok.inner.clone(),
                tok.loc,
            ))
        }
    }
}

pub struct InfixFold<Out, P: Parser<Out>> {
    sub: P,
    int: Vec<Token>,
    combine: fn(Out, AnnotTok, Out) -> Out,
}

impl<Out, P: Parser<Out>> Parser<Out> for InfixFold<Out, P> {
    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr> {
        let mut res = self.sub.parse(l)?;
        while self.int.contains(&l.peek()?.inner) {
            let inter = l.peek()?.clone();
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
    combine: fn(AnnotTok, Out) -> Out,
}

impl<Out, P: Parser<Out>> Parser<Out> for PrefixFold<Out, P> {
    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr> {
        let mut pref_list: Vec<AnnotTok> = Vec::new();
        while self.pre.contains(&l.peek()?.inner) {
            pref_list.push(l.peek()?.clone());
            l.eat();
        }
        let mut res = self.sub.parse(l)?;
        for tok in pref_list.into_iter().rev() {
            res = (self.combine)(tok, res);
        }
        Ok(res)
    }
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

pub fn ext(tp: TokenDiscriminants) -> Extract {
    Extract { tp }
}

pub fn pfx<Out, P: Parser<Out>>(
    pre: &[Token],
    combine: fn(AnnotTok, Out) -> Out,
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
    combine: fn(Out, AnnotTok, Out) -> Out,
) -> InfixFold<Out, P> {
    InfixFold {
        sub,
        int: Vec::from(int),
        combine,
    }
}

mod tests {
    use super::*;

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
            cvt(
                |x| {
                    if let Token::IntegralLiteral(val) = x.inner {
                        val
                    } else {
                        0
                    }
                },
                ext(TokenDiscriminants::IntegralLiteral),
            ),
            &[Token::Plus, Token::Dash],
            |x, op, y| match (x, op.inner, y) {
                (lhs, Token::Plus, rhs) => lhs + rhs,
                (lhs, Token::Dash, rhs) => lhs - rhs,
                _ => 0,
            },
        );
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
    }
}
