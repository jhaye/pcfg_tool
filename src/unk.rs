use core::hash::BuildHasher;
use fxhash::FxHashMap;
use multimap::MultiMap;
use std::hash::Hash;

use crate::sentence::Sentence;
use crate::signature::UnkSignature;
use crate::tree::{NodeType, Tree};

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

impl<A: Eq + Hash + From<&'static str> + From<String> + AsRef<str>> Tree<A> {
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

    /// Replaces words in this constituent tree with their respective
    /// signature, if it is not contained in the keys of `words`.
    pub fn smooth(&mut self, words: &FxHashMap<A, usize>) {
        for (i, leaf) in self.leaves_mut().drain(..).enumerate() {
            if !words.contains_key(leaf) {
                let signature = UnkSignature::new(leaf.as_ref(), i);
                *leaf = signature.to_string().into();
            }
        }
    }
}

impl<N, T> Tree<NodeType<N, T>> {
    pub fn deunkify(&mut self, word_map: Vec<(usize, T)>) {
        let mut leaves = self.leaves_mut();

        for (i, word) in word_map {
            *leaves[i] = NodeType::Terminal(word);
        }
    }
}

impl<A> Sentence<A>
where
    A: Eq + Hash + From<&'static str> + From<String> + AsRef<str> + Clone,
{
    /// For use with `GrammarParse`'s `rules_lexical`.
    pub fn unkify(
        &mut self,
        words: &MultiMap<A, impl Default, impl BuildHasher>,
    ) -> Option<Vec<(usize, A)>> {
        let mut result = vec![];

        for (i, word) in self.iter_mut().enumerate() {
            if !words.contains_key(word) {
                result.push((i, word.clone()));
                *word = "UNK".into();
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// For use with `GrammarParse`'s `rules_lexical`.
    pub fn smooth(
        &mut self,
        words: &MultiMap<A, impl Default, impl BuildHasher>,
    ) -> Option<Vec<(usize, A)>> {
        let mut result = vec![];

        for (i, word) in self.iter_mut().enumerate() {
            if !words.contains_key(word) {
                result.push((i, word.clone()));
                let signature = UnkSignature::new(word.as_ref(), i);
                *word = signature.to_string().into();
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}
