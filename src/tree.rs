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

                let children = list.drain(..).skip(1).map(|s| Self::from(s)).collect();

                Tree { root, children }
            }
            SExp::Atom(a) => Tree {
                root: a.clone(),
                children: vec![],
            },
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
