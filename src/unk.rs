use crate::tree::Tree;
use fxhash::FxHashMap;
use std::hash::Hash;

pub fn count_words<T: Eq + Hash + Clone>(tree: &Tree<T>, word_count: &mut FxHashMap<T, usize>) {
    if tree.is_leaf() {
        if let Some(v) = word_count.get_mut(&tree.root) {
            *v += 1;
        } else {
            word_count.insert(tree.root.clone(), 1);
        }
    }

    for child in &tree.children {
        count_words(child, word_count);
    }
}

impl<A: Eq + Hash + From<&'static str>> Tree<A> {
    /// Replaces words in this constituent tree with "UNK",
    /// if it is not contained in the keys of `words`.
    pub fn unkify(&mut self, words: &FxHashMap<A, usize>) {
        if self.is_leaf() && !words.contains_key(&self.root) {
            self.root = A::from("UNK");
        }

        for child in &mut self.children {
            child.unkify(words);
        }
    }
}
