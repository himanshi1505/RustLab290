//! # Spreadsheet Backend Module
//!
//! This module provides the core functionality for a spreadsheet application,
//! including cell management, formula evaluation, and dependency tracking.
use crate::structs::*;
use std::cell::UnsafeCell;
use std::cmp::{max, min};
use std::f64;
use std::thread;
use std::time::Duration;

#[cfg(feature = "gui")]
use std::collections::VecDeque;

#[cfg(feature = "gui")]
use std::fs::File;
#[cfg(feature = "gui")]
use std::io::BufWriter;

#[cfg(feature = "gui")]
use csv::{ReaderBuilder, WriterBuilder};
/// The main backend structure for the spreadsheet application.
///
/// Contains the grid of cells and manages all spreadsheet operations.
#[derive(Debug)]
pub struct Backend {
    /// The grid of cells stored in an UnsafeCell for interior mutability
    grid: UnsafeCell<Vec<Vec<CellData>>>,
    /// Number of rows in the spreadsheet
    rows: usize,
    /// Number of columns in the spreadsheet
    cols: usize,

    #[cfg(feature = "gui")]
    /// String representations of formulas for display
    pub formula_strings: Vec<Vec<String>>,

    #[cfg(feature = "gui")]
    /// Clipboard storage for copy/paste operations
    pub copy_stack: Vec<Vec<i32>>,
    #[cfg(feature = "gui")]
    /// Undo stack for storing previous states of the spreadsheet
    undo_stack: VecDeque<Vec<Vec<(CellData, String)>>>,
    #[cfg(feature = "gui")]
    /// Redo stack for storing states that can be redone
    redo_stack: VecDeque<Vec<Vec<(CellData, String)>>>,
}
#[cfg(feature = "gui")]
type CellDependencies = (Vec<(usize, usize)>, Vec<(usize, usize)>);
impl Backend {
    #[cfg(feature = "gui")]
    /// Gets the dependencies of a cell (parents and children in the dependency graph)
    pub fn get_cell_dependencies(&self, row: usize, col: usize) -> CellDependencies {
        let mut parents = Vec::new();
        let mut children = Vec::new();

        unsafe {
            let cell_data = self.get_cell_value(row, col);

            // Collect children (dependents)
            for &(child_row, child_col) in &(*cell_data).dependents {
                children.push((child_row as usize, child_col as usize));
            }

            // Collect parents (cells this cell depends on)
            match &(*cell_data).function.data {
                FunctionData::RangeFunction(range) => {
                    for r in range.top_left.row..=range.bottom_right.row {
                        for c in range.top_left.col..=range.bottom_right.col {
                            parents.push((r, c));
                        }
                    }
                }
                FunctionData::BinaryOp(bin_op) => {
                    if let OperandData::Cell(dep) = bin_op.first.data {
                        parents.push((dep.row, dep.col));
                    }
                    if let OperandData::Cell(dep) = bin_op.second.data {
                        parents.push((dep.row, dep.col));
                    }
                }
                FunctionData::SleepValue(operand) => {
                    if let OperandData::Cell(dep) = operand.data {
                        parents.push((dep.row, dep.col));
                    }
                }
                FunctionData::Value(_) => {} // No parents for constant values
            }
        }

        (parents, children)
    }
    /// Gets the number of rows and columns in the spreadsheet
    pub fn get_rows_col(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }
    /// Creates a new spreadsheet backend with the specified dimensions.
    ///
    /// Initializes all cells with:
    /// - Value of 0
    /// - No dependencies
    /// - Constant function type
    /// - No errors
    ///
    /// # Arguments
    /// * `rows` - Number of rows in the spreadsheet
    /// * `cols` - Number of columns in the spreadsheet
    ///
    /// # Example
    ///
    /// ```rust
    /// use spreadsheet_backend::Backend;
    ///
    /// // Create a 5x5 spreadsheet
    /// let backend = Backend::new(5, 5);
    /// assert_eq!(backend.get_rows(), 5);
    /// assert_eq!(backend.get_cols(), 5);
    /// ```
    pub fn new(rows: usize, cols: usize) -> Self {
        let mut grid = Vec::with_capacity(rows);
        for _row in 0..rows {
            let mut row_vec = Vec::with_capacity(cols);
            for _col in 0..cols {
                row_vec.push(CellData {
                    value: 0,
                    dependents: Vec::new(),
                    function: Function::new_constant(0),
                    error: CellError::NoError,
                    dirty_parents: 0,
                });
            }
            grid.push(row_vec);
        }

        Backend {
            grid: UnsafeCell::new(grid),
            #[cfg(feature = "gui")]
            undo_stack: VecDeque::with_capacity(100),
            #[cfg(feature = "gui")]
            redo_stack: VecDeque::with_capacity(100),
            rows,
            cols,
            #[cfg(feature = "gui")]
            formula_strings: vec![vec!["=0".to_string(); cols]; rows],

            #[cfg(feature = "gui")]
            copy_stack: vec![vec![0; 1]; 1],
        }
    }

    /// Gets a mutable pointer to a cell's data (unsafe)
    pub unsafe fn get_cell_value(&self, row: usize, col: usize) -> *mut CellData {
        let grid_ptr = (*self.grid.get())[row].as_mut_ptr();
        grid_ptr.add(col)
    }
    /// Resets the `dirty_parents` flag for a starting cell and all its dependent cells.
    ///
    /// This function performs a depth-first traversal of the dependency graph starting from
    /// the given cell, resetting the `dirty_parents` flag to 0 for all reachable cells.
    /// This is typically used after dependency checking to clean up the dirty flags.
    pub fn reset_found(&mut self, start: &Cell) {
        unsafe {
            let start_cell = self.get_cell_value(start.row, start.col);
            (*start_cell).dirty_parents = 0;
            let mut stack = vec![start_cell];

            while let Some(current) = stack.pop() {
                let deps = &(*current).dependents; // Access the dependents vector
                for &(row, col) in deps.iter() {
                    let dep = self.get_cell_value(row as usize, col as usize); // Access the dependent cell

                    if (*dep).dirty_parents > 0 {
                        (*dep).dirty_parents = 0;
                        stack.push(dep);
                    }
                }
            }
        }
    }

    /// Checks for circular dependencies starting from a given cell using DFS.
    ///
    /// Temporarily marks cells during traversal and cleans up after.
    ///
    /// # Arguments
    /// * `start` - The cell to start checking from
    ///
    /// # Returns
    /// `true` if a circular dependency is found, `false` otherwise
    ///
    /// # Example
    ///
    /// ```rust
    /// use spreadsheet_backend::{Backend, Cell};
    ///
    /// let mut backend = Backend::new(3, 3);
    /// let a1 = Cell { row: 0, col: 0 };
    /// let a2 = Cell { row: 1, col: 0 };
    ///
    /// backend.set_cell_value(a1, "=A2").unwrap();
    /// backend.set_cell_value(a2, "=A1").unwrap();
    ///
    /// assert!(backend.check_circular_dependency(&a1));
    /// ```
    pub fn check_circular_dependency(&mut self, start: &Cell) -> bool {
        let mut found_cycle = false;

        unsafe {
            let start_cell = self.get_cell_value(start.row, start.col);
            let start_cell_ptr = start_cell as *const CellData;
            let mut stack = vec![start_cell_ptr];
            (*start_cell).dirty_parents = 1;

            while let Some(current_ptr) = stack.pop() {
                let current = &*current_ptr;
                let deps = &current.dependents;

                // First pass: check for cycles and collect new deps to process
                let mut deps_to_check = Vec::new();
                for &dep_ptr in deps.iter() {
                    if dep_ptr.0 == start.row as i32 && dep_ptr.1 == start.col as i32 {
                        found_cycle = true;
                        break;
                    }

                    deps_to_check.push(dep_ptr);
                }

                if found_cycle {
                    break;
                }

                // Second pass: push unvisited deps
                for dep_ptr in &deps_to_check {
                    let dep = self.get_cell_value(dep_ptr.0 as usize, dep_ptr.1 as usize);
                    if (*dep).dirty_parents == 0 {
                        (*dep).dirty_parents = 1;
                        stack.push(dep);
                    }
                }
            }
        }

        self.reset_found(start);
        found_cycle
    }
    /// Updates the dependency graph when a cell's formula changes by removing old dependencies and adding new ones using the new formula
    /// This function:
    /// 1. Removes old dependencies from the graph
    /// 2. Adds new dependencies based on the current formula
    /// 3. Maintains consistency in the dependency graph
    ///
    /// # Arguments
    /// * `cell` - The cell whose formula changed
    /// * `old_function` - The previous function/formula of the cell
    ///
    pub fn update_graph(&mut self, cell: &Cell, old_function: &Function) {
        unsafe {
            // Remove old dependencies
            let cell_data = self.get_cell_value(cell.row, cell.col);

            match &old_function.data {
                FunctionData::RangeFunction(range) => {
                    for row in range.top_left.row..=range.bottom_right.row {
                        for col in range.top_left.col..=range.bottom_right.col {
                            let parent_data = self.get_cell_value(row, col);
                            let deps = &mut (*parent_data).dependents;
                            deps.retain(|&(r, c)| !(r == cell.row as i32 && c == cell.col as i32));
                        }
                    }
                }

                FunctionData::BinaryOp(bin_op) => {
                    if let OperandData::Cell(dep) = bin_op.first.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut (*parent_data).dependents;
                        deps.retain(|&(r, c)| !(r == cell.row as i32 && c == cell.col as i32));
                    }
                    if let OperandData::Cell(dep) = bin_op.second.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut (*parent_data).dependents;
                        deps.retain(|&(r, c)| !(r == cell.row as i32 && c == cell.col as i32));
                    }
                }

                FunctionData::SleepValue(operand) => {
                    if let OperandData::Cell(dep) = operand.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut (*parent_data).dependents;
                        deps.retain(|&(r, c)| !(r == cell.row as i32 && c == cell.col as i32));
                    }
                }

                FunctionData::Value(_) => {} // No dependencies to remove
            }

            // Add new dependencies
            match &(*cell_data).function.data {
                FunctionData::RangeFunction(range) => {
                    for row in range.top_left.row..=range.bottom_right.row {
                        for col in range.top_left.col..=range.bottom_right.col {
                            let parent_data = self.get_cell_value(row, col);
                            let deps = &mut (*parent_data).dependents;
                            deps.push((cell.row as i32, cell.col as i32));
                        }
                    }
                }

                FunctionData::BinaryOp(bin_op) => {
                    if let OperandData::Cell(dep) = bin_op.first.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut (*parent_data).dependents;
                        deps.push((cell.row as i32, cell.col as i32));
                    }
                    if let OperandData::Cell(dep) = bin_op.second.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut (*parent_data).dependents;
                        deps.push((cell.row as i32, cell.col as i32));
                    }
                }

                FunctionData::SleepValue(operand) => {
                    if let OperandData::Cell(dep) = operand.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut (*parent_data).dependents;
                        deps.push((cell.row as i32, cell.col as i32));
                    }
                }

                FunctionData::Value(_) => {} // No dependencies to add
            }
        }
    }

    /// Sets dirty parent counts for topological sorting
    /// This function is used to mark cells that need to be updated
    pub fn set_dirty_parents(&mut self, cell: &Cell, stack: &mut Vec<*mut CellData>) {
        unsafe {
            let root_data = self.get_cell_value(cell.row, cell.col);
            let root_ptr = root_data;

            (*root_ptr).dirty_parents = 0;
            stack.push(root_ptr);

            while let Some(current_ptr) = stack.pop() {
                let current = &*current_ptr;
                let deps = &current.dependents; // Access the dependents vector

                for &(row, col) in deps.iter() {
                    let child_data = self.get_cell_value(row as usize, col as usize);
                    let child_ptr = child_data;

                    if (*child_ptr).dirty_parents == 0 {
                        stack.push(child_ptr);
                    }
                    (*child_ptr).dirty_parents += 1;
                }
            }
        }
    }

    /// Recursively update dependent cells using topological sort
    /// This function is called when a cell's value changes
    /// It updates the values of all cells that depend on the changed cell
    pub fn update_dependents(&mut self, cell: &Cell) {
        let mut dirty_stack = Vec::new();
        self.set_dirty_parents(cell, &mut dirty_stack);

        let mut process_stack = Vec::new();

        unsafe {
            let cell_data = self.get_cell_value(cell.row, cell.col);

            // Process the dependents of the initial cell
            for &(row, col) in (*cell_data).dependents.iter() {
                let child_data = self.get_cell_value(row as usize, col as usize);
                (*child_data).dirty_parents -= 1;
                if (*child_data).dirty_parents == 0 {
                    process_stack.push((row as usize, col as usize));
                }
            }

            // Process the stack of dependent cells
            while let Some((row, col)) = process_stack.pop() {
                let current_data = self.get_cell_value(row, col);
                let (new_value, error) = self.evaluate_expression(&(*current_data).function);
                (*current_data).value = new_value;
                (*current_data).error = error;

                for &(dep_row, dep_col) in (*current_data).dependents.iter() {
                    let dependent_data = self.get_cell_value(dep_row as usize, dep_col as usize);
                    (*dependent_data).dirty_parents -= 1;
                    if (*dependent_data).dirty_parents == 0 {
                        process_stack.push((dep_row as usize, dep_col as usize));
                    }
                }
            }
        }
    }

    /// Evaluates a function and returns (value, error)
    /// This function is used to evaluate the result of a formula
    /// It handles different types of functions (binary operations, range functions, etc.)
    pub fn evaluate_expression(&self, func: &Function) -> (i32, CellError) {
        match func.data {
            FunctionData::BinaryOp(bin_op) => match func.type_ {
                FunctionType::Plus => match self.plus_op(&bin_op) {
                    Ok(value) => (value, CellError::NoError),
                    Err(error) => (0, error),
                },
                FunctionType::Minus => match self.minus_op(&bin_op) {
                    Ok(value) => (value, CellError::NoError),
                    Err(error) => (0, error),
                },
                FunctionType::Multiply => match self.multiply_op(&bin_op) {
                    Ok(value) => (value, CellError::NoError),
                    Err(error) => (0, error),
                },
                FunctionType::Divide => match self.divide_op(&bin_op) {
                    Ok(value) => (value, CellError::NoError),
                    Err(error) => (0, error),
                },
                _ => (0, CellError::DependencyError),
            },
            FunctionData::RangeFunction(range) => match func.type_ {
                FunctionType::Min => match self.min_function(&range) {
                    Ok(value) => (value, CellError::NoError),
                    Err(error) => (0, error),
                },
                FunctionType::Max => match self.max_function(&range) {
                    Ok(value) => (value, CellError::NoError),
                    Err(error) => (0, error),
                },
                FunctionType::Avg => match self.avg_function(&range) {
                    Ok(value) => (value, CellError::NoError),
                    Err(error) => (0, error),
                },
                FunctionType::Sum => match self.sum_function(&range) {
                    Ok(value) => (value, CellError::NoError),
                    Err(error) => (0, error),
                },
                FunctionType::Stdev => match self.stdev_function(&range) {
                    Ok(value) => (value, CellError::NoError),
                    Err(error) => (0, error),
                },
                _ => (0, CellError::DependencyError),
            },
            FunctionData::SleepValue(operand) => match self.sleep_function(&operand) {
                Ok(value) => (value, CellError::NoError),
                Err(error) => (0, error),
            },
            FunctionData::Value(value) => (value, CellError::NoError),
        }
    }
    /// Sets a cell's value based on the provided expression
    /// Handles:
    /// - Constant values ("42")
    /// - Formulas ("=A1+B2")
    /// - Range functions ("=SUM(A1:B2)")
    /// - Automatic dependency graph updates
    /// - Circular dependency detection
    ///
    /// # Arguments
    /// * `cell` - Target cell location
    /// * `expression` - String expression to parse and evaluate
    ///
    /// # Returns
    /// `Result<(), ExpressionError>` indicating success or failure
    ///
    /// # Errors
    /// - `ExpressionError::CouldNotParse` for invalid expressions
    /// - `ExpressionError::CircularDependency` for circular references
    ///
    /// # Example
    ///
    /// ```rust
    /// use spreadsheet_backend::{Backend, Cell};
    ///
    /// let mut backend = Backend::new(3, 3);
    ///
    /// // Set constant value
    /// backend.set_cell_value(Cell { row: 0, col: 0 }, "10").unwrap();
    ///
    /// // Set formula referencing another cell
    /// backend.set_cell_value(Cell { row: 1, col: 0 }, "=A1 * 2").unwrap();
    ///
    /// // This would create a circular dependency and fail
    /// backend.set_cell_value(Cell { row: 0, col: 0 }, "=A2").unwrap_err();
    /// ```
    pub fn set_cell_value(&mut self, cell: Cell, expression: &str) -> Result<(), ExpressionError> {
        // Parse the expression
        let (new_function, success) = self.parse_expression(expression);
        if !success {
            return Err(ExpressionError::CouldNotParse);
        }

        // Get a mutable reference to the target cell
        unsafe {
            let cell_data = self.get_cell_value(cell.row, cell.col);

            let cell_ptr = cell_data;

            // Copy old state
            let old_function = (*cell_ptr).function;
            //let old_value = (*cell_ptr).value;

            // Handle constant function early
            if new_function.type_ == FunctionType::Constant {
                let (new_value, error) = self.evaluate_expression(&new_function);
                (*cell_ptr).value = new_value;
                (*cell_ptr).error = error;
                (*cell_ptr).function = new_function;

                self.update_graph(&cell, &old_function);
                self.update_dependents(&cell);

                #[cfg(feature = "gui")]
                {
                    self.formula_strings[cell.row][cell.col] = "=".to_owned() + expression;
                }
                return Ok(());
            }

            // Detect self-reference in new function
            match &new_function.data {
                FunctionData::BinaryOp(bin_op) => {
                    if bin_op.first.data == OperandData::Cell(cell)
                        || bin_op.second.data == OperandData::Cell(cell)
                    {
                        return Err(ExpressionError::CircularDependency);
                    }
                }
                FunctionData::RangeFunction(range) => {
                    for row in range.top_left.row..=range.bottom_right.row {
                        for col in range.top_left.col..=range.bottom_right.col {
                            if row == cell.row && col == cell.col {
                                return Err(ExpressionError::CircularDependency);
                            }
                        }
                    }
                }
                FunctionData::SleepValue(operand) => {
                    if operand.data == OperandData::Cell(cell) {
                        return Err(ExpressionError::CircularDependency);
                    }
                }
                FunctionData::Value(_) => {}
            }

            // Set new function
            (*cell_ptr).function = new_function;

            // Update graph (remove old edges)
            self.update_graph(&cell, &old_function);

            // Check circular dependency
            if self.check_circular_dependency(&cell) {
                // Revert function
                (*cell_ptr).function = old_function;
                self.update_graph(&cell, &new_function); // Reconnect old edges
                return Err(ExpressionError::CircularDependency);
            }

            // Evaluate and update value
            let (new_value, error) = self.evaluate_expression(&new_function);
            (*cell_ptr).value = if error == CellError::NoError {
                new_value
            } else {
                0
            };
            (*cell_ptr).error = error;

            // Propagate to dependents
            self.update_dependents(&cell);
        }
        #[cfg(feature = "gui")]
        {
            self.formula_strings[cell.row][cell.col] = expression.to_string();
        }

        Ok(())
    }
    /// In Range Functions  usage is CellName= FunctionName(TopLeftCell:BottomRightCell)
    ///Evaluates the minimum of the range
    /// This function calculates the minimum of the values in a given range of cells.
    /// # Usage: A1=MIN(A2:B3)
    pub fn min_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut min_val = i32::MAX;
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                unsafe {
                    let cell_data = self.get_cell_value(row, col);

                    match (*cell_data).error {
                        CellError::NoError => {
                            min_val = min(min_val, (*cell_data).value);
                        }
                        CellError::DivideByZero => return Err(CellError::DivideByZero),
                        CellError::DependencyError => return Err(CellError::DependencyError),
                        CellError::Overflow => return Err(CellError::Overflow),
                    }
                }
            }
        }
        Ok(min_val)
    }
    ///Evaluates the maximum of the range
    /// This function calculates the maximum of the values in a given range of cells.
    /// # Usage: A1=MAX(A2:B3)
    pub fn max_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut max_val = i32::MIN;
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                unsafe {
                    let cell_data = self.get_cell_value(row, col);
                    match (*cell_data).error {
                        CellError::NoError => {
                            max_val = max(max_val, (*cell_data).value);
                        }
                        CellError::DivideByZero => return Err(CellError::DivideByZero),
                        CellError::DependencyError => return Err(CellError::DependencyError),
                        CellError::Overflow => return Err(CellError::Overflow),
                    }
                }
            }
        }
        Ok(max_val)
    }
    ///Evaluates the average of the range
    /// This function calculates the average of the values in a given range of cells by summing them up and dividing by the count of valid cells.
    /// # Usage: A1=AVG(A2:B3)
    pub fn avg_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut sum = 0;
        let mut count = 0;
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                unsafe {
                    let cell_data = self.get_cell_value(row, col);

                    match (*cell_data).error {
                        CellError::NoError => {
                            sum += (*cell_data).value;
                            count += 1;
                        }
                        CellError::DivideByZero => return Err(CellError::DivideByZero),
                        CellError::DependencyError => return Err(CellError::DependencyError),
                        CellError::Overflow => return Err(CellError::Overflow),
                    }
                }
            }
        }
        if count == 0 {
            return Err(CellError::DivideByZero);
        }
        Ok(sum / count)
    }
    ///Evaluates the sum of the range
    /// This function calculates the sum of the values in a given range of cells.
    /// # Usage: A1=SUM(A2:B3)
    pub fn sum_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut sum = 0;
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                unsafe {
                    let cell_data = self.get_cell_value(row, col);

                    match (*cell_data).error {
                        CellError::NoError => {
                            sum += (*cell_data).value;
                        }
                        CellError::DivideByZero => return Err(CellError::DivideByZero),
                        CellError::DependencyError => return Err(CellError::DependencyError),
                        CellError::Overflow => return Err(CellError::Overflow),
                    }
                }
            }
        }
        Ok(sum)
    }
    ///Evaluates the standard deviation of the range
    /// This function calculates the standard deviation of the values in a given range of cells.
    /// # Usage: A1=STDEV(A2:B3)
    pub fn stdev_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut values = Vec::new();
        let mut sum = 0;
        let mut count = 0;

        // First pass: collect values and calculate sum
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                unsafe {
                    let cell_data = self.get_cell_value(row, col);

                    match (*cell_data).error {
                        CellError::NoError => {
                            let value = (*cell_data).value;
                            values.push(value);
                            sum += value;
                            count += 1;
                        }
                        CellError::DivideByZero => return Err(CellError::DivideByZero),
                        CellError::DependencyError => return Err(CellError::DependencyError),
                        CellError::Overflow => return Err(CellError::Overflow),
                    }
                }
            }
        }

        if count == 0 {
            return Err(CellError::DivideByZero);
        }

        // Calculate mean
        let mean = sum / count;

        // Second pass: calculate variance
        let mut variance_sum: f64 = 0.0;
        for value in values {
            variance_sum += ((value - mean) * (value - mean)) as f64;
        }

        let variance = variance_sum / count as f64;
        // println!("stdev: {:?}", (variance as f64).sqrt() as i32);
        // Return standard deviation as integer (floored)
        Ok(variance.sqrt().round() as i32)
    }
    /// Evaluates the sleep function
    /// This function is used to pause execution for a specified number of seconds
    /// # Usage: A1=SLEEP(4)
    /// or
    /// # Usage: A1=SLEEP(A2)
    pub fn sleep_function(&self, operand: &Operand) -> Result<i32, CellError> {
        let value = self.get_operand_value(operand)?;
        // println!("value: {:?}", value);
        if value > 0 {
            thread::sleep(Duration::from_secs(value as u64));
        }
        Ok(value)
    }
    ///In binary operations the usage is CellName=FunctionName(Operand1, Operand2)
    /// Evaluates addition operation
    /// This function is used to add two operands together
    /// # Usage: A1=A2+A3
    pub fn plus_op(&self, bin_op: &BinaryOp) -> Result<i32, CellError> {
        let first = self.get_operand_value(&bin_op.first)?;
        let second = self.get_operand_value(&bin_op.second)?;
        Ok(first + second)
    }
    /// Evaluates subtraction operation
    /// This function is used to subtract two operands
    /// Usage: A1=A2-A3
    pub fn minus_op(&self, bin_op: &BinaryOp) -> Result<i32, CellError> {
        let first = self.get_operand_value(&bin_op.first)?;
        let second = self.get_operand_value(&bin_op.second)?;
        Ok(first - second)
    }
    /// Evaluates multiplication operation
    /// This function is used to multiply two operands
    /// # Usage: A1=A2*A3
    pub fn multiply_op(&self, bin_op: &BinaryOp) -> Result<i32, CellError> {
        let first = self.get_operand_value(&bin_op.first)?;
        let second = self.get_operand_value(&bin_op.second)?;
        if first != 0
            && second != 0
            && (first.abs() > 2_147_483_647 / second.abs()
                || second.abs() > 2_147_483_647 / first.abs())
        {
            return Err(CellError::Overflow);
        }

        Ok(first * second)
    }
    /// Evaluates division operation
    /// This function is used to divide two operands
    /// # Usage: A1=A2/A3
    /// Division by zero is handled and gives ERR
    pub fn divide_op(&self, bin_op: &BinaryOp) -> Result<i32, CellError> {
        let first = self.get_operand_value(&bin_op.first)?;
        let second = self.get_operand_value(&bin_op.second)?;

        if second == 0 {
            return Err(CellError::DivideByZero);
        }

        Ok(first / second)
    }

    /// Gets the value of an operand (either a cell reference or literal value)
    fn get_operand_value(&self, operand: &Operand) -> Result<i32, CellError> {
        match operand.data {
            OperandData::Cell(cell) => {
                // Get the cell data
                unsafe {
                    let cell_data = self.get_cell_value(cell.row, cell.col);

                    // Check for errors in the cell
                    match (*cell_data).error {
                        CellError::NoError => Ok((*cell_data).value),
                        CellError::DivideByZero => Err(CellError::DivideByZero),
                        CellError::DependencyError => Err(CellError::DependencyError),
                        CellError::Overflow => Err(CellError::Overflow),
                    }
                }
            }
            OperandData::Value(value) => Ok(value),
        }
    }
    /// Parses a formula expression and returns the corresponding function
    pub fn parse_expression(&self, expression: &str) -> (Function, bool) {
        crate::parser::parse_expression(expression, self)
    }
    #[cfg(feature = "gui")]
    /// Parses a load or save command from a string
    pub fn parse_load_or_save_cmd(expression: &str) -> Option<String> {
        crate::parser::parse_load_or_save_cmd(expression)
    }
    #[cfg(feature = "gui")]
    /// Parses a cut or copy command from a string
    pub fn parse_cut_or_copy(
        &self,
        expression: &str,
    ) -> Result<(Cell, Cell), Box<dyn std::error::Error>> {
        crate::parser::parse_cut_or_copy(self, expression)
    }
    #[cfg(feature = "gui")]
    /// Parses a paste command from a string
    pub fn parse_paste(&self, expression: &str) -> Result<Cell, Box<dyn std::error::Error>> {
        crate::parser::parse_paste(self, expression)
    }
    #[cfg(feature = "gui")]
    /// Parses an autofill command from a string
    pub fn parse_autofill(
        &self,
        expression: &str,
    ) -> Result<(Cell, Cell, Cell), Box<dyn std::error::Error>> {
        crate::parser::parse_autofill(self, expression)
    }
    #[cfg(feature = "gui")]
    /// Parses a sort command from a string
    pub fn parse_sort(
        &self,
        expression: &str,
    ) -> Result<(Cell, Cell, bool), Box<dyn std::error::Error>> {
        crate::parser::parse_sort(self, expression)
    }
    /// Returns the number of rows in the spreadsheet
    pub fn get_rows(&self) -> usize {
        self.rows
    }
    /// Returns the number of columns in the spreadsheet
    pub fn get_cols(&self) -> usize {
        self.cols
    }

    #[cfg(feature = "gui")]
    /// Performs a sort operation on a range of cells
    /// Sorts the cells in ascending or descending order based on the specified column
    /// # Usage for sorting in ascending order is: sorta(TopLeftCell:BottomRightCell)
    /// # Usage for sorting in descending order is: sortd(TopLeftCell:BottomRightCell)
    pub fn sort(&mut self, expression: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tup = self.parse_sort(expression);
        let (tl_cell, br_cell, a_or_d) = match tup {
            Ok((tl, br, a_or_d)) => (tl, br, a_or_d),
            Err(err) => return Err(err),
        };
        let tl = (tl_cell.row, tl_cell.col);
        let br = (br_cell.row, br_cell.col);
        let grid_ref = unsafe { &mut *self.grid.get() };
        grid_ref[tl.0..=br.0].sort_by(|a, b| {
            let cmp_result = a[tl.1].value.cmp(&b[tl.1].value);
            if a_or_d {
                cmp_result // Ascending order
            } else {
                cmp_result.reverse() // Descending order
            }
        });
        Ok(())
    }
    #[cfg(feature = "gui")]
    /// Undoes the last action
    /// This function pops the last state from the undo stack and applies it to the spreadsheet
    /// It also pushes the current state to the redo stack
    /// # Usage: undo()
    ///  or
    /// # Usage: click on undo button and then click somewhere else on grid to see the changes
    pub fn undo_callback(&mut self) {
        if let Some(prev_state) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(self.create_snapshot());
            self.apply_snapshot(prev_state);
        }
    }

    #[cfg(feature = "gui")]
    /// Redoes last undone action
    /// This function pops the last state from the redo stack and applies it to the spreadsheet
    /// It also pushes the current state to the undo stack
    /// # Usage: redo()
    ///  or
    /// # Usage: click on redo button and then click somewhere else on grid to see the changes
    pub fn redo_callback(&mut self) {
        if let Some(next_state) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(self.create_snapshot());
            self.apply_snapshot(next_state);
        }
    }

    #[cfg(feature = "gui")]
    /// Creates a snapshot of the current state for undo/redo
    pub fn create_snapshot(&self) -> Vec<Vec<(CellData, String)>> {
        let mut snapshot = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let mut row_data = Vec::with_capacity(self.cols);
            for col in 0..self.cols {
                unsafe {
                    let cell_data = self.get_cell_value(row, col);
                    row_data.push(((*cell_data).clone(), self.formula_strings[row][col].clone()));
                }
            }
            snapshot.push(row_data);
        }
        snapshot
    }

    #[cfg(feature = "gui")]
    /// Applies a snapshot to restore state
    pub fn apply_snapshot(&mut self, snapshot: Vec<Vec<(CellData, String)>>) {
        for (row_idx, row) in snapshot.iter().enumerate() {
            for (col_idx, value) in row.iter().enumerate() {
                unsafe {
                    let cell_data = self.get_cell_value(row_idx, col_idx);
                    let cell_ptr = cell_data;
                    (*cell_ptr).value = value.0.value;
                    (*cell_ptr).error = value.0.error;
                    (*cell_ptr).dependents = value.0.dependents.clone();
                    (*cell_ptr).function = value.0.function;
                    (*cell_ptr).dirty_parents = value.0.dirty_parents;
                    self.formula_strings[row_idx][col_idx] = value.1.clone();
                }
            }
        }
    }

    #[cfg(feature = "gui")]
    /// Save current state to undo stack
    pub fn push_undo_state(&mut self) {
        if self.undo_stack.len() >= 100 {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(self.create_snapshot());
    }
    #[cfg(feature = "gui")]
    /// Autofill a range of cells based on a given expression
    /// Preference order is  - 1. constant, 2. GP, 3. AP
    /// # Usage: autofill(TopLeftCell:BottomRightCell, DestinationCell)
    /// It identifies the type of series (constant, GP, AP) in Range given and fills till the destination cells accordingly
    pub fn autofill(&mut self, expression: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("autofill: {:?}", expression);
        let tup = self.parse_autofill(expression);
        let (tl_cell, br_cell, dest_cell) = match tup {
            Ok((tl, br, dest)) => (tl, br, dest),
            Err(err) => return Err(err),
        };
        let tl = (tl_cell.row, tl_cell.col);
        let br = (br_cell.row, br_cell.col);
        let dest = (dest_cell.row, dest_cell.col);
        let v = unsafe { (*(self.get_cell_value(tl.0, tl.1))).value };
        let d = unsafe {
            (*(self.get_cell_value(tl.0, tl.1))).value
                - (*(self.get_cell_value(tl.0 + 1, tl.1))).value
        };
        let r = unsafe {
            ((*(self.get_cell_value(tl.0, tl.1))).value as f64)
                / ((*(self.get_cell_value(tl.0 + 1, tl.1))).value as f64)
        };
        println!("v: {:?}, d: {:?}, r: {:?}", v, d, r);
        println!(
            "tl_value: {:?}, br_value: {:?}",
            unsafe { (*(self.get_cell_value(tl.0, tl.1))).value },
            unsafe { (*(self.get_cell_value(tl.0 + 1, tl.1))).value }
        );
        let mut is_constant = true;
        let mut is_ap = true;
        let mut is_gp = true;
        let grid_ref = unsafe { &*self.grid.get() };
        println!("im hereee");
        for row in grid_ref.iter().take(br.0 + 1).skip(tl.0) {
            for col in &row[tl.1..=br.1] {
                if col.value != v {
                    is_constant = false;
                    break;
                }
            }
        }
        println!("is constant: {:?}", is_constant);
        if is_constant {
            println!("is constant");
            for row in br.0 + 1..=dest.0 {
                for col in br.1..=dest.1 {
                    let cell = Cell { row, col };
                    let res = self.set_cell_value(cell, v.to_string().as_str());
                    if let Err(err) = res {
                        println!("Error autofilling value: {:?}", err);
                    }
                }
            }
            Ok(())
        } else {
            for row in tl.0..br.0 {
                for col in tl.1..=br.1 {
                    if (grid_ref[row][col].value as f64) / (grid_ref[row + 1][col].value as f64)
                        != r
                    {
                        is_gp = false;
                        break;
                    }
                }
            }
            print!("is gp: {:?}", is_gp);
            if is_gp {
                println!("is gp");
                for row in br.0 + 1..=dest.0 {
                    for col in br.1..=dest.1 {
                        let cell = Cell { row, col };
                        let res = self.set_cell_value(
                            cell,
                            &((grid_ref[row - 1][col].value as f64 / r) as i32).to_string(),
                        );
                        if let Err(err) = res {
                            println!("Error autofilling value: {:?}", err);
                        }
                    }
                }
                Ok(())
            } else {
                for row in tl.0..br.0 {
                    for col in tl.1..=br.1 {
                        if grid_ref[row][col].value - grid_ref[row + 1][col].value != d {
                            is_ap = false;
                            break;
                        }
                    }
                }
                println!("is ap: {:?}", is_ap);
                if is_ap {
                    print!("is ap");
                    for row in br.0 + 1..=dest.0 {
                        for col in br.1..=dest.1 {
                            let cell = Cell { row, col };
                            let res = self.set_cell_value(
                                cell,
                                &(grid_ref[row - 1][col].value - d).to_string(),
                            );
                            if let Err(err) = res {
                                println!("Error autofilling value: {:?}", err);
                            }
                        }
                    }
                    Ok(())
                } else {
                    Err("Autofill not possible".to_string().into())
                }
            }
        }
    }

    #[cfg(feature = "gui")]
    /// Cuts a range of cells and copies their values to the clipboard(copy stack)
    /// # Usage: cut(TopLeftCell:BottomRightCell)
    /// It removes the values from the original cells and stores them in the copy stack
    /// It also sets the original cells to 0
    /// Graph gets updated accordingly
    pub fn cut(&mut self, expression: &str) -> Result<(), Box<dyn std::error::Error>> {
        // println!("cut: {:?}", expression);
        let tup = self.parse_cut_or_copy(expression);
        let (tl_cell, br_cell) = match tup {
            Ok((tl, br)) => (tl, br),
            Err(err) => return Err(err),
        };
        let tl = (tl_cell.row, tl_cell.col);
        let br = (br_cell.row, br_cell.col);
        let _ = Backend::copy(self, expression);
        for row in tl.0..=br.0 {
            for col in tl.1..=br.1 {
                // println!("im htregrseznrte");
                let cell = Cell { row, col };
                let res = self.set_cell_value(cell, "0");
                println!("formula_strings: {:?}", self.formula_strings[row][col]);
                // unsafe {(*self.grid.get().wrapping_add(row).wrapping_add(col)).value = 0;}
                // unsafe {let cell = self.get_cell_value(row, col);
                // cell.value = 0;}
                if let Err(err) = res {
                    println!("Error cutting value: {:?}", err);
                }
            }
        }
        Ok(())
    }
    #[cfg(feature = "gui")]
    /// Copies a range of cells and stores their values to the clipboard(copy stack)
    /// # Usage: copy(TopLeftCell:BottomRightCell)
    /// It copies the values from the original cells to the copy stack
    /// It does not remove the values from the original cells
    pub fn copy(&mut self, expression: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tup = self.parse_cut_or_copy(expression);
        let (tl_cell, br_cell) = match tup {
            Ok((tl, br)) => (tl, br),
            Err(err) => return Err(err),
        };
        let tl = (tl_cell.row, tl_cell.col);
        let br = (br_cell.row, br_cell.col);
        let mut copied_data = Vec::new();
        for row in tl.0..=br.0 {
            let mut row_data = Vec::new();
            for col in tl.1..=br.1 {
                row_data.push(unsafe { (*(self.get_cell_value(row, col))).value });
            }
            copied_data.push(row_data);
        }
        self.copy_stack = copied_data;
        Ok(())
    }
    #[cfg(feature = "gui")]
    /// Pastes the selected cells from the clipboard(copy stack) to a specified location
    /// # Usage: paste(TopLeftCell)
    /// It pastes the values from the copy stack to the specified location
    /// If enough space is not available, it does not paste
    /// It also updates the graph accordingly
    pub fn paste(&mut self, expression: &str) -> Result<(), Box<dyn std::error::Error>> {
        let celll = self.parse_paste(expression);
        let tl_cell = celll?;
        let tl = (tl_cell.row, tl_cell.col);
        // println!("tl: {:?}", tl);
        let br = (
            tl.0 + self.copy_stack.len() - 1,
            tl.1 + self.copy_stack[0].len() - 1,
        );
        // println!("br: {:?}", br);

        if br.0 >= self.rows || br.1 >= self.cols {
            return Err("Paste area exceeds grid size".to_string().into());
        }
        for row in tl.0..=br.0 {
            for col in tl.1..=br.1 {
                if row < self.rows && col < self.cols {
                    let cell = Cell { row, col };
                    // println!("row: {:?}, col: {:?}", row, col);
                    let _res = self
                        .set_cell_value(cell, &self.copy_stack[row - tl.0][col - tl.1].to_string());
                    //let _col_header =
                    self.formula_strings[row][col] =
                        self.copy_stack[row - tl.0][col - tl.1].to_string();
                }
            }
        }
        Ok(())
    }
    #[cfg(feature = "gui")]
    /// Saves the current state of the spreadsheet to a CSV file
    /// # Usage: click on save button
    pub fn save_to_csv(&self, save_cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
        let filename = match crate::backend::Backend::parse_load_or_save_cmd(save_cmd) {
            Some(path) => path,
            None => return Err("Invalid load command".to_string().into()),
        };
        let file = File::create(filename)?;
        let mut wtr = WriterBuilder::new().from_writer(BufWriter::new(file));
        //let grid_ref = self.formula_strings.clone();
        for row in 0..self.rows {
            let mut record = Vec::new();
            for col in 0..self.cols {
                unsafe { record.push((*(self.get_cell_value(row, col))).value.to_string()) };
                //FIX KARNA HAI ISKO
                // record.push(grid_ref[row][col].clone());
                // unsafe {
                //     record.push((*self.grid.get().wrapping_add(row).wrapping_add(col))[row][col].value.to_string());
                // }
                // println!("row: {}, col: {}, value: {}", row, col, grid_ref[row][col]);
            }
            wtr.write_record(&record)?;
        }
        wtr.flush()?;
        Ok(())
    }
    #[cfg(feature = "gui")]
    /// Loads a CSV file and populates the spreadsheet with its data
    /// # Usage: click on load button
    pub fn load_csv(
        &mut self,
        load_cmd: &str,
        is_header_present: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let csv_path = match crate::backend::Backend::parse_load_or_save_cmd(load_cmd) {
            Some(path) => path,
            None => return Err("Invalid load command".to_string().into()),
        };
        let reader_result = ReaderBuilder::new()
            .has_headers(is_header_present)
            .from_path(csv_path);
        let reader = match reader_result {
            Ok(reader) => reader,
            Err(err) => return Err(Box::new(err)),
        };

        let mut csv_data: Vec<Vec<String>> = Vec::new();

        for record in reader.into_records() {
            let record = match record {
                Ok(record) => record,
                Err(err) => {
                    return Err(Box::new(err));
                }
            };

            let row: Vec<String> = record
                .iter()
                .map(|field| field.trim().to_string())
                .collect();

            csv_data.push(row);
        }

        let no_of_rows = csv_data.len();
        let no_of_cols = csv_data.first().map_or(0, |row| row.len());
        *self = Backend::new(no_of_rows, no_of_cols);
        self.get_rows_col().0 = no_of_rows;
        self.get_rows_col().1 = no_of_cols;
        // println!("Rows: {}, Cols: {}", self.get_rows_col().0, self.get_rows_col().1);

        for (row_idx, row) in csv_data.iter().enumerate() {
            for (col_idx, field) in row.iter().enumerate() {
                if row_idx < self.rows && col_idx < self.cols {
                    let cell = Cell {
                        row: row_idx,
                        col: col_idx,
                    };
                    let res = self.set_cell_value(cell, field);
                    if let Err(_err) = res {
                        return Err("Invalid cell value".to_string().into());
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "gui")]
    /// Loads a CSV string and populates the spreadsheet with its data
    pub fn load_csv_from_str(&mut self, data: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(data.as_bytes());

        let mut csv_data: Vec<Vec<String>> = Vec::new();

        for record in rdr.records() {
            let record = record?;
            let row: Vec<String> = record
                .iter()
                .map(|field| field.trim().to_string())
                .collect();
            csv_data.push(row);
        }

        let no_of_rows = csv_data.len();
        let no_of_cols = csv_data.first().map_or(0, |row| row.len());

        // Resize the backend to match CSV dimensions
        *self = Backend::new(no_of_rows, no_of_cols);

        // Load data into cells
        for (row_idx, row) in csv_data.iter().enumerate() {
            for (col_idx, field) in row.iter().enumerate() {
                if row_idx < self.rows && col_idx < self.cols {
                    let cell = Cell {
                        row: row_idx,
                        col: col_idx,
                    };
                    let _ = self.set_cell_value(cell, field);
                }
            }
        }

        Ok(())
    }
}
#[cfg(feature = "cli")]
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    //use crate::structs::*;
    //use crate::structs::FunctionType::Sleep;

    #[test]
    fn test_sleep_function_positive_value() {
        let backend = Backend::new(3, 3);

        // Set up an operand with a positive value
        let operand = Operand {
            type_: OperandType::Int,
            data: OperandData::Value(2), // Sleep for 2 seconds
        };

        let start_time = Instant::now();
        let result = backend.sleep_function(&operand);
        let elapsed_time = start_time.elapsed();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
        assert!(elapsed_time.as_secs() >= 2); // Ensure at least 2 seconds have passed
    }

    #[test]
    fn test_sleep_function_zero_value() {
        let backend = Backend::new(3, 3);

        // Set up an operand with a value of 0
        let operand = Operand {
            type_: OperandType::Int,
            data: OperandData::Value(0), // No sleep
        };

        let start_time = Instant::now();
        let result = backend.sleep_function(&operand);
        let elapsed_time = start_time.elapsed();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert!(elapsed_time.as_secs() < 1); // Ensure no significant delay
    }

    #[test]
    fn test_sleep_function_negative_value() {
        let backend = Backend::new(3, 3);

        // Set up an operand with a negative value
        let operand = Operand {
            type_: OperandType::Int,
            data: OperandData::Value(-5), // Negative value
        };

        let result = backend.sleep_function(&operand);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), -5); // Negative values should not cause sleep
    }

    #[test]
    fn test_new_backend() {
        let backend = Backend::new(5, 5);
        assert_eq!(backend.get_rows_col(), (5, 5));

        let backend = Backend::new(0, 0);
        assert_eq!(backend.get_rows_col(), (0, 0));

        let backend = Backend::new(100, 100);
        assert_eq!(backend.get_rows_col(), (100, 100));
    }

    #[test]
    fn test_get_rows_col() {
        let backend = Backend::new(3, 4);
        assert_eq!(backend.get_rows_col(), (3, 4));

        let backend = Backend::new(1, 1);
        assert_eq!(backend.get_rows_col(), (1, 1));
    }

    #[test]
    fn test_set_and_get_cell_value() {
        let mut backend = Backend::new(3, 3);
        let cell = Cell { row: 1, col: 1 };
        let expression = "42";
        backend.set_cell_value(cell, expression).unwrap();

        unsafe {
            let cell_data = backend.get_cell_value(1, 1);
            assert_eq!((*cell_data).value, 42);
        }
    }

    #[test]
    fn test_set_cell_value_constant() {
        // Lines 148, 150-152, 154-157
        let mut backend = Backend::new(3, 3);
        let cell = Cell { row: 1, col: 1 };
        backend.set_cell_value(cell, "42").unwrap();

        unsafe {
            let cell_data = backend.get_cell_value(1, 1);
            assert_eq!((*cell_data).value, 42);
            assert_eq!((*cell_data).error, CellError::NoError);
        }
    }

    #[test]
    fn test_set_cell_value_circular_dependency() {
        // Lines 159-161, 167-168
        let mut backend = Backend::new(3, 3);
        let cell = Cell { row: 0, col: 0 };
        // backend.set_cell_value(cell, "=A1").unwrap();

        let result = backend.set_cell_value(cell, "A1");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExpressionError::CircularDependency);
    }
    #[test]
    fn test_update_graph_remove_dependencies() {
        // Lines 171-174, 176-178
        let mut backend = Backend::new(3, 3);
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "B1").unwrap();

        let old_function = Function::new_constant(5);
        backend.update_graph(&cell, &old_function);

        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            assert_eq!((*cell_data).value, 0); // Old dependencies removed
        }
    }

    #[test]
    fn test_update_graph_add_dependencies() {
        // Lines 181-182, 185-186
        let mut backend = Backend::new(3, 3);
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "B1").unwrap();

        unsafe {
            let cell_data = backend.get_cell_value(0, 1);
            assert_eq!((*cell_data).dependents.len(), 1); // New dependencies added
        }
    }

    #[test]
    fn test_min_function() {
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 2 }, "5")
            .unwrap();

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 2 },
        };
        let result = backend.min_function(&range).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_reset_found() {
        // Lines 190, 193
        let mut backend = Backend::new(3, 3);
        let start = Cell { row: 1, col: 1 };
        backend.set_cell_value(start, "10").unwrap();

        backend.reset_found(&start);

        unsafe {
            let cell_data = backend.get_cell_value(1, 1);
            assert_eq!((*cell_data).dirty_parents, 0);
        }
    }

    #[test]
    fn test_check_circular_dependency() {
        let mut backend = Backend::new(3, 3);
        let cell_a = Cell { row: 0, col: 0 };
        let cell_b = Cell { row: 0, col: 1 };

        let _res1 = backend.set_cell_value(cell_a, "=B1");
        let res = backend.set_cell_value(cell_b, "=A1");

        assert!(res.is_err());
    }

    #[test]
    fn test_multiply_op_overflow() {
        let backend = Backend::new(3, 3);

        // Set up operands that will cause overflow
        let bin_op = BinaryOp {
            first: Operand {
                type_: OperandType::Int,
                data: OperandData::Value(2_147_483_647), // Maximum i32 value
            },
            second: Operand {
                type_: OperandType::Int,
                data: OperandData::Value(2), // Multiplying by 2 will overflow
            },
        };

        let result = backend.multiply_op(&bin_op);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::Overflow);
    }

    #[test]
    fn test_divide_op_by_zero() {
        let backend = Backend::new(3, 3);

        // Set up operands where the second operand is zero
        let bin_op = BinaryOp {
            first: Operand {
                type_: OperandType::Int,
                data: OperandData::Value(42),
            },
            second: Operand {
                type_: OperandType::Int,
                data: OperandData::Value(0), // Division by zero
            },
        };

        let result = backend.divide_op(&bin_op);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DivideByZero);
    }

    #[test]
    fn test_set_dirty_parents() {
        // Lines 221-226, 231-235
        let mut backend = Backend::new(3, 3);
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "10").unwrap();

        let mut stack = Vec::new();
        backend.set_dirty_parents(&cell, &mut stack);

        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_update_dependents() {
        // Lines 237-240, 244-248
        let mut backend = Backend::new(3, 3);
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "10").unwrap();

        backend.update_dependents(&cell);

        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            assert_eq!((*cell_data).value, 10);
        }
    }

    #[test]
    fn test_evaluate_expression_binary_op() {
        // Lines 257-262, 267-271
        let backend = Backend::new(3, 3);
        let func = Function::new_binary_op(
            FunctionType::Plus,
            BinaryOp {
                first: Operand {
                    type_: OperandType::Int,
                    data: OperandData::Value(10),
                },
                second: Operand {
                    type_: OperandType::Int,
                    data: OperandData::Value(20),
                },
            },
        );

        let (value, error) = backend.evaluate_expression(&func);
        assert_eq!(value, 30);
        assert_eq!(error, CellError::NoError);
    }

    // #[test]
    // fn test_min_function() {
    //     // Lines 308, 311-312
    //     let mut backend = Backend::new(3, 3);
    //     backend.set_cell_value(Cell { row: 0, col: 0 }, "10").unwrap();
    //     backend.set_cell_value(Cell { row: 0, col: 1 }, "20").unwrap();
    //     backend.set_cell_value(Cell { row: 0, col: 2 }, "5").unwrap();

    //     let range = RangeFunction {
    //         top_left: Cell { row: 0, col: 0 },
    //         bottom_right: Cell { row: 0, col: 2 },
    //     };
    //     let result = backend.min_function(&range).unwrap();
    //     assert_eq!(result, 5);
    // }

    #[test]
    fn test_max_function() {
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 2 }, "5")
            .unwrap();

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 2 },
        };
        let result = backend.max_function(&range).unwrap();
        assert_eq!(result, 20);

        // Test with negative values
        backend
            .set_cell_value(Cell { row: 1, col: 0 }, "-10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 1, col: 1 }, "-20")
            .unwrap();
        let range = RangeFunction {
            top_left: Cell { row: 1, col: 0 },
            bottom_right: Cell { row: 1, col: 1 },
        };
        let result = backend.max_function(&range).unwrap();
        assert_eq!(result, -10);
    }

    #[test]
    fn test_avg_function() {
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 2 }, "30")
            .unwrap();

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 2 },
        };
        let result = backend.avg_function(&range).unwrap();
        assert_eq!(result, 20);

        // Test with zero values
        backend
            .set_cell_value(Cell { row: 1, col: 0 }, "0")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 1, col: 1 }, "0")
            .unwrap();
        let range = RangeFunction {
            top_left: Cell { row: 1, col: 0 },
            bottom_right: Cell { row: 1, col: 1 },
        };
        let result = backend.avg_function(&range).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sum_function() {
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 2 }, "30")
            .unwrap();

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 2 },
        };
        let result = backend.sum_function(&range).unwrap();
        assert_eq!(result, 60);

        // Test with negative values
        backend
            .set_cell_value(Cell { row: 1, col: 0 }, "-10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 1, col: 1 }, "-20")
            .unwrap();
        let range = RangeFunction {
            top_left: Cell { row: 1, col: 0 },
            bottom_right: Cell { row: 1, col: 1 },
        };
        let result = backend.sum_function(&range).unwrap();
        assert_eq!(result, -30);
    }

    #[test]
    fn test_stdev_function() {
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 2 }, "30")
            .unwrap();

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 2 },
        };
        let result = backend.stdev_function(&range).unwrap();
        assert_eq!(result, 8); // Standard deviation of [10, 20, 30] is approximately 8.16, floored to 8

        // Test with a single value
        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };
        let result = backend.stdev_function(&range).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_plus_op() {
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();

        let bin_op = BinaryOp {
            first: Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 0 }),
            },
            second: Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 1 }),
            },
        };
        let result = backend.plus_op(&bin_op).unwrap();
        assert_eq!(result, 30);

        // Test with negative values
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "-10")
            .unwrap();
        let result = backend.plus_op(&bin_op).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_minus_op() {
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "10")
            .unwrap();

        let bin_op = BinaryOp {
            first: Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 0 }),
            },
            second: Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 1 }),
            },
        };
        let result = backend.minus_op(&bin_op).unwrap();
        assert_eq!(result, 10);

        // Test with negative values
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "-10")
            .unwrap();
        let result = backend.minus_op(&bin_op).unwrap();
        assert_eq!(result, 30);
    }

    #[test]
    fn test_multiply_op() {
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "5")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "4")
            .unwrap();

        let bin_op = BinaryOp {
            first: Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 0 }),
            },
            second: Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 1 }),
            },
        };
        let result = backend.multiply_op(&bin_op).unwrap();
        assert_eq!(result, 20);

        // Test with zero
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "0")
            .unwrap();
        let result = backend.multiply_op(&bin_op).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_divide_op() {
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "10")
            .unwrap();

        let bin_op = BinaryOp {
            first: Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 0 }),
            },
            second: Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 1 }),
            },
        };
        let result = backend.divide_op(&bin_op).unwrap();
        assert_eq!(result, 2);

        // Test division by zero
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "0")
            .unwrap();
        let result = backend.divide_op(&bin_op);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DivideByZero);
    }

    #[test]
    fn test_get_operand_value() {
        // Lines 466-469, 471
        let mut backend = Backend::new(3, 3);
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "42")
            .unwrap();

        let operand = Operand {
            type_: OperandType::Cell,
            data: OperandData::Cell(Cell { row: 0, col: 0 }),
        };

        let result = backend.get_operand_value(&operand).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_get_rows() {
        // Lines 707-708
        let backend = Backend::new(3, 3);
        assert_eq!(backend.get_rows(), 3);
    }

    #[test]
    fn test_get_cols() {
        // Lines 711
        let backend = Backend::new(3, 3);
        assert_eq!(backend.get_cols(), 3);
    }

    #[test]
    fn test_update_graph_with_range_function() {
        let mut backend = Backend::new(5, 5);
        let cell = Cell { row: 2, col: 2 };

        // Set the old function as a RangeFunction
        let _old_function = Function::new_range_function(
            FunctionType::Sum,
            RangeFunction {
                top_left: Cell { row: 0, col: 0 },
                bottom_right: Cell { row: 1, col: 1 },
            },
        );

        // Verify that the old dependencies are removed
        unsafe {
            for row in 0..=1 {
                for col in 0..=1 {
                    let parent_data = backend.get_cell_value(row, col);
                    assert!(!(*parent_data).dependents.contains(&(2, 2)));
                }
            }
        }

        // Set the new function as a RangeFunction
        backend.set_cell_value(cell, "SUM(A1:B2)").unwrap();

        // Update the graph
        // backend.update_graph(&cell, &old_function);

        // Verify that the new dependencies are added
        unsafe {
            for row in 0..=1 {
                for col in 0..=1 {
                    let parent_data = backend.get_cell_value(row, col);
                    assert!((*parent_data).dependents.contains(&(2, 2)));
                }
            }
        }
    }

    #[test]
    fn test_update_graph_with_binary_op() {
        let mut backend = Backend::new(5, 5);
        let cell = Cell { row: 2, col: 2 };

        // Set the old function as a BinaryOp
        let _old_function = Function::new_binary_op(
            FunctionType::Plus,
            BinaryOp {
                first: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 0, col: 0 }),
                },
                second: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 1, col: 1 }),
                },
            },
        );

        // Verify that the old dependencies are removed
        unsafe {
            let parent_data = backend.get_cell_value(0, 0);
            assert!(!(*parent_data).dependents.contains(&(2, 2)));

            let parent_data = backend.get_cell_value(0, 1);
            assert!(!(*parent_data).dependents.contains(&(2, 2)));
        }

        // Set the new function as a BinaryOp
        backend.set_cell_value(cell, "A1+B1").unwrap();

        // Update the graph
        // backend.update_graph(&cell, &old_function);

        // Verify that the new dependencies are added
        unsafe {
            let parent_data = backend.get_cell_value(0, 0);
            assert!((*parent_data).dependents.contains(&(2, 2)));

            let parent_data2 = backend.get_cell_value(0, 1);
            assert!((*parent_data2).dependents.contains(&(2, 2)));
        }
    }

    #[test]
    fn test_update_graph_with_sleep_cell() {
        let mut backend = Backend::new(5, 5);
        let cell = Cell { row: 2, col: 2 };

        // Set the old function as a SleepValue
        let _old_function = Function {
            type_: FunctionType::Sleep,
            data: FunctionData::SleepValue(Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 0 }),
            }),
        };

        // Verify that the old dependencies are removed
        unsafe {
            let parent_data = backend.get_cell_value(0, 0);
            assert!(!(*parent_data).dependents.contains(&(2, 2)));
        }

        // Set the new function as a SleepValue
        backend.set_cell_value(cell, "SLEEP(A1)").unwrap();

        // Update the graph
        // backend.update_graph(&cell, &old_function);

        // Verify that the new dependencies are added
        unsafe {
            let parent_data = backend.get_cell_value(0, 0);
            assert!((*parent_data).dependents.contains(&(2, 2)));
        }
    }

    #[test]
    fn test_update_graph_with_sleep_value() {
        let mut backend = Backend::new(5, 5);
        let cell = Cell { row: 2, col: 2 };

        // Set the old function as a SleepValue
        let _old_function = Function {
            type_: FunctionType::Sleep,
            data: FunctionData::SleepValue(Operand {
                type_: OperandType::Int,
                data: OperandData::Value(0),
            }),
        };

        // Set the new function as a SleepValue
        backend.set_cell_value(cell, "SLEEP(0)").unwrap();
    }

    #[test]
    fn test_update_graph_with_cell_data_as_range_function() {
        let mut backend = Backend::new(5, 5);
        let cell = Cell { row: 2, col: 2 };

        // Set the cell's function as a RangeFunction
        backend.set_cell_value(cell, "SUM(A1:B2)").unwrap();

        // Update the graph
        // backend.update_graph(&cell, &Function::new_constant(0));

        // Verify that the dependencies are added
        unsafe {
            for row in 0..=1 {
                for col in 0..=1 {
                    let parent_data = backend.get_cell_value(row, col);
                    assert!((*parent_data).dependents.contains(&(2, 2)));
                }
            }
        }
    }

    #[test]
    fn test_get_operand_value_division_by_zero_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell value to 0 to simulate division by zero
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "0").unwrap();

        // Create an operand referencing the cell
        let operand = Operand {
            type_: OperandType::Cell,
            data: OperandData::Cell(cell),
        };

        // Simulate a division by zero error
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DivideByZero;
        }

        // Call get_operand_value and verify the error
        let result = backend.get_operand_value(&operand);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DivideByZero);
    }

    #[test]
    fn test_get_operand_value_dependency_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell value to simulate a dependency error
        let cell = Cell { row: 1, col: 1 };
        backend.set_cell_value(cell, "42").unwrap();

        // Create an operand referencing the cell
        let operand = Operand {
            type_: OperandType::Cell,
            data: OperandData::Cell(cell),
        };

        // Simulate a dependency error
        unsafe {
            let cell_data = backend.get_cell_value(1, 1);
            (*cell_data).error = CellError::DependencyError;
        }

        // Call get_operand_value and verify the error
        let result = backend.get_operand_value(&operand);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DependencyError);
    }

    #[test]
    fn test_sum_function_division_by_zero_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a division by zero error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "0").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DivideByZero;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.sum_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DivideByZero);
    }

    #[test]
    fn test_sum_function_dependency_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a dependency error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "42").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DependencyError;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.sum_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DependencyError);
    }

    #[test]
    fn test_stdev_function_division_by_zero_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a division by zero error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "0").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DivideByZero;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.stdev_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DivideByZero);
    }

    #[test]
    fn test_stdev_function_dependency_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a dependency error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "42").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DependencyError;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.stdev_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DependencyError);
    }

    #[test]
    fn test_avg_function_division_by_zero_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a division by zero error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "0").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DivideByZero;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.avg_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DivideByZero);
    }

    #[test]
    fn test_avg_function_dependency_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a dependency error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "42").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DependencyError;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.avg_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DependencyError);
    }

    #[test]
    fn test_max_function_division_by_zero_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a division by zero error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "0").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DivideByZero;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.max_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DivideByZero);
    }

    #[test]
    fn test_max_function_dependency_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a dependency error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "42").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DependencyError;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.max_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DependencyError);
    }

    #[test]
    fn test_min_function_division_by_zero_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a division by zero error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "0").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DivideByZero;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.min_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DivideByZero);
    }

    #[test]
    fn test_min_function_dependency_error() {
        let mut backend = Backend::new(3, 3);

        // Set a cell with a dependency error
        let cell = Cell { row: 0, col: 0 };
        backend.set_cell_value(cell, "42").unwrap();
        unsafe {
            let cell_data = backend.get_cell_value(0, 0);
            (*cell_data).error = CellError::DependencyError;
        }

        let range = RangeFunction {
            top_left: Cell { row: 0, col: 0 },
            bottom_right: Cell { row: 0, col: 0 },
        };

        let result = backend.min_function(&range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CellError::DependencyError);
    }

    #[test]
    fn test_evaluate_expression_minus() {
        let mut backend = Backend::new(3, 3);

        // Set up operands
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "10")
            .unwrap();

        // Create a minus function
        let func = Function::new_binary_op(
            FunctionType::Minus,
            BinaryOp {
                first: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 0, col: 0 }),
                },
                second: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 0, col: 1 }),
                },
            },
        );

        // Evaluate the function
        let (value, error) = backend.evaluate_expression(&func);
        assert_eq!(value, 10);
        assert_eq!(error, CellError::NoError);
    }

    #[test]
    fn test_evaluate_expression_multiply() {
        let mut backend = Backend::new(3, 3);

        // Set up operands
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "5")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "4")
            .unwrap();

        // Create a multiply function
        let func = Function::new_binary_op(
            FunctionType::Multiply,
            BinaryOp {
                first: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 0, col: 0 }),
                },
                second: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 0, col: 1 }),
                },
            },
        );

        // Evaluate the function
        let (value, error) = backend.evaluate_expression(&func);
        assert_eq!(value, 20);
        assert_eq!(error, CellError::NoError);
    }

    #[test]
    fn test_evaluate_expression_divide() {
        let mut backend = Backend::new(3, 3);

        // Set up operands
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "10")
            .unwrap();

        // Create a divide function
        let func = Function::new_binary_op(
            FunctionType::Divide,
            BinaryOp {
                first: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 0, col: 0 }),
                },
                second: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 0, col: 1 }),
                },
            },
        );

        // Evaluate the function
        let (value, error) = backend.evaluate_expression(&func);
        assert_eq!(value, 2);
        assert_eq!(error, CellError::NoError);
    }

    #[test]
    fn test_evaluate_expression_min() {
        let mut backend = Backend::new(3, 3);

        // Set up range values
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 2 }, "5")
            .unwrap();

        // Create a min function
        let func = Function::new_range_function(
            FunctionType::Min,
            RangeFunction {
                top_left: Cell { row: 0, col: 0 },
                bottom_right: Cell { row: 0, col: 2 },
            },
        );

        // Evaluate the function
        let (value, error) = backend.evaluate_expression(&func);
        assert_eq!(value, 5);
        assert_eq!(error, CellError::NoError);
    }

    #[test]
    fn test_evaluate_expression_max() {
        let mut backend = Backend::new(3, 3);

        // Set up range values
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 2 }, "5")
            .unwrap();

        // Create a max function
        let func = Function::new_range_function(
            FunctionType::Max,
            RangeFunction {
                top_left: Cell { row: 0, col: 0 },
                bottom_right: Cell { row: 0, col: 2 },
            },
        );

        // Evaluate the function
        let (value, error) = backend.evaluate_expression(&func);
        assert_eq!(value, 20);
        assert_eq!(error, CellError::NoError);
    }

    #[test]
    fn test_evaluate_expression_avg() {
        let mut backend = Backend::new(3, 3);

        // Set up range values
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 2 }, "30")
            .unwrap();

        // Create an avg function
        let func = Function::new_range_function(
            FunctionType::Avg,
            RangeFunction {
                top_left: Cell { row: 0, col: 0 },
                bottom_right: Cell { row: 0, col: 2 },
            },
        );

        // Evaluate the function
        let (value, error) = backend.evaluate_expression(&func);
        assert_eq!(value, 20);
        assert_eq!(error, CellError::NoError);
    }

    #[test]
    fn test_evaluate_expression_stdev() {
        let mut backend = Backend::new(3, 3);

        // Set up range values
        backend
            .set_cell_value(Cell { row: 0, col: 0 }, "10")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 1 }, "20")
            .unwrap();
        backend
            .set_cell_value(Cell { row: 0, col: 2 }, "30")
            .unwrap();

        // Create a stdev function
        let func = Function::new_range_function(
            FunctionType::Stdev,
            RangeFunction {
                top_left: Cell { row: 0, col: 0 },
                bottom_right: Cell { row: 0, col: 2 },
            },
        );

        // Evaluate the function
        let (value, error) = backend.evaluate_expression(&func);
        assert_eq!(value, 8); // Standard deviation of [10, 20, 30] is approximately 8.16, floored to 8
        assert_eq!(error, CellError::NoError);
    }

    #[test]
    fn test_update_graph_with_range_function2() {
        let mut backend = Backend::new(5, 5);
        let cell = Cell { row: 2, col: 2 };

        // Set the old function as a RangeFunction
        let old_function = Function::new_range_function(
            FunctionType::Sum,
            RangeFunction {
                top_left: Cell { row: 0, col: 0 },
                bottom_right: Cell { row: 1, col: 1 },
            },
        );
        // Verify that the old dependencies are removed
        unsafe {
            for row in 0..=1 {
                for col in 0..=1 {
                    let parent_data = backend.get_cell_value(row, col);
                    assert!(!(*parent_data).dependents.contains(&(2, 2)));
                }
            }
        }

        // Set the new function as a RangeFunction
        backend.set_cell_value(cell, "SUM(A1:B2)").unwrap();

        // Update the graph
        backend.update_graph(&cell, &old_function);

        // Verify that the new dependencies are added
        unsafe {
            for row in 0..=1 {
                for col in 0..=1 {
                    let parent_data = backend.get_cell_value(row, col);
                    assert!((*parent_data).dependents.contains(&(2, 2)));
                }
            }
        }
    }

    #[test]
    fn test_update_graph_with_binary_op2() {
        let mut backend = Backend::new(5, 5);
        let cell = Cell { row: 2, col: 2 };

        // Set the old function as a BinaryOp
        let old_function = Function::new_binary_op(
            FunctionType::Plus,
            BinaryOp {
                first: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 0, col: 0 }),
                },
                second: Operand {
                    type_: OperandType::Cell,
                    data: OperandData::Cell(Cell { row: 1, col: 1 }),
                },
            },
        );
        // Verify that the old dependencies are removed
        unsafe {
            let parent_data = backend.get_cell_value(0, 0);
            assert!(!(*parent_data).dependents.contains(&(2, 2)));

            let parent_data = backend.get_cell_value(1, 1);
            assert!(!(*parent_data).dependents.contains(&(2, 2)));
        }

        // Set the new function as a BinaryOp
        backend.set_cell_value(cell, "A1+B2").unwrap();

        // Update the graph
        backend.update_graph(&cell, &old_function);

        // Verify that the new dependencies are added
        unsafe {
            let parent_data = backend.get_cell_value(0, 0);
            assert!((*parent_data).dependents.contains(&(2, 2)));

            let parent_data = backend.get_cell_value(1, 1);
            assert!((*parent_data).dependents.contains(&(2, 2)));
        }
    }

    #[test]
    fn test_update_graph_with_sleep_value2() {
        let mut backend = Backend::new(5, 5);
        let cell = Cell { row: 2, col: 2 };

        // Set the old function as a SleepValue
        let old_function = Function {
            type_: FunctionType::Sleep,
            data: FunctionData::SleepValue(Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 0 }),
            }),
        };
        // Verify that the old dependencies are removed
        unsafe {
            let parent_data = backend.get_cell_value(0, 0);
            assert!(!(*parent_data).dependents.contains(&(2, 2)));
        }

        // Set the new function as a SleepValue
        backend.set_cell_value(cell, "SLEEP(A1)").unwrap();

        // Update the graph
        backend.update_graph(&cell, &old_function);

        // Verify that the new dependencies are added
        unsafe {
            let parent_data = backend.get_cell_value(0, 0);
            assert!((*parent_data).dependents.contains(&(2, 2)));
        }
    }
}
