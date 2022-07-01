use std::collections::BinaryHeap;
use std::hash::Hash;

use float_ord::FloatOrd;
use fxhash::{FxBuildHasher, FxHashMap};
use multimap::MultiMap;

use super::chart::Chart;
use super::rule::{Rule, WeightedRule};
use crate::tree::NodeType;
use crate::Sentence;
use crate::Tree;

type ChartEntry = (FloatOrd<f64>, Option<BacktraceInfo>);
type IntNt = u32;

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

/// Differentiates modes for the CYK algorithm. Pruning is further differentiated into
/// threshold beam and fixed-size beam.
#[derive(Copy, Clone)]
pub enum CykMode {
    Base,
    PruneThreshold(f64),
    PruneFixedSize(usize),
}

impl CykMode {
    fn is_prune(&self) -> bool {
        match self {
            CykMode::Base => false,
            CykMode::PruneThreshold(_) | CykMode::PruneFixedSize(_) => true,
        }
    }
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
    initial_nonterminal: IntNt,
    // Lexical rules which we search by terminal on the RHS.
    pub rules_lexical: MultiMap<T, (IntNt, W), FxBuildHasher>,
    // Non-lexical rules with one non-terminals on the RHS.
    // We search by the non-terminal on the RHS.
    rules_chain: MultiMap<IntNt, (IntNt, W), FxBuildHasher>,
    // Non-lexical rules with two non-terminals on the RHS.
    // We search by non-terminal on the LHS.
    rules_double: MultiMap<IntNt, (IntNt, IntNt, W), FxBuildHasher>,
    // Lookup table for intified non-terminals.
    lookup: Vec<N>,
    lookup_index: FxHashMap<N, IntNt>,
}

impl<N, T> GrammarParse<N, T, FloatOrd<f64>>
where
    N: Eq + Hash + Clone,
    T: Eq + Hash + Clone,
{
    pub fn new(initial_nonterminal: N) -> Self {
        let mut result = Self {
            initial_nonterminal: 0,
            rules_lexical: MultiMap::default(),
            rules_chain: MultiMap::default(),
            rules_double: MultiMap::default(),
            lookup: vec![],
            lookup_index: FxHashMap::default(),
        };
        result.initial_nonterminal = result.intify(initial_nonterminal);

        result
    }

    fn intify(&mut self, n: N) -> IntNt {
        self.lookup_index.get(&n).copied().unwrap_or_else(|| {
            let index = self.lookup.len() as IntNt;
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

    pub fn cyk(&self, sentence: &Sentence<T>, mode: CykMode) -> Option<Tree<NodeType<N, T>>> {
        let num_nt = self.lookup.len();
        let s_len = sentence.len();
        const ZERO: FloatOrd<f64> = FloatOrd(0.0);

        let mut chart: Chart<ChartEntry> = Chart::new(s_len, num_nt);
        self.chart_setup(sentence, &mut chart, &mode);

        for r in 2..=s_len {
            for i in 0..=(s_len - r) {
                let j = i + r;
                let i_j = chart.cell_start_index(i, r);
                for a in 0..num_nt {
                    for m in (i + 1)..j {
                        let i_m = chart.cell_start_index(i, m - i);
                        let m_j = chart.cell_start_index(m, j - m);

                        if let Some(binary_rules) = self.rules_double.get_vec(&(a as IntNt)) {
                            let binary_rules_iter = binary_rules
                                .iter()
                                .map(|(b, c, w)| (*b as usize, *c as usize, w))
                                .filter(|(b, c, _)| {
                                    // Manually filter out zero factors for pruning.
                                    // This provides a significant speedup.
                                    if mode.is_prune() {
                                        chart[i_m + *b].0 > ZERO && chart[m_j + *c].0 > ZERO
                                    } else {
                                        true
                                    }
                                });

                            chart[i_j + a] = chart[i_j + a].max(
                                binary_rules_iter
                                    .map(|(b, c, weight)| {
                                        (
                                            FloatOrd(
                                                weight.0
                                                    * chart[i_m + b].0 .0
                                                    * chart[m_j + c].0 .0,
                                            ),
                                            Some(BacktraceInfo::Binary(i_m + b, m_j + c)),
                                        )
                                    })
                                    .max()
                                    .unwrap_or_default(),
                            );
                        }
                    }
                }
                self.unary_closure(chart.get_cell_mut(i_j));
                match mode {
                    CykMode::Base => {}
                    CykMode::PruneThreshold(t) => self.prune_threshold(chart.get_cell_mut(i_j), t),
                    CykMode::PruneFixedSize(n) => self.prune_fixed_size(chart.get_cell_mut(i_j), n),
                }
            }
        }

        let root_cell = chart.cell_start_index(0, s_len) + (self.initial_nonterminal as usize);
        Self::construct_best_tree(chart.data(), root_cell, sentence, &self.lookup)
    }

    fn chart_setup(&self, sentence: &Sentence<T>, chart: &mut Chart<ChartEntry>, mode: &CykMode) {
        let num_nt = chart.num_nt();

        for (i, word) in sentence.iter().enumerate() {
            if let Some(lexicals) = self.rules_lexical.get_vec(word) {
                for (nt, weight) in lexicals {
                    let nt = *nt as usize;
                    chart[(i * num_nt) + nt] = (*weight, Some(BacktraceInfo::Term(i)));
                }
            }
            self.unary_closure(chart.get_cell_mut(i * num_nt));
            match mode {
                CykMode::Base => {}
                CykMode::PruneThreshold(t) => {
                    self.prune_threshold(chart.get_cell_mut(i * num_nt), *t)
                }
                CykMode::PruneFixedSize(n) => {
                    self.prune_fixed_size(chart.get_cell_mut(i * num_nt), *n)
                }
            }
        }
    }

    fn unary_closure(&self, c: &mut [ChartEntry]) {
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
                if let Some(chain_rules) = self.rules_chain.get_vec(&(b as IntNt)) {
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

    /// Zeroes all entries that are smaller than best probability in the given cell
    /// multiplied by `threshold`;
    fn prune_threshold(&self, c: &mut [ChartEntry], threshold: f64) {
        let m = c.iter().max().unwrap().0;
        let cutoff = FloatOrd(m.0 * threshold);

        for chart_ele in c {
            if chart_ele.0 < cutoff {
                *chart_ele = Default::default();
            }
        }
    }

    /// Zeroes all entries that are smaller than the n-best entry in the cell.
    /// If `n` is less than the cell size, we use the last entry.
    fn prune_fixed_size(&self, c: &mut [ChartEntry], n: usize) {
        let num_nt = c.len();

        let n_best = {
            let mut sorted = vec![Default::default(); c.len()];
            sorted.copy_from_slice(c);
            sorted.sort_unstable();
            sorted.reverse();

            if n > num_nt {
                *sorted.last().unwrap()
            } else {
                sorted[n - 1]
            }
        };

        for chart_ele in c {
            if *chart_ele < n_best {
                *chart_ele = Default::default();
            }
        }
    }

    fn construct_best_tree(
        c: &[ChartEntry],
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
                Self::construct_best_tree(c, c_idx - nt + i, sentence, lookup).map(|tree| Tree {
                    root: NodeType::NonTerminal(lookup[nt].clone()),
                    children: vec![tree],
                })
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
                        root: NodeType::NonTerminal("PN".to_string()),
                        children: vec![Tree {
                            root: NodeType::Terminal("she".to_string()),
                            children: vec![],
                        }],
                    }],
                },
                Tree {
                    root: NodeType::NonTerminal("VP".to_string()),
                    children: vec![
                        Tree {
                            root: NodeType::NonTerminal("VP".to_string()),
                            children: vec![
                                Tree {
                                    root: NodeType::NonTerminal("V".to_string()),
                                    children: vec![Tree {
                                        root: NodeType::Terminal("eats".to_string()),
                                        children: vec![],
                                    }],
                                },
                                Tree {
                                    root: NodeType::NonTerminal("NP".to_string()),
                                    children: vec![
                                        Tree {
                                            root: NodeType::NonTerminal("Det".to_string()),
                                            children: vec![Tree {
                                                root: NodeType::Terminal("a".to_string()),
                                                children: vec![],
                                            }],
                                        },
                                        Tree {
                                            root: NodeType::NonTerminal("N".to_string()),
                                            children: vec![Tree {
                                                root: NodeType::Terminal("fish".to_string()),
                                                children: vec![],
                                            }],
                                        },
                                    ],
                                },
                            ],
                        },
                        Tree {
                            root: NodeType::NonTerminal("PP".to_string()),
                            children: vec![
                                Tree {
                                    root: NodeType::NonTerminal("P".to_string()),
                                    children: vec![Tree {
                                        root: NodeType::Terminal("with".to_string()),
                                        children: vec![],
                                    }],
                                },
                                Tree {
                                    root: NodeType::NonTerminal("NP".to_string()),
                                    children: vec![
                                        Tree {
                                            root: NodeType::NonTerminal("Det".to_string()),
                                            children: vec![Tree {
                                                root: NodeType::Terminal("a".to_string()),
                                                children: vec![],
                                            }],
                                        },
                                        Tree {
                                            root: NodeType::NonTerminal("N".to_string()),
                                            children: vec![Tree {
                                                root: NodeType::Terminal("fork".to_string()),
                                                children: vec![],
                                            }],
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

        let sentence = Sentence(vec![
            "a".to_string(),
            "fish".to_string(),
            "doesn't".to_string(),
            "eat".to_string(),
            "with".to_string(),
            "a".to_string(),
            "fork".to_string(),
        ]);
        assert!(grammar.cyk(&sentence).is_none());
    }
}
