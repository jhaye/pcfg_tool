use std::fmt;

use crate::SExp;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Tree<A> {
    pub root: A,
    pub children: Vec<Tree<A>>,
}

impl<A> Tree<A> {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn leaves(&self) -> Vec<&A> {
        if self.is_leaf() {
            vec![&self.root]
        } else {
            self.children
                .iter()
                .map(|c| c.leaves())
                .fold(vec![], |mut acc, mut x| {
                    acc.append(&mut x);
                    acc
                })
        }
    }

    pub fn leaves_mut(&mut self) -> Vec<&mut A> {
        if self.is_leaf() {
            vec![&mut self.root]
        } else {
            self.children
                .iter_mut()
                .map(|c| c.leaves_mut())
                .fold(vec![], |mut acc, mut x| {
                    acc.append(&mut x);
                    acc
                })
        }
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
            let mut result = write!(f, "({}", self.root);
            for child in &self.children {
                result = result.and(write!(f, " "));
                result = result.and(child.fmt(f));
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
    use std::str::FromStr;

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

    #[test]
    fn get_leaves() {
        let tree = Tree::from(SExp::from_str("(S (NP a b) c)").unwrap());
        let tree_leaves = tree.leaves();
        let mut leaves_iter = tree_leaves.iter();
        let leaf_a = leaves_iter.next().unwrap();
        assert_eq!("a".to_string(), (**leaf_a).clone().into_string());
        let leaf_b = leaves_iter.next().unwrap();
        assert_eq!("b".to_string(), (**leaf_b).clone().into_string());
        let leaf_c = leaves_iter.next().unwrap();
        assert_eq!("c".to_string(), (**leaf_c).clone().into_string());
    }
}
