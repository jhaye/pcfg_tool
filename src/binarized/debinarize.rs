use super::node::{Binarized, MarkovizedNode};
use crate::tree::Tree;

impl<A> Tree<Binarized<A>> {
    pub fn debinarize(mut self) -> Tree<A> {
        if self.is_leaf() {
            let root = match self.root {
                Binarized::Bare(a) => a,
                Binarized::Markovized(_) => panic!("Leaf node cannot be markovized!"),
            };

            Tree {
                root,
                children: vec![],
            }
        } else if let Binarized::Markovized(_) = self.children.iter().last().unwrap().root {
            let last = self.children.pop().unwrap();
            self.children.extend(last.children);
            self.debinarize()
        } else {
            let root = match self.root {
                Binarized::Bare(a) => a,
                Binarized::Markovized(MarkovizedNode { label: a, .. }) => a,
            };

            Tree {
                root,
                children: self.children.drain(..).map(|c| c.debinarize()).collect(),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sexp::SExp;
    use std::str::FromStr;

    #[test]
    fn debinarize_successful() {
        let binarized_tree = Tree::from(
            SExp::from_str("(ROOT (FRAG^<ROOT> (RB (Not)) (FRAG|<NP-TMP,.>^<ROOT> (NP-TMP^<FRAG,ROOT> (DT (this)) (NN (year))) (. (.)))))")
                .unwrap(),
        );
        let binarized_tree = binarized_tree.parse_markovized();
        assert_eq!(
            "(ROOT (RB Not) (NP-TMP (DT this) (NN year)) (. .))".to_string(),
            format!("{}", binarized_tree.debinarize())
        );

        let binarized_tree2 = Tree::from(
            SExp::from_str(
                "(S (A (A1 a) (A|<A2,A3> (A2 a) (A3 a))) (S|<B,C,D> (B b) (S|<C,D> (C c) (D d))))",
            )
            .unwrap(),
        );
        let binarized_tree2 = binarized_tree2.parse_markovized();
        assert_eq!(
            "(S (A (A1 a) (A2 a) (A3 a)) (B b) (C c) (D d))".to_string(),
            format!("{}", binarized_tree2.debinarize())
        );
    }
}
