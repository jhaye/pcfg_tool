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

            let parents_augmented = if v == 0 {
                vec![]
            } else {
                // Ensure that parents down the call tree don't exceed v.
                let mut vec = if parents.len() == v {
                    parents.iter().skip(1).cloned().collect()
                } else {
                    Vec::from(parents)
                };
                vec.push(new_root.extract_label().clone());
                vec
            };

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

            Tree {
                root: Binarized::Markovized(MarkovizedNode {
                    label: self.root.clone(),
                    children: vec![],
                    ancestors: Vec::from(parents),
                }),
                children: vec![
                    self.children[0].clone().markovize(v, h, parents),
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

        let markovized_tree = tree.clone().markovize(3, 999, &[]);
        assert_eq!("(ROOT (FRAG^<ROOT> (RB Not) (FRAG|<NP-TMP,.>^<ROOT> (NP-TMP^<ROOT,FRAG> (DT this) (NN year)) (. .))))".to_string(), format!("{}", markovized_tree));

        let markovized_tree2 = tree.clone().markovize(1, 999, &[]);
        assert_eq!("(ROOT (FRAG^<ROOT> (RB Not) (FRAG|<NP-TMP,.>^<ROOT> (NP-TMP^<FRAG> (DT this) (NN year)) (. .))))".to_string(), format!("{}", markovized_tree2));

        let markovized_tree3 = tree.clone().markovize(0, 999, &[]);
        assert_eq!(
            "(ROOT (FRAG (RB Not) (FRAG|<NP-TMP,.> (NP-TMP (DT this) (NN year)) (. .))))"
                .to_string(),
            format!("{}", markovized_tree3)
        );

        let markovized_tree4 = tree.clone().markovize(0, 1, &[]);
        assert_eq!(
            "(ROOT (FRAG (RB Not) (FRAG|<NP-TMP> (NP-TMP (DT this) (NN year)) (. .))))".to_string(),
            format!("{}", markovized_tree4)
        );

        let markovized_tree5 = tree.markovize(0, 0, &[]);
        assert_eq!(
            "(ROOT (FRAG (RB Not) (FRAG (NP-TMP (DT this) (NN year)) (. .))))".to_string(),
            format!("{}", markovized_tree5)
        );
    }
}