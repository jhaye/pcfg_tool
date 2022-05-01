use std::hash::Hash;
use std::str::FromStr;

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{multispace0, multispace1};
use nom::combinator::all_consuming;
use nom::error::Error as NError;
use nom::multi::many_till;
use nom::number::complete::double;
use nom::sequence::{separated_pair, terminated, tuple};
use nom::{Finish, IResult};
use smallstr::SmallString;

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub enum Rule<N, T>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    Lexical { lhs: N, rhs: T },
    NonLexical { lhs: N, rhs: Vec<N> },
}

#[derive(PartialEq, Eq, Debug)]
pub struct WeightedRule<N: Eq + Hash, T: Eq + Hash, W> {
    pub rule: Rule<N, T>,
    pub weight: W,
}

type ParsedWeightedRule = WeightedRule<SmallString<[u8; 8]>, SmallString<[u8; 8]>, f64>;
type NonLexicalRhs = (Vec<SmallString<[u8; 8]>>, f64);

impl FromStr for ParsedWeightedRule {
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

fn parse_rule(input: &str) -> IResult<&str, ParsedWeightedRule> {
    alt((parse_lexical_rule, parse_nonlexical_rule))(input.trim())
}

fn parse_lexical_rule(input: &str) -> IResult<&str, ParsedWeightedRule> {
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

fn parse_nonlexical_rule(input: &str) -> IResult<&str, ParsedWeightedRule> {
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

fn parse_rhs_nonlexical_rule(input: &str) -> IResult<&str, NonLexicalRhs> {
    many_till(terminated(is_not(" \t"), multispace1), double)(input.trim())
        .map(|(i, (mut rhs, w))| (i, (rhs.drain(..).map(SmallString::from).collect(), w)))
}

#[cfg(test)]
mod test {
    use super::*;

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
