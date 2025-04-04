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
    /// Checks if setting this cell creates a circular dependency
    pub fn is_in_cycle(&mut self, start: &Cell) -> bool {
        let mut stack = vec![start];
        let mut found_cycle = false;

        // Mark starting cell as visited
        self.grid[start.row][start.col].dirty_parents = true;

        while let Some(current) = stack.pop() {
            // Get all dependents of current cell
            let dependents = & mut self.grid[current.row][current.col].dependents;

            for dep in dependents {
                // Cycle detected if we return to start cell
                if dep == start {
                    found_cycle = true;
                    break;
                }

                // Only process unvisited cells
                if !self.grid[dep.row][dep.col].dirty_parents {
                    self.grid[dep.row][dep.col].dirty_parents = true;
                    stack.push(dep);
                }
            }

            if found_cycle {
                break;
            }
        }

        // Reset visited markers
        self.reset_found(start);
        found_cycle
    }

    /// Resets dirty_parents flags for all cells reachable from start
    pub fn reset_found(&mut self, start: &Cell) {
        let mut stack = vec![start]; //NOTE:::: change to our VecWrapper to save sapce and time complexity 
        self.grid[start.row][start.col].dirty_parents = false;

        while let Some(current) = stack.pop() {
            let dependents = & mut self.grid[current.row][current.col].dependents;

            for dep in dependents {
                if self.grid[dep.row][dep.col].dirty_parents {
                    self.grid[dep.row][dep.col].dirty_parents = false;
                    stack.push(dep); //NOTE::::make sure reference is getting pushed and no cloning is happening
                }
            }
        }
    }
    pub fn checkCircularDependency(&self,  cell: &Cell) -> bool{
        isInCycle(cell)
    }
    pub fn set_cell_value( &mut self, cell: Cell, expression: &str,) -> Result<(), ExpressionError> {
        // Parse the expression
        let (new_function, success) = self.parse_expression(expression);
        if !success {
            return Err(ExpressionError::CouldNotParse);
        }

        let cell_data = self.get_cell_data_mut(cell).ok_or(ExpressionError::CouldNotParse)?;

        // Save old state
        let old_function = cell_data.function.clone();
        let old_value = cell_data.value;

        // Handle constant functions
        if new_function.is_constant() {
            let (new_value, error) = self.evaluate_expression(&new_function);
            cell_data.value = new_value;
            cell_data.error = error;
            cell_data.function = new_function;
            self.update_graph(cell, &old_function);
            self.update_dependents(cell, old_value);
            return Ok(());
        }

        // Update function and dependencies
        cell_data.function = new_function.clone();
        self.update_graph(cell, &old_function);

        // Check for circular dependencies
        if self.check_circular_dependency(cell) {
            // Revert changes
            cell_data.function = old_function.clone();
            self.update_graph(cell, &new_function);
            return Err(ExpressionError::CircularDependency);
        }

        // Evaluate new value
        let (new_value, error) = self.evaluate_expression(&cell_data.function);
        cell_data.value = if error == CellError::NoError {
            new_value
        } else {
            0
        };
        cell_data.error = error;

        // Update dependents
        self.update_dependents(cell, old_value);

        Ok(())
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
