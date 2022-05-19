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
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
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
        let s_len = sentence.len();

        let mut c = vec![Default::default(); (s_len * (s_len + 1) / 2) * num_nt];

        for (i, word) in sentence.iter().enumerate() {
            if let Some(lexicals) = self.rules_lexical.get_vec(word) {
                for (nt, weight) in lexicals {
                    let nt = *nt as usize;
                    c[(i * num_nt) + nt] = (*weight, Some(BacktraceInfo::Term(i)));
                }
            }
            self.unary_closure(&mut c[(i * num_nt)..((i + 1) * num_nt)]);
        }

        for r in 2..=s_len {
            for i in 0..=(s_len - r) {
                let j = i + r;
                let i_j = cyk_cell_index(i, r, s_len, num_nt);
                for a in 0..num_nt {
                    for m in (i + 1)..j {
                        let i_m = cyk_cell_index(i, m - i, s_len, num_nt);
                        let m_j = cyk_cell_index(m, j - m, s_len, num_nt);

                        if let Some(binary_rules) = self.rules_double.get_vec(&(a as u32)) {
                            c[i_j + a] = c[i_j + a].max(
                                binary_rules
                                    .iter()
                                    .map(|(x, y, weight)| {
                                        let x = *x as usize;
                                        let y = *y as usize;
                                        (
                                            FloatOrd(
                                                weight.0 * c[i_m + (x)].0 .0 * c[m_j + (y)].0 .0,
                                            ),
                                            Some(BacktraceInfo::Binary(i_m + (x), m_j + (y))),
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
            cyk_cell_index(0, s_len, s_len, num_nt) + (self.initial_nonterminal as usize),
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
                root: NodeType::NonTerminal(lookup[c_idx % num_nt].clone()),
                children: vec![Tree {
                    root: NodeType::Terminal(sentence.0[t].clone()),
                    children: vec![],
                }],
            }),
            Some(BacktraceInfo::Chain(i)) => {
                let nt = c_idx % num_nt;
                if let Some(tree) = Self::construct_best_tree(c, c_idx - nt + i, sentence, lookup) {
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
const fn cyk_cell_index(
    start_pos: usize,
    span: usize,
    sentence_len: usize,
    num_nt: usize,
) -> usize {
    let rows_subtract = sentence_len - span + 1;
    let base_cells_subtract = (rows_subtract * (rows_subtract + 1)) / 2;
    let num_base_cells = (sentence_len * (sentence_len + 1)) / 2;
    (num_base_cells - base_cells_subtract + start_pos) * num_nt
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn cyk_base_correct() {
        let mut grammar = GrammarParse::new("S".to_string());

        grammar.insert_rule(WeightedRule {
            rule: Rule::NonLexical {
                lhs: "S".to_string(),
                rhs: vec!["NP".to_string(), "VP".to_string()],
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::NonLexical {
                lhs: "VP".to_string(),
                rhs: vec!["VP".to_string(), "PP".to_string()],
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::NonLexical {
                lhs: "VP".to_string(),
                rhs: vec!["V".to_string(), "NP".to_string()],
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::NonLexical {
                lhs: "PP".to_string(),
                rhs: vec!["P".to_string(), "NP".to_string()],
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::NonLexical {
                lhs: "NP".to_string(),
                rhs: vec!["Det".to_string(), "N".to_string()],
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::NonLexical {
                lhs: "NP".to_string(),
                rhs: vec!["PN".to_string()],
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::Lexical {
                lhs: "VP".to_string(),
                rhs: "eats".to_string(),
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::Lexical {
                lhs: "PN".to_string(),
                rhs: "she".to_string(),
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::Lexical {
                lhs: "V".to_string(),
                rhs: "eats".to_string(),
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::Lexical {
                lhs: "P".to_string(),
                rhs: "with".to_string(),
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::Lexical {
                lhs: "N".to_string(),
                rhs: "fish".to_string(),
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::Lexical {
                lhs: "N".to_string(),
                rhs: "fork".to_string(),
            },
            weight: FloatOrd(1.0),
        });
        grammar.insert_rule(WeightedRule {
            rule: Rule::Lexical {
                lhs: "Det".to_string(),
                rhs: "a".to_string(),
            },
            weight: FloatOrd(1.0),
        });

        let tree = Tree {
            root: NodeType::NonTerminal("S".to_string()),
            children: vec![
                Tree {
                    root: NodeType::NonTerminal("NP".to_string()),
                    children: vec![Tree {
                        root: NodeType::Terminal("she".to_string()),
                        children: vec![],
                    }],
                },
                Tree {
                    root: NodeType::NonTerminal("VP".to_string()),
                    children: vec![
                        Tree {
                            root: NodeType::NonTerminal("VP".to_string()),
                            children: vec![
                                Tree {
                                    root: NodeType::Terminal("eats".to_string()),
                                    children: vec![],
                                },
                                Tree {
                                    root: NodeType::NonTerminal("NP".to_string()),
                                    children: vec![
                                        Tree {
                                            root: NodeType::Terminal("a".to_string()),
                                            children: vec![],
                                        },
                                        Tree {
                                            root: NodeType::Terminal("fish".to_string()),
                                            children: vec![],
                                        },
                                    ],
                                },
                            ],
                        },
                        Tree {
                            root: NodeType::NonTerminal("PP".to_string()),
                            children: vec![
                                Tree {
                                    root: NodeType::Terminal("with".to_string()),
                                    children: vec![],
                                },
                                Tree {
                                    root: NodeType::NonTerminal("NP".to_string()),
                                    children: vec![
                                        Tree {
                                            root: NodeType::Terminal("a".to_string()),
                                            children: vec![],
                                        },
                                        Tree {
                                            root: NodeType::Terminal("fork".to_string()),
                                            children: vec![],
                                        },
                                    ],
                                },
                            ],
                        },
                    ],
                },
            ],
        };

        let sentence = Sentence(vec![
            "she".to_string(),
            "eats".to_string(),
            "a".to_string(),
            "fish".to_string(),
            "with".to_string(),
            "a".to_string(),
            "fork".to_string(),
        ]);

        assert_eq!(tree, grammar.cyk(&sentence).unwrap());
    }
}
