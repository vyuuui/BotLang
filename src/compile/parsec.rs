use crate::compile::lex::{AnnotTok, Lex, Token};
use crate::compile::CompileErr;
use std::marker::PhantomData;

pub trait Parser<Out> {
    fn parse(&self, l: &mut Lex) -> Result<Out, CompileErr>;
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
            Err(CompileErr::Mismatch(
                self.t.clone(),
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

pub fn mat(tok: Token) -> Match<()> {
    Match {
        t: tok,
        marker: PhantomData,
    }
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

mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let parser = pfx(
            &[Token::Plus, Token::Dash],
            |_, x| x,
            mat(Token::IntegralLiteral(3)),
        );

        let mut lexer = Lex::new(String::from("++-+3"));
        parser.parse(&mut lexer).expect("Failed parse");
    }
}
