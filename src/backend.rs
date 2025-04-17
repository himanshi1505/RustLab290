use crate::parser::*;
use crate::structs::*;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::VecDeque;
use std::f64;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

#[cfg(feature = "gui")]
use csv::{ReaderBuilder, WriterBuilder};

pub struct Backend {
    grid: Vec<Vec<Rc<RefCell<CellData>>>>,
    undo_stack: VecDeque<Vec<Vec<i32>>>,
    redo_stack: VecDeque<Vec<Vec<i32>>>,
    rows: usize,
    cols: usize,
}

impl Backend {
    pub fn get_rows_col(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }
    pub fn new(rows: usize, cols: usize) -> Self {
        //init backend
        let mut grid = Vec::new();

        for _ in 0..rows {
            let mut row = Vec::new();
            for _ in 0..cols {
                row.push(Rc::new(RefCell::new(CellData::default())));
            }
            grid.push(row);
        }

        Backend {
            grid,
            rows,
            cols,
            undo_stack: VecDeque::with_capacity(100),
            redo_stack: VecDeque::with_capacity(100),
        }
    }
    pub fn reset(&mut self) {
        for row in &mut self.grid {
            for cell in row {
                *cell = Rc::new(RefCell::new(CellData::default()));
            }
        }
    }

    pub fn get_cell_value(&self, cell: &Cell) -> Option<&Rc<RefCell<CellData>>> {
        self.grid.get(cell.row)?.get(cell.col)
    }

    // pub fn get_cell_error(&self, cell: &Cell) -> CellError {
    //     self.grid.get()
    // }

    pub fn reset_found(&mut self, start: &Cell) {
        if let Some(cell_data_rc) = self.get_cell_value(start) {
            let mut cell_data = cell_data_rc.borrow_mut();
            cell_data.dirty_parents = 0;
            drop(cell_data);
            let mut stack = vec![Rc::clone(cell_data_rc)];

            while let Some(current_rc) = stack.pop() {
                let current = current_rc.borrow();

                for dep_rc in &current.dependents {
                    let mut dep = dep_rc.borrow_mut();
                    if dep.dirty_parents > 0 {
                        dep.dirty_parents = 0;
                        stack.push(Rc::clone(dep_rc));
                    }
                }
            }
        }
    }

    /// Checks if setting this cell creates a circular dependency
    pub fn is_in_cycle(&mut self, start: &Cell) -> bool {
        let mut found_cycle = false;

        // Mark starting cell as visited
        if let Some(cell_data_rc) = self.get_cell_value(start) {
            let mut stack = vec![Rc::clone(cell_data_rc)];
            let mut cell_data = cell_data_rc.borrow_mut();
            cell_data.dirty_parents = 1;
            drop(cell_data);

            while let Some(current_rc) = stack.pop() {
                // Get all dependents of current cell

                let current_cell_data = current_rc.borrow();
                // To avoid borrow checker issues, collect cells to process first
                let mut deps_to_check = Vec::new();
                let mut cycle_detected = false;

                // First pass: check for cycles and collect cells to process
                for dep in &current_cell_data.dependents {
                    // println!("dep: {:?}", dep);
                    // Cycle detected if we return to start cell
                    if let Some(start_data) = self.get_cell_value(start) {
                        if Rc::ptr_eq(dep, start_data) {
                            cycle_detected = true;
                            break;
                        }
                    }
                    deps_to_check.push(dep);
                }

                if cycle_detected {
                    found_cycle = true;
                    break;
                }

                // Second pass: process unvisited cells
                for dep in deps_to_check {
                    let mut dep_data = dep.borrow_mut();
                    if dep_data.dirty_parents == 0 {
                        dep_data.dirty_parents = 1;
                        stack.push(Rc::clone(dep));
                    }
                }
            }
        }
        // Reset visited markers
        self.reset_found(start);
        found_cycle
    }

    pub fn check_circular_dependency(&mut self, cell: &Cell) -> bool {
        self.is_in_cycle(cell)
    }

    pub fn update_graph(&mut self, cell: &Cell, old_function: &Function) {
        // Remove old dependencies
        if let Some(cell_data_rc) = self.get_cell_value(cell) {
            match &old_function.data {
                FunctionData::RangeFunction(range) => {
                    // Remove from all cells in old range

                    for row in range.top_left.row..=range.bottom_right.row {
                        for col in range.top_left.col..=range.bottom_right.col {
                            let parent_cell = Cell { row, col };
                            if let Some(parent_data_rc) = self.get_cell_value(&parent_cell) {
                                let mut parent_data = parent_data_rc.borrow_mut();
                                parent_data
                                    .dependents
                                    .retain(|c| !Rc::ptr_eq(c, cell_data_rc));
                            }
                        }
                    }
                }
                FunctionData::BinaryOp(bin_op) => {
                    // Remove first operand dependency
                    if let OperandData::Cell(dep) = bin_op.first.data {
                        if let Some(parent_data_rc) = self.get_cell_value(&dep) {
                            let mut parent_data = parent_data_rc.borrow_mut();
                            parent_data
                                .dependents
                                .retain(|c| !Rc::ptr_eq(c, cell_data_rc));
                        }
                    }
                    // Remove second operand dependency
                    if let OperandData::Cell(dep) = bin_op.second.data {
                        if let Some(parent_data_rc) = self.get_cell_value(&dep) {
                            let mut parent_data = parent_data_rc.borrow_mut();
                            parent_data
                                .dependents
                                .retain(|c| !Rc::ptr_eq(c, cell_data_rc));
                        }
                    }
                }
                FunctionData::SleepValue(operand) => {
                    if let OperandData::Cell(dep) = operand.data {
                        if let Some(parent_data_rc) = self.get_cell_value(&dep) {
                            let mut parent_data = parent_data_rc.borrow_mut();
                            parent_data
                                .dependents
                                .retain(|c| !Rc::ptr_eq(c, cell_data_rc));
                        }
                    }
                }
                FunctionData::Value(_) => {} // Constants have no dependencies
            }
        }

        // Add new dependencies
        if let Some(cell_data_rc) = self.get_cell_value(cell) {
            let mut cell_data = cell_data_rc.borrow_mut();

            match &cell_data.function.data {
                FunctionData::RangeFunction(range) => {
                    // Add to all cells in new range
                    for row in range.top_left.row..=range.bottom_right.row {
                        for col in range.top_left.col..=range.bottom_right.col {
                            let parent_cell = Cell { row, col };
                            if let Some(parent_data_rc) = self.get_cell_value(&parent_cell) {
                                let mut parent_data = parent_data_rc.borrow_mut();
                                parent_data.dependents.push(Rc::clone(&cell_data_rc));
                            }
                        }
                    }
                }
                FunctionData::BinaryOp(bin_op) => {
                    // Add first operand dependency
                    if let OperandData::Cell(dep) = bin_op.first.data {
                        if let Some(parent_data_rc) = self.get_cell_value(&dep) {
                            let mut parent_data = parent_data_rc.borrow_mut();
                            parent_data.dependents.push(Rc::clone(&cell_data_rc));
                        }
                    }
                    // Add second operand dependency
                    if let OperandData::Cell(dep) = bin_op.second.data {
                        if let Some(parent_data_rc) = self.get_cell_value(&dep) {
                            let mut parent_data = parent_data_rc.borrow_mut();
                            parent_data.dependents.push(Rc::clone(&cell_data_rc));
                        }
                    }
                }
                FunctionData::SleepValue(operand) => {
                    if let OperandData::Cell(dep) = operand.data {
                        if let Some(parent_data_rc) = self.get_cell_value(&dep) {
                            let mut parent_data = parent_data_rc.borrow_mut();
                            parent_data.dependents.push(Rc::clone(&cell_data_rc));
                        }
                    }
                }
                FunctionData::Value(_) => {} // Constants have no dependencies
            }
        }
    }

    /// Sets dirty parent counts for topological sorting
    //check if stack has the copied values or references??

    pub fn set_dirty_parents(&mut self, cell: &Cell, stack: &mut Vec<Rc<RefCell<CellData>>>) {
        if let Some(root_data_rc) = self.get_cell_value(cell) {
            let mut root_data = root_data_rc.borrow_mut();
            root_data.dirty_parents = 0;
            stack.push(Rc::clone(root_data_rc));
            drop(root_data);

            while let Some(current_rc) = stack.pop() {
                let current = current_rc.borrow();

                for child_data_rc in &current.dependents {
                    let mut child_data = child_data_rc.borrow_mut();
                    if (child_data.dirty_parents) == 0 {
                        stack.push(Rc::clone(child_data_rc));
                    }
                    child_data.dirty_parents += 1;
                }
            }
        }
    }

    /// Recursively update dependent cells using topological sort
    pub fn update_dependents(&mut self, cell: &Cell) {
        let mut stack = Vec::new();
        self.set_dirty_parents(cell, &mut stack);

        let mut process_stack = Vec::new();
        if let Some(cell_data_rc) = self.get_cell_value(cell) {
            for child_data_rc in &cell_data_rc.borrow_mut().dependents {
                let mut child_data = child_data_rc.borrow_mut();
                child_data.dirty_parents -= 1;
                if child_data.dirty_parents == 0 {
                    process_stack.push(Rc::clone(child_data_rc));
                }
            }
        }

        while let Some(current_data_rc) = process_stack.pop() {
            let mut current_data = current_data_rc.borrow_mut();
            let (new_value, error) = self.evaluate_expression(&current_data.function);
            current_data.value = new_value;
            current_data.error = error;

            for dependent_data_rc in &current_data.dependents {
                let mut dependent_data = dependent_data_rc.borrow_mut();
                dependent_data.dirty_parents -= 1;
                if dependent_data.dirty_parents == 0 {
                    process_stack.push(Rc::clone(dependent_data_rc));
                }
            }
        }
    }

    /// Checks if this function can be safely replaced with a constant value
    pub fn is_expression_constant(&self, func: &Function) -> bool {
        match func.type_ {
            FunctionType::Plus
            | FunctionType::Minus
            | FunctionType::Multiply
            | FunctionType::Divide => {
                if let FunctionData::BinaryOp(bin_op) = func.data {
                    matches!(bin_op.first.type_, OperandType::Int)
                        && matches!(bin_op.second.type_, OperandType::Int)
                } else {
                    false
                }
            }
            FunctionType::Constant => true,
            _ => false,
        }
    }

    /// Evaluates a function and returns (value, error)
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
    pub fn set_cell_value(&mut self, cell: Cell, expression: &str) -> Result<(), ExpressionError> {
        // Parse the expression
        let (new_function, success) = self.parse_expression(expression);
        if !success {
            return Err(ExpressionError::CouldNotParse);
        }

        // Get cell data
        let cell_data_rc = self
            .get_cell_value(&cell)
            .ok_or(ExpressionError::CouldNotParse)?;
        let cell_data_rc = Rc::clone(&cell_data_rc); // Clone the Rc to avoid borrow issues

        // Extract old state with a scoped borrow
        let (old_function, old_value) = {
            let cell_data = cell_data_rc.borrow();
            (cell_data.function.clone(), cell_data.value)
        };

        // Handle constant functions early
        if new_function.type_ == FunctionType::Constant {
            let (new_value, error) = self.evaluate_expression(&new_function);

            // Update cell in a single mutable borrow
            {
                let mut cell_data = cell_data_rc.borrow_mut();
                cell_data.value = new_value;
                cell_data.error = error;
                cell_data.function = new_function;
                // println!("cell_data.value: {:?}", cell_data.value);
            }

            self.update_graph(&cell, &old_function);
            self.update_dependents(&cell);
            return Ok(());
        }
        //iterate through the new rhs cells and if any of them is equal to lhs, then return error circular dependency
        match &new_function.data {
            FunctionData::BinaryOp(bin_op) => {
                if bin_op.first.data == OperandData::Cell(cell)
                    || bin_op.second.data == OperandData::Cell(cell)
                {
                    return Err(ExpressionError::CircularDependency);
                }
            }
            FunctionData::RangeFunction(range) => {
                // Check all cells in the range
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
        // Update function in a separate borrow scope
        {
            let mut cell_data = cell_data_rc.borrow_mut();
            cell_data.function = new_function.clone();
        }

        // Update graph with old function
        self.update_graph(&cell, &old_function);

        // Check for circular dependencies
        if self.check_circular_dependency(&cell) {
            // Revert changes in a separate borrow scope
            {
                let mut cell_data = cell_data_rc.borrow_mut();
                cell_data.function = old_function; // No need to clone again
            }

            self.update_graph(&cell, &new_function);
            return Err(ExpressionError::CircularDependency);
        }

        // Evaluate new value
        let (new_value, error) = self.evaluate_expression(&new_function);
        // println!("error: {:?}", error);
        // Update cell value in a separate borrow scope
        {
            let mut cell_data = cell_data_rc.borrow_mut();
            cell_data.value = if error == CellError::NoError {
                new_value
            } else {
                0
            };
            cell_data.error = error;
        }

        // Update dependents
        self.update_dependents(&cell);

        Ok(())
    }

    pub fn min_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut min_val = i32::MAX;
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                let cell = Cell { row, col };
                if let Some(cell_data_rc) = self.get_cell_value(&cell) {
                    let cell_data = cell_data_rc.borrow();
                    match cell_data.error {
                        CellError::NoError => {
                            min_val = min(min_val, cell_data.value);
                        }
                        CellError::DivideByZero => return Err(CellError::DivideByZero),
                        CellError::DependencyError => return Err(CellError::DependencyError),
                    }
                } else {
                    return Err(CellError::DependencyError);
                }
            }
        }
        Ok(min_val)
    }

    pub fn max_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut max_val = i32::MIN;
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                let cell = Cell { row, col };
                if let Some(cell_data_rc) = self.get_cell_value(&cell) {
                    let cell_data = cell_data_rc.borrow();
                    if let Some(cell_data_rc) = self.get_cell_value(&cell) {
                        let cell_data = cell_data_rc.borrow();
                        match cell_data.error {
                            CellError::NoError => {
                                max_val = max(max_val, cell_data.value);
                            }
                            CellError::DivideByZero => return Err(CellError::DivideByZero),
                            CellError::DependencyError => return Err(CellError::DependencyError),
                        }
                    }
                } else {
                    return Err(CellError::DependencyError);
                }
            }
        }
        Ok(max_val)
    }

    pub fn avg_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut sum = 0;
        let mut count = 0;
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                let cell = Cell { row, col };
                if let Some(cell_data_rc) = self.get_cell_value(&cell) {
                    let cell_data = cell_data_rc.borrow();
                    if let Some(cell_data_rc) = self.get_cell_value(&cell) {
                        let cell_data = cell_data_rc.borrow();
                        match cell_data.error {
                            CellError::NoError => {
                                sum += cell_data.value;
                                count += 1;
                            }
                            CellError::DivideByZero => return Err(CellError::DivideByZero),
                            CellError::DependencyError => return Err(CellError::DependencyError),
                        }
                    }
                } else {
                    return Err(CellError::DependencyError);
                }
            }
        }
        if count == 0 {
            return Err(CellError::DivideByZero);
        }
        Ok(sum / count)
    }

    pub fn sum_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut sum = 0;
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                let cell = Cell { row, col };
                if let Some(cell_data_rc) = self.get_cell_value(&cell) {
                    let cell_data = cell_data_rc.borrow();
                    if let Some(cell_data_rc) = self.get_cell_value(&cell) {
                        let cell_data = cell_data_rc.borrow();
                        match cell_data.error {
                            CellError::NoError => {
                                sum += cell_data.value;
                            }
                            CellError::DivideByZero => return Err(CellError::DivideByZero),
                            CellError::DependencyError => return Err(CellError::DependencyError),
                        }
                    }
                } else {
                    return Err(CellError::DependencyError);
                }
            }
        }
        Ok(sum)
    }

    pub fn stdev_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut values = Vec::new();
        let mut sum = 0;
        let mut count = 0;

        // First pass: collect values and calculate sum
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                let cell = Cell { row, col };
                if let Some(cell_data_rc) = self.get_cell_value(&cell) {
                    let cell_data = cell_data_rc.borrow();
                    if let Some(cell_data_rc) = self.get_cell_value(&cell) {
                        let cell_data = cell_data_rc.borrow();
                        match cell_data.error {
                            CellError::NoError => {
                                let value = cell_data.value;
                                values.push(value);
                                sum += value;
                                count += 1;
                            }
                            CellError::DivideByZero => return Err(CellError::DivideByZero),
                            CellError::DependencyError => return Err(CellError::DependencyError),
                        }
                    }
                } else {
                    return Err(CellError::DependencyError);
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
        Ok((variance as f64).sqrt().round() as i32)
    }

    pub fn sleep_function(&self, operand: &Operand) -> Result<i32, CellError> {
        let value = self.get_operand_value(operand)?;
        // println!("value: {:?}", value);
        if (value > 0) {
            thread::sleep(Duration::from_secs(value as u64));
        }
        Ok(value)
    }

    // Binary operations
    pub fn plus_op(&self, bin_op: &BinaryOp) -> Result<i32, CellError> {
        let first = self.get_operand_value(&bin_op.first)?;
        let second = self.get_operand_value(&bin_op.second)?;
        Ok(first + second)
    }

    pub fn minus_op(&self, bin_op: &BinaryOp) -> Result<i32, CellError> {
        let first = self.get_operand_value(&bin_op.first)?;
        let second = self.get_operand_value(&bin_op.second)?;
        Ok(first - second)
    }

    pub fn multiply_op(&self, bin_op: &BinaryOp) -> Result<i32, CellError> {
        let first = self.get_operand_value(&bin_op.first)?;
        let second = self.get_operand_value(&bin_op.second)?;
        Ok(first * second)
    }

    pub fn divide_op(&self, bin_op: &BinaryOp) -> Result<i32, CellError> {
        let first = self.get_operand_value(&bin_op.first)?;
        let second = self.get_operand_value(&bin_op.second)?;

        if second == 0 {
            return Err(CellError::DivideByZero);
        }

        Ok(first / second)
    }
    fn get_operand_value(&self, operand: &Operand) -> Result<i32, CellError> {
        match operand.data {
            OperandData::Cell(cell) => {
                // Get the cell data
                let cell_data_rc = self
                    .get_cell_value(&cell)
                    .ok_or(CellError::DependencyError)?;
                let cell_data = cell_data_rc.borrow();

                // Check for errors in the cell
                match cell_data.error {
                    CellError::NoError => return Ok(cell_data.value),
                    CellError::DivideByZero => return Err(CellError::DivideByZero),
                    CellError::DependencyError => return Err(CellError::DependencyError),
                }
            }
            // {let celldata = self.get_cell_value(&cell)
            // .ok_or(CellError::DependencyError);
            // // .map(|d| d.borrow().value),
            // let cellerror = celldata.map(|d| d.borrow().error);
            // match cellerror with {
            //     Ok(CellError::NoError) => {let cellvalue = celldata.map(|d| d.borrow().value);
            //     return cellvalue;},
            //     Ok(CellError::DivideByZero) => return Err(CellError::DivideByZero),
            //     Ok(CellError::DependencyError) => return Err(CellError::DependencyError),
            //     _ => {}
            // }}
            // if cellerror != CellError::NoError {
            //     return Err(cellerror);
            // }
            // let cellvalue = celldata.map(|d| d.borrow().value);
            // // if cellvalue.is_none() {
            // //     return Err(CellError::DependencyError);
            // // }
            // return cellvalue;}
            OperandData::Value(value) => Ok(value),
        }
    }

    pub fn parse_expression(&self, expression: &str) -> (Function, bool) {
        crate::parser::parse_expression(expression, &self)
    }

    pub fn get_rows(&self) -> usize {
        self.rows
    }

    pub fn get_cols(&self) -> usize {
        self.cols
    }

    #[cfg(feature = "gui")]
    // Save to CSV file
    pub fn save_to_csv(&self, filename: &str) -> Result<(), csv::Error> {
        let file = File::create(filename)?;
        let mut wtr = WriterBuilder::new().from_writer(BufWriter::new(file));

        for row in 0..self.rows {
            let mut record = Vec::new();
            for col in 0..self.cols {
                let cell = Cell { row, col };
                if let Some(cell_data) = self.get_cell_value(&cell) {
                    record.push(cell_data.borrow().value.to_string());
                } else {
                    record.push(String::new());
                }
            }
            wtr.write_record(&record)?;
        }
        wtr.flush()?;
        Ok(())
    }

    #[cfg(feature = "gui")]
    // Load from CSV file
    pub fn load_from_csv(&mut self, filename: &str) -> Result<(), csv::Error> {
        let file = File::open(filename)?;
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(BufReader::new(file));

        self.clear_undo_redo();
        let mut new_state = self.create_snapshot();

        for (row_idx, result) in rdr.records().enumerate() {
            let record = result?;
            for (col_idx, field) in record.iter().enumerate() {
                if row_idx < self.rows && col_idx < self.cols {
                    let value = field.parse().unwrap_or(0);
                    new_state[row_idx][col_idx] = value;
                }
            }
        }

        self.push_undo_state();
        self.apply_snapshot(new_state);
        Ok(())
    }

    #[cfg(feature = "gui")]
    // Undo last action
    pub fn undo(&mut self) {
        if let Some(prev_state) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(self.create_snapshot());
            self.apply_snapshot(prev_state);
        }
    }

    #[cfg(feature = "gui")]
    // Redo last undone action
    pub fn redo(&mut self) {
        if let Some(next_state) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(self.create_snapshot());
            self.apply_snapshot(next_state);
        }
    }

    #[cfg(feature = "gui")]
    // Helper: Create snapshot of current state
    fn create_snapshot(&self) -> Vec<Vec<i32>> {
        let mut snapshot = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let mut row_data = Vec::with_capacity(self.cols);
            for col in 0..self.cols {
                let cell = Cell { row, col };
                row_data.push(
                    self.get_cell_value(&cell)
                        .map(|c| c.borrow().value)
                        .unwrap_or(0),
                );
            }
            snapshot.push(row_data);
        }
        snapshot
    }

    #[cfg(feature = "gui")]
    // Helper: Apply state snapshot
    fn apply_snapshot(&mut self, snapshot: Vec<Vec<i32>>) {
        for (row_idx, row) in snapshot.iter().enumerate() {
            for (col_idx, &value) in row.iter().enumerate() {
                let cell = Cell {
                    row: row_idx,
                    col: col_idx,
                };
                if let Some(cell_data) = self.get_cell_value(&cell) {
                    cell_data.borrow_mut().value = value;
                }
            }
        }
    }

    #[cfg(feature = "gui")]
    // Helper: Clear undo/redo stacks
    fn clear_undo_redo(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    #[cfg(feature = "gui")]
    // Helper: Save current state to undo stack
    fn push_undo_state(&mut self) {
        if self.undo_stack.len() >= 100 {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(self.create_snapshot());
    }
}

// typedef struct CellData {
//     /**
//      * The children
//      */
//     Vec dependents;
//     /**
//      * Cells that this cell depends on
//      */
//     Function function;
//     CellError error;
//     /**
//      * The number of parents that need to be recalculated before this one can be
//      */
//     int dirty_parents;
//     int value;
//     /**
//      * Useful for DFS
//      * */
// } CellData;

// } CellData;
