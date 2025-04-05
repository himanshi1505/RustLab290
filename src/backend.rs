use crate::cell::Cell;

#[derive(Debug, Clone, PartialEq)]
pub enum CellError {
    NoError,
    CircularDependency,
    ParseError,
    DivisionByZero,
    // Add other error variants as needed
}

#[derive(Debug, Clone)]
pub enum Function {
    PlusOp { first: Operand, second: Operand },
    MinusOp { first: Operand, second: Operand },
    MultiplyOp { first: Operand, second: Operand },
    DivideOp { first: Operand, second: Operand },
    Constant(i32),
    // Add other function variants
}

#[derive(Debug, Clone)]
pub enum Operand {
    Cell(Cell),
    Int(i32),
}

#[derive(Debug, Default)]
pub struct CellData {
    pub function: Function,
    pub error: CellError,
    pub dependents: Vec<Cell>,
    pub value: i32,
    pub dirty_parents: bool,
}

pub struct Backend {
    grid: Vec<Vec<CellData>>,
    rows: usize,
    cols: usize,
}

impl Backend {
    pub fn new(rows: usize, cols: usize) -> Self {  //init backend
        let mut grid = Vec::new();
    
        for _ in 0..rows {
            let mut row = Vec::new();
            for _ in 0..cols {
                row.push(CellData::default());
            }
            grid.push(row);
        }
    
        Backend { grid, rows, cols }
    }

    pub fn get_cell_value(&self, cell: Cell) -> Option<(i32, CellError)> {
        self.grid
            .get(cell.row as usize)
            .and_then(|row| row.get(cell.col as usize))
            .map(|data| (data.value, data.error.clone()))
    }

    pub fn reset(&mut self) {
        for row in &mut self.grid {
            for cell in row {
                *cell = CellData::default();
            }
        }
    }

    
}

impl Default for CellData {
    fn default() -> Self {
        CellData {
            function: Function::Constant(0),
            error: CellError::NoError,
            dependents: Vec::new(),
            value: 0,
            dirty_parents: false,
        }
    }
}
