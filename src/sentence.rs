use std::slice::{Iter, IterMut};
use std::str::FromStr;

use nom::bytes::complete::is_not;
use nom::character::complete::multispace1;
use nom::combinator::all_consuming;

use nom::error::Error as NError;
use nom::multi::separated_list1;
use nom::{Finish, IResult};
use smallstr::SmallString;

use crate::tree::{NodeType, Tree};

#[derive(PartialEq, Eq, Debug)]
pub struct Sentence<A>(pub Vec<A>);

impl<A> Sentence<A> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> Iter<'_, A> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, A> {
        self.0.iter_mut()
    }
}

impl<A: From<&'static str>> Sentence<A> {
    pub fn into_noparse(mut self) -> Tree<NodeType<A, A>> {
        Tree {
            root: NodeType::NonTerminal("NOPARSE".into()),
            children: self
                .0
                .drain(..)
                .map(|w| Tree {
                    root: NodeType::Terminal(w),
                    children: vec![],
                })
                .collect(),
        }
    }
}

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
    separated_list1(multispace1, is_not(" \t"))(input.trim())
        .map(|(i, mut o)| (i, Sentence(o.drain(..).map(SmallString::from).collect())))
}

#[cfg(test)]
mod test {
    use super::*;

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
    fn noparse_construction() {
        let sentence = Sentence::from_str("A little test sentence .").unwrap();
        assert_eq!(
            "(NOPARSE A little test sentence .)".to_string(),
            format!("{}", sentence.into_noparse())
        );
    }
}
