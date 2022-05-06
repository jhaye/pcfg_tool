use std::hash::Hash;

use fxhash::FxHashMap;

use super::rule::{Rule, WeightedRule};

#[derive(Debug)]
pub struct GrammarIntified<N, T, W>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    initial_nonterminal: u32,
    pub rules: FxHashMap<Rule<u32, T>, W>,
    lookup: Vec<N>,
    lookup_index: FxHashMap<N, u32>,
}

impl<N, T, W> GrammarIntified<N, T, W>
where
    N: Eq + Hash + Clone,
    T: Eq + Hash,
{
    pub fn new(initial_nonterminal: N) -> Self {
        let mut result = Self {
            initial_nonterminal: 0,
            rules: FxHashMap::default(),
            lookup: vec![],
            lookup_index: FxHashMap::default(),
        };
        result.initial_nonterminal = result.intify(initial_nonterminal);

        result
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
