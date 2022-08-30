use std::str::FromStr;

use nom::branch::alt;
use nom::bytes::complete::is_not;
use nom::character::complete::{char, multispace0, multispace1};
use nom::combinator::all_consuming;
use nom::error::Error as NError;
use nom::multi::separated_list0;
use nom::sequence::{delimited, preceded};
use nom::{Finish, IResult};

#[derive(PartialEq, Eq, Debug)]
pub enum SExp<A> {
    Atom(A),
    List(Vec<SExp<A>>),
}

impl<T> FromStr for SExp<T>
where
    T: for<'a> From<&'a str>,
{
    type Err = NError<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match all_consuming(parse_sexp)(s).finish() {
            Ok((_, sexp)) => Ok(sexp),
            Err(NError { input, code }) => Err(NError {
                input: input.to_string(),
                code,
            }),
        }
    }
}

fn parse_sexp<'a, T>(input: &'a str) -> IResult<&str, SExp<T>>
where
    T: From<&'a str>,
{
    alt((
        delimited(char('('), parse_sexp_list, preceded(multispace0, char(')'))),
        parse_atom,
    ))(input.trim())
}

fn parse_sexp_list<'a, T>(input: &'a str) -> IResult<&str, SExp<T>>
where
    T: From<&'a str>,
{
    separated_list0(multispace1, parse_sexp)(input.trim()).map(|(i, o)| (i, SExp::List(o)))
}

fn parse_atom<'a, T>(input: &'a str) -> IResult<&str, SExp<T>>
where
    T: From<&'a str>,
{
    is_not("() \t")(input.trim()).map(|(i, o)| (i, SExp::Atom(T::from(o))))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn atom_correct() {
        // Fails with brackets
        assert!(parse_atom::<String>("( a )").is_err());

        // Produces same results with whitespace
        let (_, atom) = parse_atom("a").unwrap();
        assert_eq!(atom, SExp::Atom("a".to_string()));
        let (_, atom) = parse_atom(" a ").unwrap();
        assert_eq!(atom, SExp::Atom("a".to_string()));

        // Works with special characters
        let (_, atom) = parse_atom("NP-SBJ|<,,ADJP,,>").unwrap();
        assert_eq!(atom, SExp::Atom(String::from("NP-SBJ|<,,ADJP,,>")));
    }

    #[test]
    fn sexp_correct() {
        // Just an atom
        assert_eq!(SExp::from_str("a").unwrap(), SExp::Atom("a".to_string()));

        // With whitespace
        assert_eq!(SExp::from_str(" a ").unwrap(), SExp::Atom("a".to_string()));
        assert!(SExp::from_str(" ( a ) ").is_ok());

        // With brackets
        assert_eq!(
            SExp::from_str("(a)").unwrap(),
            SExp::List(vec![SExp::Atom("a".to_string())])
        );

        // List with multiple elements
        assert_eq!(
            SExp::from_str("(a b)").unwrap(),
            SExp::List(vec![
                SExp::Atom("a".to_string()),
                SExp::Atom("b".to_string())
            ])
        );

        // Empty list
        assert_eq!(SExp::from_str("()").unwrap(), SExp::List(vec![]));

        // Nested
        assert_eq!(
            SExp::from_str("(a (b c))").unwrap(),
            SExp::List(vec![
                SExp::Atom("a".to_string()),
                SExp::List(vec![
                    SExp::Atom("b".to_string()),
                    SExp::Atom("c".to_string()),
                ])
            ])
        );

        // Bracket mismatch
        assert!(SExp::from_str("(a))").is_err());

        // Nothing
        assert!(SExp::from_str("").is_err());

        // Combined
        assert!(SExp::from_str("  (   A (   A  a  ) ( A b ))").is_ok());
        assert!(SExp::from_str("( A    (   B  (  A a ) ) )").is_ok());
    }
}
