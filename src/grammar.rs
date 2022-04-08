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
    rules: HashMap<Rule<N, T>, u32>,
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

    pub fn absorb(&mut self, mut other: Self) {
        other
            .rules
            .drain()
            .for_each(|(r, w)| self.insert_with_weight(r, w));
    }
}
