use std::collections::VecDeque;
use std::str::FromStr;

use smallstr::SmallString;

use super::node::{Binarized, MarkovizedNode};
use crate::tree::Tree;

impl Tree<SmallString<[u8; 8]>> {
    pub fn markovize(
        mut self,
        v: usize,
        h: usize,
        parents: &[SmallString<[u8; 8]>],
    ) -> Tree<Binarized<SmallString<[u8; 8]>>> {
        // is preterminal
        if self.children.iter().all(|c| c.is_leaf()) {
            Tree {
                root: Binarized::Bare(self.root),
                children: self
                    .children
                    .drain(..)
                    .map(|c| Tree {
                        root: Binarized::Bare(c.root),
                        children: vec![],
                    })
                    .collect(),
            }
        } else if self.children.len() <= 2 {
            let new_root = Binarized::from_str(&self.root).unwrap();

            let parents_augmented = augment_parents(parents, new_root.extract_label().clone(), v);

            Tree {
                root: match new_root {
                    Binarized::Markovized(mut a) => {
                        a.ancestors = Vec::from(parents);
                        Binarized::Markovized(a)
                    }
                    Binarized::Bare(a) => Binarized::Markovized(MarkovizedNode {
                        label: a,
                        children: vec![],
                        ancestors: Vec::from(parents),
                    }),
                },
                children: self
                    .children
                    .drain(..)
                    .map(|c| c.markovize(v, h, &parents_augmented))
                    .collect(),
            }
        } else {
            let augmented_label = MarkovizedNode {
                label: Binarized::from_str(&self.root)
                    .unwrap()
                    .extract_label()
                    .clone(),
                children: self
                    .children
                    .iter()
                    .skip(1)
                    .take(h)
                    .map(|c| &c.root)
                    .cloned()
                    .collect(),
                ancestors: vec![],
            };

            let parents_augmented = augment_parents(parents, augmented_label.label.clone(), v);

            Tree {
                root: Binarized::Markovized(MarkovizedNode {
                    label: self.root.clone(),
                    children: vec![],
                    ancestors: Vec::from(parents),
                }),
                children: vec![
                    self.children[0].clone().markovize(v, h, &parents_augmented),
                    Tree {
                        // We have to convert the markovized node back into a string
                        // to make the recursion work.
                        root: format!("{}", augmented_label).into(),
                        children: self.children[1..].to_vec(),
                    }
                    .markovize(v, h, parents),
                ],
            }
        }
    }
}

fn augment_parents<T: Clone>(parents: &[T], augmenter: T, v: usize) -> Vec<T> {
    if v == 0 || v == 1 {
        vec![]
    } else {
        let vec = Vec::from(parents);
        let mut queue = VecDeque::from(vec);
        queue.push_front(augmenter);
        queue.truncate(v - 1);
        queue.drain(..).collect()
    }
}

#[cfg(test)]
mod test {
    use crate::sexp::SExp;
    use crate::tree::Tree;
    use std::str::FromStr;

    #[test]
    fn markovization_valid() {
        let tree = Tree::from(
            SExp::from_str("(ROOT (FRAG (RB Not) (NP-TMP (DT this) (NN year)) (. .)))").unwrap(),
        );

        let markovized_tree = tree.clone().markovize(4, 999, &[]);
        assert_eq!("(ROOT (FRAG^<ROOT> (RB Not) (FRAG|<NP-TMP,.>^<ROOT> (NP-TMP^<FRAG,ROOT> (DT this) (NN year)) (. .))))".to_string(), format!("{}", markovized_tree));

        let markovized_tree2 = tree.clone().markovize(2, 999, &[]);
        assert_eq!("(ROOT (FRAG^<ROOT> (RB Not) (FRAG|<NP-TMP,.>^<ROOT> (NP-TMP^<FRAG> (DT this) (NN year)) (. .))))".to_string(), format!("{}", markovized_tree2));

        let markovized_tree3 = tree.clone().markovize(1, 999, &[]);
        assert_eq!(
            "(ROOT (FRAG (RB Not) (FRAG|<NP-TMP,.> (NP-TMP (DT this) (NN year)) (. .))))"
                .to_string(),
            format!("{}", markovized_tree3)
        );

        let markovized_tree4 = tree.clone().markovize(1, 1, &[]);
        assert_eq!(
            "(ROOT (FRAG (RB Not) (FRAG|<NP-TMP> (NP-TMP (DT this) (NN year)) (. .))))".to_string(),
            format!("{}", markovized_tree4)
        );

        let markovized_tree5 = tree.markovize(1, 0, &[]);
        assert_eq!(
            "(ROOT (FRAG (RB Not) (FRAG (NP-TMP (DT this) (NN year)) (. .))))".to_string(),
            format!("{}", markovized_tree5)
        );

        let markovized_tree6 = Tree::from(
            SExp::from_str("(S (A a) (B (BB (BBB (B1 b) (B2 b) (B3 b)))) (C c) (D d))").unwrap(),
        )
        .markovize(1, 999, &[]);
        assert_eq!(
            "(S (A a) (S|<B,C,D> (B (BB (BBB (B1 b) (BBB|<B2,B3> (B2 b) (B3 b))))) (S|<C,D> (C c) (D d))))".to_string(),
            format!("{}", markovized_tree6)
        );
    }
}
