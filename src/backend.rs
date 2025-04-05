use crate::cell::Cell;


#[derive(Debug, PartialEq)]
pub enum ExpressionError {
    CouldNotParse,
    CircularDependency,
}


#[derive(Debug, Clone, PartialEq)]
pub enum CellError {
    NoError,
    CircularDependency,
    ParseError,
    DivisionByZero,
    InvalidCell,
    EvaluationError(String),
}

#[derive(Debug, Clone)]
pub enum Function {
    RangeFunction { 
        top_left: Cell,
        bottom_right: Cell 
    },
    BinaryOp { 
        first: Operand,
        second: Operand 
    },
    SleepFunction { 
        operand: Operand 
    },
    Constant(i32),
}


#[derive(Debug, Clone)]
pub enum Operand {
    Cell(Cell),
    Int(i32),
}

#[derive(Debug)]
pub struct CellData {
    pub function: Function,
    pub error: CellError,
    pub dependents: Vec<Cell>,
    pub value: i32,
    pub dirty_parents: u32,
}

pub struct Backend {
    grid: Vec<Vec<CellData>>,
    rows: u32,
    cols: u32,
}

impl Backend {
    pub fn new(rows: u32, cols: u32) -> Self {  //init backend
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
            .map(|data| (data.value, data.error.clone())) ////NOTE:::: change this to reference, remove clone
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
        if let Some(cell_data) = self.get_cell_value(start) {
            cell_data.dirty_parents = 1;
        }

        while let Some(current) = stack.pop() {
            // Get all dependents of current cell
            
            let dependents = self.get_cell_value(current).dependents;

            for dep in dependents {
                // Cycle detected if we return to start cell
                if dep == start {
                    found_cycle = true;
                    break;
                }

                // Only process unvisited cells
                if let Some(dep_data) = self.get_cell_value(dep) {
                    if dep_data.dirty_parents == 0 {
                        dep_data.dirty_parents = 1;
                        stack.push(dep);
                    }
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
        if let Some(cell_data) = self.get_cell_value(start) {
            cell_data.dirty_parents = 0;
        }

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
        self.is_in_cycle(cell)
    }
 /*   pub fn set_cell_value( &mut self, cell: Cell, expression: &str,) -> Result<(), ExpressionError> {
        // Parse the expression
        let (new_function, success) = self.parse_expression(expression);
        if !success {
            return Err(ExpressionError::CouldNotParse);
        }

        let cell_data = self.get_cell_value(cell).ok_or(ExpressionError::CouldNotParse)?;

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
  */ 
  pub fn update_graph(&mut self, cell: Cell, old_function: &Function) {
    let cell_row = cell.row as usize;
    let cell_col = cell.col as usize;
    
    let cell_data = &mut self.grid[cell_row][cell_col];

    // Remove old dependencies
    match old_function {
        Function::RangeFunction { top_left, bottom_right } => {
            for row in top_left.row..=bottom_right.row {
                for col in top_left.col..=bottom_right.col {
                    if let Some(parent) = self.get_cell_value(row as usize, col as usize) {
                        parent.dependents.retain(|&c| c != cell);
                    }
                }
            }
        }
        Function::BinaryOp { first, second } => {
            if let Operand::Cell(dep) = first {
                if let Some(parent) = self.get_cell_value(dep.row as usize, dep.col as usize) {
                    parent.dependents.retain(|&c| c != cell);
                }
            }
            if let Operand::Cell(dep) = second {
                if let Some(parent) = self.get_cell_value(dep.row as usize, dep.col as usize) {
                    parent.dependents.retain(|&c| c != cell);
                }
            }
        }
        Function::SleepFunction { operand } => {
            if let Operand::Cell(dep) = operand {
                if let Some(parent) = self.get_cell_value(dep.row as usize, dep.col as usize) {
                    parent.dependents.retain(|&c| c != cell);
                }
            }
        }
        _ => {}
    }

    // Add new dependencies
    match &cell_data.function {
        Function::BinaryOp { first, second } => {
            if let Operand::Cell(dep) = first {
                if let Some(parent) = self.get_cell_value(dep.row as usize, dep.col as usize) {
                    parent.dependents.push(cell);
                }
            }
            if let Operand::Cell(dep) = second {
                if let Some(parent) = self.get_cell_value(dep.row as usize, dep.col as usize) {
                    parent.dependents.push(cell);
                }
            }
        }
        Function::RangeFunction { top_left, bottom_right } => {
            for row in top_left.row..=bottom_right.row {
                for col in top_left.col..=bottom_right.col {
                    if let Some(parent) = self.get_cell_value(row as usize, col as usize) {
                        parent.dependents.push(cell);
                    }
                }
            }
        }
        Function::SleepFunction { operand } => {
            if let Operand::Cell(dep) = operand {
                if let Some(parent) = self.get_cell_value(dep.row as usize, dep.col as usize) {
                    parent.dependents.push(cell);
                }
            }
        }
        _ => {}
    }
}


   

/// Sets dirty parent counts for topological sorting
pub fn set_dirty_parents(&mut self, cell: Cell, stack: &mut Vec<Cell>) {
    if let Some(root_data) = self.get_cell_value(cell) {
        root_data.dirty_parents = 0;
    }
    stack.push(cell);

    while let Some(current) = stack.pop() {
        let dependents = self.get_cell_value(current)
            .map(|c| c.dependents.clone())
            .unwrap_or_default();

        for child in dependents {
            if let Some(child_data) = self.get_cell_value(child) {
                if child_data.dirty_parents == 0 {
                    stack.push(child);
                }
                child_data.dirty_parents += 1;
            }
        }
    }
}

/// Recursively update dependent cells using topological sort
pub fn update_dependants(&mut self, cell: Cell) {
    let mut stack = Vec::new();
    self.set_dirty_parents(cell, &mut stack);

    let mut process_stack = Vec::new();
    if let Some(cell_data) = self.get_cell_value(cell) {
        for &child in &cell_data.dependents {
            if let Some(child_data) = self.get_cell_value(child) {
                child_data.dirty_parents -= 1;
                if child_data.dirty_parents == 0 {
                    process_stack.push(child);
                }
            }
        }
    }

    while let Some(current) = process_stack.pop() {
        if let Some(current_data) = self.get_cell_value(current) {
            let (new_value, error) = self.evaluate_expression(&current_data.function);
            current_data.value = new_value;
            current_data.error = error;

            for &dependent in &current_data.dependents {
                if let Some(dep_data) = self.get_cell_value(dependent) {
                    dep_data.dirty_parents -= 1;
                    if dep_data.dirty_parents == 0 {
                        process_stack.push(dependent);
                    }
                }
            }
        }
    }
}

   

        /// Checks if this function can be safely replaced with a constant value
    // pub fn is_expression_constant(&self) -> bool {
    //     match self {
    //         Function::PlusOp(bin_op) |
    //         Function::MinusOp(bin_op) |
    //         Function::MultiplyOp(bin_op) |
    //         Function::DivideOp(bin_op) => {
    //             // Both operands must be literal integers
    //             matches!(&bin_op.first, Operand::Int(_)) &&
    //             matches!(&bin_op.second, Operand::Int(_))
    //         }
    //         Function::Constant(_) => true,
    //         _ => false
    //     }
    // }

    /// Creates a constant value function
    pub fn constant_function(value: i32) -> Function {
        Function::Constant(value)
    }
   */
    
}

impl Default for CellData {
    fn default() -> Self {
        CellData {
            function: Function::Constant(0),
            error: CellError::NoError,
            dependents: Vec::new(),
            value: 0,
            dirty_parents: 0,
        }
    }
}
