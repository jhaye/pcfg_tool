use std::hash::Hash;
use std::str::FromStr;

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{char, multispace0, multispace1};
use nom::combinator::all_consuming;
use nom::error::Error as NError;
use nom::multi::{many_till, separated_list0, separated_list1};
use nom::number::complete::double;
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
use nom::{Finish, IResult};
use smallstr::SmallString;

use crate::grammar::Rule;

#[derive(PartialEq, Eq, Debug)]
pub enum SExp<A> {
    Atom(A),
    List(Vec<SExp<A>>),
}

impl FromStr for SExp<SmallString<[u8; 8]>> {
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

fn parse_sexp(input: &str) -> IResult<&str, SExp<SmallString<[u8; 8]>>> {
    alt((
        delimited(char('('), parse_sexp_list, preceded(multispace0, char(')'))),
        parse_atom,
    ))(input.trim())
}

fn parse_sexp_list(input: &str) -> IResult<&str, SExp<SmallString<[u8; 8]>>> {
    separated_list0(multispace1, parse_sexp)(input.trim()).map(|(i, o)| (i, SExp::List(o)))
}

fn parse_atom(input: &str) -> IResult<&str, SExp<SmallString<[u8; 8]>>> {
    is_not("() \t")(input.trim()).map(|(i, o)| (i, SExp::Atom(SmallString::from(o))))
}

#[derive(PartialEq, Eq, Debug)]
struct Sentence<A>(Vec<A>);

impl FromStr for Sentence<SmallString<[u8; 8]>> {
    type Err = NError<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match all_consuming(parse_sentence)(s).finish() {
            Ok((_, sentence)) => Ok(sentence),
            Err(NError { input, code }) => Err(NError {
                input: input.to_string(),
                code,
            }),
        }
    }
}

fn parse_sentence(input: &str) -> IResult<&str, Sentence<SmallString<[u8; 8]>>> {
    separated_list1(multispace1, is_not(" \t"))(input.trim()).map(|(i, mut o)| {
        (
            i,
            Sentence(o.drain(..).map(|w| SmallString::from(w)).collect()),
        )
    })
}

#[derive(PartialEq, Eq, Debug)]
struct WeightedRule<N: Eq + Hash, T: Eq + Hash, W> {
    rule: Rule<N, T>,
    weight: W,
}

impl FromStr for WeightedRule<SmallString<[u8; 8]>, SmallString<[u8; 8]>, f64> {
    type Err = NError<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match all_consuming(parse_rule)(s).finish() {
            Ok((_, rule)) => Ok(rule),
            Err(NError { input, code }) => Err(NError {
                input: input.to_string(),
                code,
            }),
        }
    }
}

fn parse_rule(
    input: &str,
) -> IResult<&str, WeightedRule<SmallString<[u8; 8]>, SmallString<[u8; 8]>, f64>> {
    alt((parse_lexical_rule, parse_nonlexical_rule))(input.trim())
}

fn parse_lexical_rule(
    input: &str,
) -> IResult<&str, WeightedRule<SmallString<[u8; 8]>, SmallString<[u8; 8]>, f64>> {
    tuple((
        terminated(is_not(" \t"), multispace1),
        terminated(is_not(" \t"), multispace1),
        terminated(double, multispace0),
    ))(input)
    .map(|(i, (n, t, weight))| {
        (
            i,
            WeightedRule {
                rule: Rule::Lexical {
                    lhs: SmallString::from(n),
                    rhs: SmallString::from(t),
                },
                weight,
            },
        )
    })
}

fn parse_nonlexical_rule(
    input: &str,
) -> IResult<&str, WeightedRule<SmallString<[u8; 8]>, SmallString<[u8; 8]>, f64>> {
    separated_pair(
        terminated(is_not(" \t"), multispace1),
        tag("->"),
        parse_rhs_nonlexical_rule,
    )(input)
    .map(|(i, (n, (rhs, weight)))| {
        (
            i,
            WeightedRule {
                rule: Rule::NonLexical {
                    lhs: SmallString::from(n),
                    rhs,
                },
                weight,
            },
        )
    })
}

fn parse_rhs_nonlexical_rule(input: &str) -> IResult<&str, (Vec<SmallString<[u8; 8]>>, f64)> {
    many_till(terminated(is_not(" \t"), multispace1), double)(input.trim()).map(
        |(i, (mut rhs, w))| {
            (
                i,
                (rhs.drain(..).map(|n| SmallString::from(n)).collect(), w),
            )
        },
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn atom_correct() {
        // Fails with brackets
        assert!(parse_atom("( a )").is_err());

        // Produces same results with whitespace
        let (_, atom) = parse_atom("a").unwrap();
        assert_eq!(atom, SExp::Atom(SmallString::from("a")));
        let (_, atom) = parse_atom(" a ").unwrap();
        assert_eq!(atom, SExp::Atom(SmallString::from("a")));

        // Works with special characters
        let (_, atom) = parse_atom("NP-SBJ|<,,ADJP,,>").unwrap();
        assert_eq!(atom, SExp::Atom(SmallString::from("NP-SBJ|<,,ADJP,,>")));
    }

    #[test]
    fn sexp_correct() {
        // Just an atom
        assert_eq!(
            SExp::from_str("a").unwrap(),
            SExp::Atom(SmallString::from("a"))
        );

        // With whitespace
        assert_eq!(
            SExp::from_str(" a ").unwrap(),
            SExp::Atom(SmallString::from("a"))
        );
        assert!(SExp::from_str(" ( a ) ").is_ok());

        // With brackets
        assert_eq!(
            SExp::from_str("(a)").unwrap(),
            SExp::List(vec![SExp::Atom(SmallString::from("a"))])
        );

        // List with multiple elements
        assert_eq!(
            SExp::from_str("(a b)").unwrap(),
            SExp::List(vec![
                SExp::Atom(SmallString::from("a")),
                SExp::Atom(SmallString::from("b"))
            ])
        );

        // Empty list
        assert_eq!(SExp::from_str("()").unwrap(), SExp::List(vec![]));

        // Nested
        assert_eq!(
            SExp::from_str("(a (b c))").unwrap(),
            SExp::List(vec![
                SExp::Atom(SmallString::from("a")),
                SExp::List(vec![
                    SExp::Atom(SmallString::from("b")),
                    SExp::Atom(SmallString::from("c")),
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

    #[test]
    fn sentence_correct() {
        let parsed =
            Sentence::from_str("Pierre Vinken , 61 years old , will join the board as a nonexecutive director Nov. 29 .").unwrap();
        let manual = Sentence(
            vec![
                "Pierre",
                "Vinken",
                ",",
                "61",
                "years",
                "old",
                ",",
                "will",
                "join",
                "the",
                "board",
                "as",
                "a",
                "nonexecutive",
                "director",
                "Nov.",
                "29",
                ".",
            ]
            .drain(..)
            .map(|w| SmallString::from(w))
            .collect(),
        );
        assert_eq!(manual, parsed);
    }

    #[test]
    fn rule_correct() {
        // basic lexical
        let parsed = WeightedRule::from_str("IN before 0.01694915254237288").unwrap();
        let rule = WeightedRule {
            rule: Rule::Lexical {
                lhs: SmallString::from("IN"),
                rhs: SmallString::from("before"),
            },
            weight: 0.01694915254237288,
        };
        assert_eq!(rule, parsed);

        // lexical fails with less or more
        assert!(WeightedRule::from_str("IN before extra 0.01694915254237288").is_err());
        assert!(WeightedRule::from_str("SHORT 0.01694915254237288").is_err());

        // basic non-lexical
        let parsed = WeightedRule::from_str("ADJP -> JJ JJ 0.14285714285714285").unwrap();
        let rule = WeightedRule {
            rule: Rule::NonLexical {
                lhs: SmallString::from("ADJP"),
                rhs: vec![SmallString::from("JJ"), SmallString::from("JJ")],
            },
            weight: 0.14285714285714285,
        };
        assert_eq!(rule, parsed);

        // fails with empty LHS or over-full RHS
        // note: empty RHS is permitted, as it would be interpreted as a
        //       lexical rule with "->" as a terminal
        assert!(WeightedRule::from_str("-> JJ JJ 0.14285714285714285").is_err());
        assert!(WeightedRule::from_str("ADJP EXTRA -> JJ JJ 0.14285714285714285").is_err());
    }
}
