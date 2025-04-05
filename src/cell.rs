#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
}

impl Cell {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}
