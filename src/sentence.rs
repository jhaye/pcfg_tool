use std::str::FromStr;

use nom::bytes::complete::is_not;
use nom::character::complete::multispace1;
use nom::combinator::all_consuming;
use nom::error::Error as NError;
use nom::multi::separated_list1;
use nom::{Finish, IResult};
use smallstr::SmallString;

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
}
