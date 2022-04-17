use crate::tree::Tree;

use fxhash::{FxHashMap, FxHashSet};
use multimap::MultiMap;

use std::fmt::Display;
use std::hash::Hash;
use std::io::{self, Write};

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub enum Rule<N, T>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    Lexical { lhs: N, rhs: T },
    NonLexical { lhs: N, rhs: Vec<N> },
}

#[derive(Debug)]
pub struct GrammarAbsoluteWeight<N, T>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    pub rules: FxHashMap<Rule<N, T>, u32>,
}

pub struct GrammarNormalisedWeight<N, T>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    pub rules: FxHashMap<Rule<N, T>, f64>,
}

impl<N, T> GrammarAbsoluteWeight<N, T>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            rules: FxHashMap::default(),
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

impl<N: Eq + Hash, T: Eq + Hash> Default for GrammarAbsoluteWeight<N, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Eq + Hash + Clone> From<Tree<A>> for GrammarAbsoluteWeight<A, A> {
    fn from(tree: Tree<A>) -> Self {
        let mut rule_set = GrammarAbsoluteWeight::new();

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
            }
            _ => {}
        }

        for child in tree.children {
            rule_set.absorb(GrammarAbsoluteWeight::from(child))
        }

        rule_set
    }
}

impl<N, T> GrammarNormalisedWeight<N, T>
where
    N: Eq + Hash + Display,
    T: Eq + Hash + Display,
{
    pub fn write_non_lexical_rules<W: Write>(&self, buf: &mut W) -> io::Result<()> {
        for (rule, weight) in &self.rules {
            if let Rule::NonLexical { lhs, rhs } = rule {
                write!(buf, "{} -> ", lhs)?;
                for n in rhs {
                    write!(buf, "{} ", n)?;
                }
                writeln!(buf, "{}", weight)?;
            }
        }

        Ok(())
    }

    pub fn write_lexical_rules<W: Write>(&self, buf: &mut W) -> io::Result<()> {
        for (rule, weight) in &self.rules {
            if let Rule::Lexical { lhs, rhs } = rule {
                write!(buf, "{} ", lhs)?;
                write!(buf, "{} ", rhs)?;
                writeln!(buf, "{}", weight)?;
            }
        }

        Ok(())
    }

    pub fn write_terminals<W: Write>(&self, buf: &mut W) -> io::Result<()> {
        let mut terminals = FxHashSet::default();

        for rule in self.rules.keys() {
            if let Rule::Lexical { lhs: _, rhs } = rule {
                let _ = terminals.insert(rhs);
            }
        }

        for terminal in terminals {
            writeln!(buf, "{}", terminal)?;
        }

        Ok(())
    }
}

impl<A: Eq + Hash + Clone> From<GrammarAbsoluteWeight<A, A>> for GrammarNormalisedWeight<A, A> {
    fn from(grammar: GrammarAbsoluteWeight<A, A>) -> Self {
        let mut grammar = grammar;

        let mut lhs_buckets = MultiMap::new();

        // Group rules by non-terminal on LHS of rule.
        for (rule, weight) in grammar.rules.drain() {
            let lhs = match &rule {
                Rule::Lexical { lhs: x, rhs: _ } => x,
                Rule::NonLexical { lhs: x, rhs: _ } => x,
            };

            lhs_buckets.insert(lhs.clone(), (rule, weight));
        }

        let mut grammar_map: FxHashMap<Rule<A, A>, f64> = FxHashMap::default();

        // Normalise weights.
        for (_, bucket) in lhs_buckets.iter_all() {
            let total = bucket.iter().fold(0, |acc, (_, x)| acc + x) as f64;

            for (rule, weight) in bucket {
                grammar_map.insert((*rule).clone(), (*weight as f64) / total);
            }
        }

        GrammarNormalisedWeight { rules: grammar_map }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_rule_induction_from_tree() {
        let rule_set = GrammarAbsoluteWeight::from(Tree {
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

        // Also works when constituent tree is a line.
        let rule_set = GrammarAbsoluteWeight::from(Tree {
            root: "NP".to_string(),
            children: vec![Tree {
                root: "D".to_string(),
                children: vec![Tree {
                    root: "the".to_string(),
                    children: vec![],
                }],
            }],
        });

        assert_eq!(
            rule_set.rules.get(&Rule::Lexical {
                lhs: "D".to_string(),
                rhs: "the".to_string()
            }),
            Some(&1)
        );
    }

    #[test]
    fn rule_induction_with_duplicate() {
        let rule_set = GrammarAbsoluteWeight::from(Tree {
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

    #[test]
    fn rule_normalisation() {
        let normalised_grammar = GrammarNormalisedWeight::from(GrammarAbsoluteWeight::from(Tree {
            root: "NP".to_string(),
            children: vec![
                Tree {
                    root: "NP".to_string(),
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
        }));

        assert!(normalised_grammar.rules.len() == 3);

        assert_eq!(
            normalised_grammar.rules.get(&Rule::NonLexical {
                lhs: "NP".to_string(),
                rhs: vec!["NP".to_string(), "D".to_string()]
            }),
            Some(&(1.0 / 2.0))
        );
        assert_eq!(
            normalised_grammar.rules.get(&Rule::Lexical {
                lhs: "NP".to_string(),
                rhs: "the".to_string()
            }),
            Some(&(1.0 / 2.0))
        );
        assert_eq!(
            normalised_grammar.rules.get(&Rule::Lexical {
                lhs: "D".to_string(),
                rhs: "the".to_string()
            }),
            Some(&1.0)
        );
    }
}
