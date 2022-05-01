use std::hash::Hash;

use fxhash::FxHashMap;

use super::rule::{Rule, WeightedRule};

#[derive(Debug)]
pub struct GrammarIntified<N, T, W>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    pub rules: FxHashMap<Rule<u32, T>, W>,
    lookup: Vec<N>,
    lookup_index: FxHashMap<N, u32>,
}

impl<N, T, W> GrammarIntified<N, T, W>
where
    N: Eq + Hash + Clone,
    T: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            rules: FxHashMap::default(),
            lookup: vec![],
            lookup_index: FxHashMap::default(),
        }
    }

    fn intify(&mut self, n: N) -> u32 {
        self.lookup_index.get(&n).copied().unwrap_or_else(|| {
            let index = self.lookup.len() as u32;
            self.lookup.push(n.clone());
            self.lookup_index.insert(n, index);
            index
        })
    }

    pub fn insert_rule(&mut self, weighted_rule: WeightedRule<N, T, W>) {
        match weighted_rule.rule {
            Rule::Lexical { lhs, rhs } => {
                let rule = Rule::Lexical {
                    lhs: self.intify(lhs),
                    rhs,
                };

                self.rules.insert(rule, weighted_rule.weight);
            }

            Rule::NonLexical { lhs, mut rhs } => {
                let rule = Rule::NonLexical {
                    lhs: self.intify(lhs),
                    rhs: rhs.drain(..).map(|n| self.intify(n)).collect(),
                };

                self.rules.insert(rule, weighted_rule.weight);
            }
        };
    }
}

impl<N, T, W> Default for GrammarIntified<N, T, W>
where
    N: Eq + Hash + Clone,
    T: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}
