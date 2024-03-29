use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use nom::branch::alt;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::combinator::{all_consuming, opt};
use nom::error::Error as NError;
use nom::multi::separated_list0;
use nom::sequence::delimited;
use nom::sequence::{preceded, tuple};
use nom::{Finish, IResult};
use smallstr::SmallString;

use crate::tree::Tree;

#[derive(Debug, PartialEq)]
pub struct MarkovizedNode<A> {
    pub label: A,
    pub children: Vec<A>,
    pub ancestors: Vec<A>,
}

#[derive(Debug, PartialEq)]
pub enum Binarized<A> {
    Markovized(MarkovizedNode<A>),
    Bare(A),
}

impl<A> Binarized<A> {
    /// A node is markovized, when it has children defined
    /// in its annotation. A node might feature ancestors,
    /// but without children it's not markovized.
    pub fn is_markovized(&self) -> bool {
        match self {
            Binarized::Markovized(node) => !node.children.is_empty(),
            Binarized::Bare(_) => false,
        }
    }

    pub fn extract_label(&self) -> &A {
        match self {
            Binarized::Markovized(MarkovizedNode { label, .. }) => label,
            Binarized::Bare(a) => a,
        }
    }
}

impl FromStr for Binarized<SmallString<[u8; 8]>> {
    type Err = NError<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match all_consuming(parse_binarized_node)(s.trim()).finish() {
            Ok((_, sentence)) => Ok(sentence),
            Err(NError { input, code }) => Err(NError {
                input: input.to_string(),
                code,
            }),
        }
    }
}

fn parse_binarized_node(input: &str) -> IResult<&str, Binarized<SmallString<[u8; 8]>>> {
    alt((parse_bare_node, parse_markovized_node))(input)
}

fn parse_bare_node(input: &str) -> IResult<&str, Binarized<SmallString<[u8; 8]>>> {
    all_consuming(is_not("|^"))(input).map(|(i, o)| (i, Binarized::Bare(SmallString::from(o))))
}

fn parse_markovized_node(input: &str) -> IResult<&str, Binarized<SmallString<[u8; 8]>>> {
    tuple((
        is_not("|^"),
        opt(preceded(
            tag("|"),
            delimited(
                tag("<"),
                separated_list0(tag(","), alt((tag(","), is_not("|^<>,")))),
                tag(">"),
            ),
        )),
        opt(preceded(
            tag("^"),
            delimited(
                tag("<"),
                separated_list0(tag(","), alt((tag(","), is_not("|^<>,")))),
                tag(">"),
            ),
        )),
    ))(input)
    .map(|(i, (label, children, ancestors))| {
        (
            i,
            Binarized::Markovized(MarkovizedNode {
                label: SmallString::from(label),
                children: children
                    .map(|mut v| v.drain(..).map(SmallString::from).collect())
                    .unwrap_or_default(),
                ancestors: ancestors
                    .map(|mut v| v.drain(..).map(SmallString::from).collect())
                    .unwrap_or_default(),
            }),
        )
    })
}

impl<A: fmt::Display> fmt::Display for MarkovizedNode<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = write!(f, "{}", self.label);

        if !self.children.is_empty() {
            result = result.and(write!(f, "|<"));
            for (i, parent) in self.children.iter().enumerate() {
                if i == 0 {
                    result = result.and(write!(f, "{}", parent));
                } else {
                    result = result.and(write!(f, ",{}", parent));
                }
            }

            result = result.and(write!(f, ">"));
        }

        if !self.ancestors.is_empty() {
            result = result.and(write!(f, "^<"));
            for (i, ancestor) in self.ancestors.iter().enumerate() {
                if i == 0 {
                    result = result.and(write!(f, "{}", ancestor));
                } else {
                    result = result.and(write!(f, ",{}", ancestor));
                }
            }

            result = result.and(write!(f, ">"));
        }

        result
    }
}

impl<A: fmt::Display> fmt::Display for Binarized<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Binarized::Markovized(a) => a.fmt(f),
            Binarized::Bare(a) => a.fmt(f),
        }
    }
}

impl<A: Borrow<str>> Tree<A> {
    pub fn parse_markovized(mut self) -> Tree<Binarized<SmallString<[u8; 8]>>> {
        if self.is_leaf() {
            Tree {
                root: Binarized::from_str(self.root.borrow()).unwrap(),
                children: vec![],
            }
        } else {
            Tree {
                root: Binarized::from_str(self.root.borrow()).unwrap(),
                children: self
                    .children
                    .drain(..)
                    .map(|c| c.parse_markovized())
                    .collect(),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn markovized_node_print() {
        let bare = MarkovizedNode {
            label: "label",
            children: vec![],
            ancestors: vec![],
        };

        assert_eq!("label".to_string(), format!("{}", bare));

        let single = MarkovizedNode {
            label: "label",
            children: vec!["parent"],
            ancestors: vec!["ancestor"],
        };

        assert_eq!(
            "label|<parent>^<ancestor>".to_string(),
            format!("{}", single)
        );

        let multiple = MarkovizedNode {
            label: "label",
            children: vec!["p1", "p2"],
            ancestors: vec!["a1", "a2"],
        };

        assert_eq!("label|<p1,p2>^<a1,a2>".to_string(), format!("{}", multiple));
    }

    #[test]
    fn binarized_parse() {
        assert_eq!(
            Binarized::Bare(SmallString::from("label")),
            Binarized::from_str("label").unwrap()
        );

        assert_eq!(
            Binarized::Markovized(MarkovizedNode {
                label: SmallString::from("label"),
                children: vec![SmallString::from("parent")],
                ancestors: vec![SmallString::from("ancestor")]
            }),
            Binarized::from_str("label|<parent>^<ancestor>").unwrap()
        );

        assert_eq!(
            Binarized::Markovized(MarkovizedNode {
                label: SmallString::from("label"),
                children: vec![SmallString::from("parent")],
                ancestors: vec![]
            }),
            Binarized::from_str("label|<parent>").unwrap()
        );

        assert_eq!(
            Binarized::Markovized(MarkovizedNode {
                label: SmallString::from("label"),
                children: vec![],
                ancestors: vec![SmallString::from("ancestor")]
            }),
            Binarized::from_str("label^<ancestor>").unwrap()
        );

        assert_eq!(
            Binarized::Markovized(MarkovizedNode {
                label: SmallString::from("label"),
                children: vec![SmallString::from("p1"), SmallString::from("p2")],
                ancestors: vec![SmallString::from("a1"), SmallString::from("a2")]
            }),
            Binarized::from_str("label|<p1,p2>^<a1,a2>").unwrap()
        );

        assert_eq!(
            Binarized::Markovized(MarkovizedNode {
                label: SmallString::from("label"),
                children: vec![SmallString::from(","), SmallString::from("p")],
                ancestors: vec![SmallString::from("p"), SmallString::from(",")]
            }),
            Binarized::from_str("label|<,,p>^<p,,>").unwrap()
        );
    }
}
