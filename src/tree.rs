use std::fmt;

use crate::SExp;

#[derive(PartialEq, Eq, Debug)]
pub struct Tree<A> {
    pub root: A,
    pub children: Vec<Tree<A>>,
}

impl<A> Tree<A> {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

impl<A: Clone> From<SExp<A>> for Tree<A> {
    fn from(sexp: SExp<A>) -> Self {
        match sexp {
            SExp::List(list) => {
                let mut list = list;
                let root = match list.get(0).unwrap() {
                    SExp::Atom(a) => a.clone(),
                    _ => panic!("First element in SExp list has to be an atom!"),
                };

                let children = list.drain(..).skip(1).map(Self::from).collect();

                Tree { root, children }
            }
            SExp::Atom(root) => Tree {
                root,
                children: vec![],
            },
        }
    }
}

impl<A: fmt::Display> fmt::Display for Tree<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_leaf() {
            write!(f, "{}", self.root)
        } else {
            let mut result = write!(f, "( {} ", self.root);
            for child in &self.children {
                result = result.and(child.fmt(f));
                result = result.and(write!(f, " "));
            }
            result.and(write!(f, ")"))
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum NodeType<N, T> {
    Terminal(T),
    NonTerminal(N),
}

impl<N: fmt::Display, T: fmt::Display> fmt::Display for NodeType<N, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeType::Terminal(t) => t.fmt(f),
            NodeType::NonTerminal(n) => n.fmt(f),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sexp_tree_conversion() {
        assert_eq!(
            Tree {
                root: "a".to_string(),
                children: vec![]
            },
            Tree::from(SExp::Atom("a".to_string())),
        );

        assert_eq!(
            Tree {
                root: "NP".to_string(),
                children: vec![
                    Tree {
                        root: "D".to_string(),
                        children: vec![Tree {
                            root: "the".to_string(),
                            children: vec![]
                        }]
                    },
                    Tree {
                        root: "N".to_string(),
                        children: vec![Tree {
                            root: "ball".to_string(),
                            children: vec![]
                        }]
                    },
                ]
            },
            Tree::from(SExp::List(vec![
                SExp::Atom("NP".to_string()),
                SExp::List(vec![
                    SExp::Atom("D".to_string()),
                    SExp::Atom("the".to_string())
                ]),
                SExp::List(vec![
                    SExp::Atom("N".to_string()),
                    SExp::Atom("ball".to_string())
                ])
            ])),
        );
    }
}
