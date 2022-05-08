use std::hash::Hash;

use fxhash::FxHashMap;
use multimap::MultiMap;

use super::rule::{Rule, WeightedRule};

#[derive(Debug)]
/// Grammar built specifically for deriving most
/// probable constituent trees from sentences with
/// CYK algorithm.
pub struct GrammarParse<N, T, W>
where
    N: Eq + Hash,
    T: Eq + Hash,
{
    initial_nonterminal: u32,
    // Lexical rules which we search by terminal on the RHS.
    rules_lexical: MultiMap<T, (u32, W)>,
    // Non-lexical rules with one non-terminals on the RHS.
    // We search by the non-terminal on the RHS.
    rules_chain: FxHashMap<u32, (u32, W)>,
    // Non-lexical rules with two non-terminals on the RHS.
    // We search by non-terminal on the LHS.
    rules_double: MultiMap<u32, (u32, u32, W)>,
    lookup: Vec<N>,
    lookup_index: FxHashMap<N, u32>,
}

impl<N, T, W> GrammarParse<N, T, W>
where
    N: Eq + Hash + Clone,
    T: Eq + Hash,
{
    pub fn new(initial_nonterminal: N) -> Self {
        let mut result = Self {
            initial_nonterminal: 0,
            rules_lexical: MultiMap::new(),
            rules_chain: FxHashMap::default(),
            rules_double: MultiMap::new(),
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
                let lhs = self.intify(lhs);
                self.rules_lexical.insert(rhs, (lhs, weighted_rule.weight));
            }

            Rule::NonLexical { lhs, mut rhs } => {
                let lhs = self.intify(lhs);
                let rhs: Vec<_> = rhs.drain(..).map(|n| self.intify(n)).collect();

                match rhs.as_slice() {
                    [n] => {
                        self.rules_chain.insert(*n, (lhs, weighted_rule.weight));
                    }
                    [n1, n2] => self
                        .rules_double
                        .insert(lhs, (*n1, *n2, weighted_rule.weight)),
                    _ => panic!("Parsing is only supported with binarised grammar rules!"),
                }
            }
        };
    }
}
