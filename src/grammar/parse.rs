use std::collections::BinaryHeap;
use std::hash::Hash;

use float_ord::FloatOrd;
use fxhash::FxHashMap;
use multimap::MultiMap;

use super::rule::{Rule, WeightedRule};
use crate::tree::NodeType;
use crate::Sentence;
use crate::Tree;

/// Reresents backtrace information used during the execution of the
/// cyk algorithm to construct the constituent tree.
/// For `Binary`, the contained  integers refer to the cell in c of that non-terminal.
/// For `Chain`, it refers to the non-terminal in the same cell in c.
/// For `Term` it represents the location of the terminal in the input sentence.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum BacktraceInfo {
    Binary(usize, usize),
    Chain(usize),
    Term(usize),
}

#[derive(Debug)]
/// Grammar built specifically for deriving most
/// probable constituent trees from sentences with
/// CYK algorithm.
pub struct GrammarParse<N, T, W>
where
    N: Eq + Hash,
    T: Eq + Hash,
    W: Copy + Default,
{
    initial_nonterminal: u32,
    // Lexical rules which we search by terminal on the RHS.
    rules_lexical: MultiMap<T, (u32, W)>,
    // Non-lexical rules with one non-terminals on the RHS.
    // We search by the non-terminal on the RHS.
    rules_chain: MultiMap<u32, (u32, W)>,
    // Non-lexical rules with two non-terminals on the RHS.
    // We search by non-terminal on the LHS.
    rules_double: MultiMap<u32, (u32, u32, W)>,
    // Lookup table for intified non-terminals.
    lookup: Vec<N>,
    lookup_index: FxHashMap<N, u32>,
}

impl<N, T> GrammarParse<N, T, FloatOrd<f64>>
where
    N: Eq + Hash + Clone,
    T: Eq + Hash + Clone,
{
    pub fn new(initial_nonterminal: N) -> Self {
        let mut result = Self {
            initial_nonterminal: 0,
            rules_lexical: MultiMap::new(),
            rules_chain: MultiMap::default(),
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

    pub fn insert_rule(&mut self, weighted_rule: WeightedRule<N, T, FloatOrd<f64>>) {
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

    pub fn cyk(&self, sentence: &Sentence<T>) -> Option<Tree<NodeType<N, T>>> {
        let num_nt = self.lookup.len();

        let mut c = vec![
            Default::default();
            (sentence.len() * (sentence.len() + 1) / 2) * self.lookup.len()
        ];

        for (i, word) in sentence.iter().enumerate() {
            if let Some(lexicals) = self.rules_lexical.get_vec(word) {
                for (nt, weight) in lexicals {
                    let nt = *nt as usize;
                    c[(i * num_nt) + nt] = (*weight, Some(BacktraceInfo::Term(i)));
                }
            }
            self.unary_closure(&mut c[i * num_nt..num_nt]);
        }

        for r in 2..sentence.len() {
            for i in 1..(sentence.len() - r) {
                let j = i + r;
                let i_j = cyk_cell_index(c.len(), i, r, num_nt);
                for a in 0..num_nt {
                    for m in (i + 1)..(j - 1) {
                        let i_m = cyk_cell_index(c.len(), i, m - i, num_nt);
                        let m_j = cyk_cell_index(c.len(), m, j - m, num_nt);
                        if let Some(binary_rules) = self.rules_double.get_vec(&(a as u32)) {
                            c[i_j + a] = c[i_j + a].max(
                                binary_rules
                                    .iter()
                                    .map(|(x, y, weight)| {
                                        (
                                            FloatOrd(
                                                weight.0
                                                    * c[i_m + (*x as usize)].0 .0
                                                    * c[m_j + (*y as usize)].0 .0,
                                            ),
                                            Some(BacktraceInfo::Binary(
                                                i_m + (*x as usize),
                                                m_j + (*y as usize),
                                            )),
                                        )
                                    })
                                    .max()
                                    .unwrap_or_default(),
                            );
                        }
                    }
                }
                self.unary_closure(&mut c[i_j..(i_j + num_nt)]);
            }
        }

        Self::construct_best_tree(
            &c,
            cyk_cell_index(c.len(), 0, sentence.len(), num_nt)
                + (self.initial_nonterminal as usize),
            sentence,
            &self.lookup,
        )
    }

    fn unary_closure(&self, c: &mut [(FloatOrd<f64>, Option<BacktraceInfo>)]) {
        // Use max heap so we can easily extract the element with
        // the greatest weight.
        let mut queue = BinaryHeap::with_capacity(c.len());

        // Fill the queue with elements from c.
        // We invert tuple order of non-terminal and weight in the
        // queue, so that the queue is sorted by weight.
        for ele in c
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, (w, _))| w.0 != 0.0)
            .map(|(i, w)| (w, i))
        {
            queue.push(ele);
        }
        for weight in c.iter_mut() {
            *weight = Default::default();
        }

        while let Some(((q, backtrace), b)) = queue.pop() {
            if q > c[b].0 {
                c[b] = (q, backtrace);
                if let Some(chain_rules) = self.rules_chain.get_vec(&(b as u32)) {
                    for (a, chain_weight) in chain_rules {
                        queue.push((
                            (
                                FloatOrd(chain_weight.0 * q.0),
                                Some(BacktraceInfo::Chain(b)),
                            ),
                            *a as usize,
                        ));
                    }
                }
            }
        }
    }

    fn construct_best_tree(
        c: &[(FloatOrd<f64>, Option<BacktraceInfo>)],
        c_idx: usize,
        sentence: &Sentence<T>,
        lookup: &[N],
    ) -> Option<Tree<NodeType<N, T>>> {
        let num_nt = lookup.len();

        match c[c_idx].1 {
            None => None,
            Some(BacktraceInfo::Term(t)) => Some(Tree {
                root: NodeType::Terminal(sentence.0[t].clone()),
                children: vec![],
            }),
            Some(BacktraceInfo::Chain(i)) => {
                if let Some(tree) =
                    Self::construct_best_tree(c, c_idx - (c_idx % num_nt) + i, sentence, lookup)
                {
                    let nt = c_idx % num_nt;
                    Some(Tree {
                        root: NodeType::NonTerminal(lookup[nt].clone()),
                        children: vec![tree],
                    })
                } else {
                    None
                }
            }
            Some(BacktraceInfo::Binary(i, j)) => {
                if let (Some(tree_i), Some(tree_j)) = (
                    Self::construct_best_tree(c, i, sentence, lookup),
                    Self::construct_best_tree(c, j, sentence, lookup),
                ) {
                    let nt = c_idx % num_nt;
                    Some(Tree {
                        root: NodeType::NonTerminal(lookup[nt].clone()),
                        children: vec![tree_i, tree_j],
                    })
                } else {
                    None
                }
            }
        }
    }
}

/// Calculates the index for the corresponding cell in c during the execution of the cyk algorithm.
/// Individual cells are further subdivided for each non-terminal. This offset has to be added
/// afterwards.
const fn cyk_cell_index(c_size: usize, start_pos: usize, span: usize, num_nt: usize) -> usize {
    ((c_size - (span * (span + 1)) / 2) - 1 + start_pos) * num_nt
}
