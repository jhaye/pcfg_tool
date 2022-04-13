use crate::tree::Tree;

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Eq, PartialEq, Hash)]
pub enum Rule<N, T>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    Lexical { lhs: N, rhs: T },
    NonLexical { lhs: N, rhs: Vec<N> },
}

pub struct RuleSetAbsoluteWeight<N, T>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    pub rules: HashMap<Rule<N, T>, u32>,
}

impl<N, T> RuleSetAbsoluteWeight<N, T>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub fn insert(&mut self, rule: Rule<N, T>) {
        self.insert_with_weight(rule, 1);
    }

    fn insert_with_weight(&mut self, rule: Rule<N, T>, weight: u32) {
        if let Some(&x) = self.rules.get(&rule) {
            self.rules.insert(rule, x + weight);
        } else {
            self.rules.insert(rule, weight);
        }
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn absorb(&mut self, mut other: Self) {
        other
            .rules
            .drain()
            .for_each(|(r, w)| self.insert_with_weight(r, w));
    }

    pub fn merge(mut self, other: Self) -> Self {
        self.absorb(other);
        self
    }
}

impl<N: Eq + Hash, T: Eq + Hash> Default for RuleSetAbsoluteWeight<N, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Eq + Hash + Clone> From<Tree<A>> for RuleSetAbsoluteWeight<A, A> {
    fn from(tree: Tree<A>) -> Self {
        let mut rule_set = RuleSetAbsoluteWeight::new();

        match tree.children.len() {
            1 => {
                let child = tree.children.get(0).unwrap();
                if child.is_leaf() {
                    rule_set.insert(Rule::Lexical {
                        lhs: tree.root,
                        rhs: child.root.clone(),
                    });
                } else {
                    rule_set.insert(Rule::NonLexical {
                        lhs: tree.root,
                        rhs: vec![child.root.clone()],
                    });
                }
            }
            x if x > 1 => {
                rule_set.insert(Rule::NonLexical {
                    lhs: tree.root,
                    rhs: tree.children.iter().map(|c| c.root.clone()).collect(),
                });

                for child in tree.children {
                    rule_set.absorb(RuleSetAbsoluteWeight::from(child))
                }
            }
            _ => {}
        }

        rule_set
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_rule_induction_from_tree() {
        let rule_set = RuleSetAbsoluteWeight::from(Tree {
            root: "NP".to_string(),
            children: vec![
                Tree {
                    root: "D".to_string(),
                    children: vec![Tree {
                        root: "the".to_string(),
                        children: vec![],
                    }],
                },
                Tree {
                    root: "N".to_string(),
                    children: vec![Tree {
                        root: "ball".to_string(),
                        children: vec![],
                    }],
                },
            ],
        });

        assert!(rule_set.len() == 3);

        assert_eq!(
            rule_set.rules.get(&Rule::NonLexical {
                lhs: "NP".to_string(),
                rhs: vec!["D".to_string(), "N".to_string()]
            }),
            Some(&1)
        );

        assert_eq!(
            rule_set.rules.get(&Rule::Lexical {
                lhs: "D".to_string(),
                rhs: "the".to_string()
            }),
            Some(&1)
        );

        assert_eq!(
            rule_set.rules.get(&Rule::Lexical {
                lhs: "N".to_string(),
                rhs: "ball".to_string()
            }),
            Some(&1)
        );
    }

    #[test]
    fn rule_induction_with_duplicate() {
        let rule_set = RuleSetAbsoluteWeight::from(Tree {
            root: "NP".to_string(),
            children: vec![
                Tree {
                    root: "D".to_string(),
                    children: vec![Tree {
                        root: "the".to_string(),
                        children: vec![],
                    }],
                },
                Tree {
                    root: "D".to_string(),
                    children: vec![Tree {
                        root: "the".to_string(),
                        children: vec![],
                    }],
                },
            ],
        });

        assert!(rule_set.len() == 2);

        assert_eq!(
            rule_set.rules.get(&Rule::Lexical {
                lhs: "D".to_string(),
                rhs: "the".to_string()
            }),
            Some(&2)
        );
    }
}
