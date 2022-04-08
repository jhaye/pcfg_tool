use crate::SExp;

#[derive(PartialEq, Eq, Debug)]
pub struct Tree<A> {
    pub root: A,
    pub children: Vec<Tree<A>>,
}

impl<A: Clone> From<&SExp<A>> for Tree<A> {
    fn from(sexp: &SExp<A>) -> Self {
        match sexp {
            SExp::List(list) => {
                let root = match list.get(0).unwrap() {
                    SExp::Atom(a) => a,
                    _ => panic!("First element in SExp list has to be an atom!"),
                };

                let mut children = Vec::new();
                for sexp in list.iter().skip(1) {
                    let subtree = Self::from(sexp);
                    children.push(subtree);
                }

                Tree {
                    root: root.clone(),
                    children,
                }
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
            Tree::from(&SExp::Atom("a".to_string())),
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
            Tree::from(&SExp::List(vec![
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
