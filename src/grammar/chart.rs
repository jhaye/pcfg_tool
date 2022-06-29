use std::ops::{Index, IndexMut};

pub struct Chart<T> {
    data: Vec<T>,
    sentence_len: usize,
    num_nonterminals: usize,
}

type ChartIdx = usize;

impl<T> Chart<T>
where
    T: Clone + Default,
{
    pub fn new(sentence_len: usize, num_nonterminals: usize) -> Self {
        Self {
            data: vec![
                Default::default();
                (sentence_len * (sentence_len + 1) / 2) * num_nonterminals
            ],
            sentence_len,
            num_nonterminals,
        }
    }

    pub fn data(&self) -> &[T] {
        self.data.as_slice()
    }

    pub fn get_cell_mut(&mut self, start: ChartIdx) -> &mut [T] {
        &mut self.data[start..(start + self.num_nonterminals)]
    }

    /// Calculates the index for the corresponding cell.
    /// Individual cells are further subdivided for each entry.
    /// This offset has to be added afterwards.
    pub const fn cell_start_index(&self, start_pos: usize, span: usize) -> ChartIdx {
        let rows_subtract = self.sentence_len - span + 1;
        let base_cells_subtract = (rows_subtract * (rows_subtract + 1)) / 2;
        let num_base_cells = (self.sentence_len * (self.sentence_len + 1)) / 2;
        (num_base_cells - base_cells_subtract + start_pos) * self.num_nonterminals
    }
}

impl<T> Index<usize> for Chart<T> {
    type Output = T;

    fn index(&self, index: ChartIdx) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> IndexMut<usize> for Chart<T> {
    fn index_mut(&mut self, index: ChartIdx) -> &mut Self::Output {
        &mut self.data[index]
    }
}
