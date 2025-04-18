use crate::parser::*;
use crate::structs::*;
use std::cell::UnsafeCell;
use std::cmp::{max, min};
use std::collections::VecDeque;
use std::f64;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::rc::Rc;
use std::thread;
use std::ptr::eq;
use std::time::Duration;

#[cfg(feature = "gui")]
use csv::{ReaderBuilder, WriterBuilder};

pub struct Backend {
    grid: UnsafeCell<Vec<Vec<CellData>>>,
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
        let mut grid = Vec::with_capacity(rows);
        for row in 0..rows {
            let mut row_vec = Vec::with_capacity(cols);
            for col in 0..cols {
                row_vec.push(CellData {
                    value: 0,
                    dependents: UnsafeCell::new(Vec::new()),
                    function: Function::new_constant(0),
                    error: CellError::NoError,
                    dirty_parents: 0,
                });
            }
            grid.push(row_vec);
        }

        Backend {
            grid: UnsafeCell::new(grid),
            undo_stack: VecDeque::with_capacity(100),
            redo_stack: VecDeque::with_capacity(100),
            rows,
            cols,
        }
    }
    pub fn reset(&mut self) {
        let grid = unsafe { &mut *self.grid.get() };
        for row in grid {
            for cell in row {
                *cell = CellData::default();
            }
        }
    }

    // Unsafe mutable access
    pub unsafe fn get_cell_value(&self, row:usize,col:usize) -> &mut CellData {
        let grid = unsafe { &mut *self.grid.get() };
        &mut grid[row][col]
    }


    // pub fn get_cell_error(&self, cell: &Cell) -> CellError {
    //     self.grid.get()
    // }
    pub fn reset_found(&mut self, start: &Cell) {
        unsafe {
            let start_cell = self.get_cell_value(start.row, start.col) ;
                start_cell.dirty_parents = 0;
                let mut stack = vec![start_cell];

                while let Some(current) = stack.pop() {
                    let deps = &mut *current.dependents.get();
                    for &dep_ptr in deps.iter() {
                        if dep_ptr.is_null() {
                            continue;
                        }

                        let dep = &mut *(dep_ptr as *mut CellData);

                        if dep.dirty_parents > 0 {
                            dep.dirty_parents = 0;
                            stack.push(dep);
                        }
                    }
                }
            
        }
    }

    /// Checks if setting this cell creates a circular dependency
    pub fn check_circular_dependency(&mut self, start: &Cell) -> bool {
        let mut found_cycle = false;

        unsafe {
        let start_cell = self.get_cell_value(start.row, start.col);
        let start_cell_ptr = start_cell as *const CellData;
        let mut stack = vec![start_cell_ptr];
        unsafe { (*start_cell).dirty_parents = 1; }
        
        while let Some(current_ptr) = stack.pop() {
            let current = unsafe { &*current_ptr };
            let deps = &*current.dependents.get();
        
            // First pass: check for cycles and collect new deps to process
            let mut deps_to_check = Vec::new();
            for &dep_ptr in deps.iter() {
                if dep_ptr.is_null() {
                    continue;
                }
        
                if std::ptr::eq(dep_ptr, start_cell_ptr) {
                    found_cycle = true;
                    break;
                }
        
                deps_to_check.push(dep_ptr);
            }

                    if found_cycle {
                        break;
                    }

                    // Second pass: push unvisited deps
                    for dep_ptr in deps_to_check {
                        let dep = &mut *(dep_ptr as *mut CellData);
                        if dep.dirty_parents == 0 {
                            dep.dirty_parents = 1;
                            stack.push(dep);
                        }
                    }
                
            }
        }

        self.reset_found(start);
        found_cycle
    }


    pub fn update_graph(&mut self, cell: &Cell, old_function: &Function) {
        unsafe {
            // Remove old dependencies
            let cell_data = self.get_cell_value(cell.row,cell.col); 
                let cell_ptr = cell_data as *const CellData;
    
                match &old_function.data {
                    FunctionData::RangeFunction(range) => {
                        for row in range.top_left.row..=range.bottom_right.row {
                            for col in range.top_left.col..=range.bottom_right.col {
                              
                                 let parent_data = self.get_cell_value(row,col);
                                    let deps = &mut *parent_data.dependents.get();
                                    deps.retain(|&ptr| !std::ptr::eq(ptr, cell_ptr));
                                
                            }
                        }
                    }
    
                    FunctionData::BinaryOp(bin_op) => {
                        if let OperandData::Cell(dep) = bin_op.first.data {
                            let parent_data = self.get_cell_value(dep.row,dep.col);
                                let deps = &mut *parent_data.dependents.get();
                                deps.retain(|&ptr| !std::ptr::eq(ptr, cell_ptr));
                            
                        }
                        if let OperandData::Cell(dep) = bin_op.second.data {
                            let parent_data = self.get_cell_value(dep.row,dep.col);
                                let deps = &mut *parent_data.dependents.get();
                                deps.retain(|&ptr| !std::ptr::eq(ptr, cell_ptr));
                            
                        }
                    }
    
                    FunctionData::SleepValue(operand) => {
                        if let OperandData::Cell(dep) = operand.data {
                            let parent_data = self.get_cell_value(dep.row,dep.col);
                                let deps = &mut *parent_data.dependents.get();
                                deps.retain(|&ptr| !std::ptr::eq(ptr, cell_ptr));
                            
                        }
                    }
    
                    FunctionData::Value(_) => {} // No dependencies to remove
                
            }
    
            // Add new dependencies
            
                let cell_ptr = cell_data as *const CellData;
    
                match &cell_data.function.data {
                    FunctionData::RangeFunction(range) => {
                        for row in range.top_left.row..=range.bottom_right.row {
                            for col in range.top_left.col..=range.bottom_right.col {
                               
                                 let parent_data= self.get_cell_value(row,col) ;
                                    let deps = &mut *parent_data.dependents.get();
                                    deps.push(cell_ptr);
                                
                            }
                        }
                    }
    
                    FunctionData::BinaryOp(bin_op) => {
                        if let OperandData::Cell(dep) = bin_op.first.data {
                            let parent_data = self.get_cell_value(dep.row,dep.col);
                                let deps = &mut *parent_data.dependents.get();
                                deps.push(cell_ptr);
                            
                        }
                        if let OperandData::Cell(dep) = bin_op.second.data {
                            let parent_data = self.get_cell_value(dep.row,dep.col);
                                let deps = &mut *parent_data.dependents.get();
                                deps.push(cell_ptr);
                            
                        }
                    }
    
                    FunctionData::SleepValue(operand) => {
                        if let OperandData::Cell(dep) = operand.data {
                            let parent_data = self.get_cell_value(dep.row,dep.col);
                                let deps = &mut *parent_data.dependents.get();
                                deps.push(cell_ptr);
                            
                        }
                    }
    
                    FunctionData::Value(_) => {} // No dependencies to add
                
            }
        }
    }
    

    /// Sets dirty parent counts for topological sorting
    //check if stack has the copied values or references??
    pub fn set_dirty_parents(&mut self, cell: &Cell, stack: &mut Vec<*mut CellData>) {
        unsafe { let root_data = self.get_cell_value(cell.row,cell.col) ;
            let root_ptr = root_data as *mut CellData;
    
            
                (*root_ptr).dirty_parents = 0;
                stack.push(root_ptr);
    
                while let Some(current_ptr) = stack.pop() {
                    for &child_ptr in &*(*current_ptr).dependents.get() {
                        let child_mut_ptr = child_ptr as *mut CellData;
                        if (*child_mut_ptr).dirty_parents == 0 {
                            stack.push(child_mut_ptr);
                        }
                        (*child_mut_ptr).dirty_parents += 1;
                    }
                
            }
        }
    }
    
    

    /// Recursively update dependent cells using topological sort
    pub fn update_dependents(&mut self, cell: &Cell) {
        let mut dirty_stack = Vec::new();
        self.set_dirty_parents(cell, &mut dirty_stack);
    
        let mut process_stack = Vec::new();
    
        unsafe {  let cell_data = self.get_cell_value(cell.row,cell.col) ;{
           
                for &child_ptr in &*cell_data.dependents.get() {
                    let child_mut_ptr = child_ptr as *mut CellData;
                    (*child_mut_ptr).dirty_parents -= 1;
                    if (*child_mut_ptr).dirty_parents == 0 {
                        process_stack.push(child_mut_ptr);
                    }
                }
    
                while let Some(current_ptr) = process_stack.pop() {
                    let (new_value, error) = self.evaluate_expression(&(*current_ptr).function);
                    (*current_ptr).value = new_value;
                    (*current_ptr).error = error;
    
                    for &dependent_ptr in &*(*current_ptr).dependents.get() {
                        let dependent_mut_ptr = dependent_ptr as *mut CellData;
                        (*dependent_mut_ptr).dirty_parents -= 1;
                        if (*dependent_mut_ptr).dirty_parents == 0 {
                            process_stack.push(dependent_mut_ptr);
                        }
                    }
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
    
        // Get a mutable reference to the target cell
        unsafe { let cell_data = self
            .get_cell_value(cell.row,cell.col);

        let cell_ptr = cell_data as *mut CellData;
    
      
            // Copy old state
            let old_function = (*cell_ptr).function.clone();
            let old_value = (*cell_ptr).value;
    
            // Handle constant function early
            if new_function.type_ == FunctionType::Constant {
                let (new_value, error) = self.evaluate_expression(&new_function);
                (*cell_ptr).value = new_value;
                (*cell_ptr).error = error;
                (*cell_ptr).function = new_function;
    
                self.update_graph(&cell, &old_function);
                self.update_dependents(&cell);
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
            (*cell_ptr).function = new_function.clone();
    
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
            (*cell_ptr).value = if error == CellError::NoError { new_value } else { 0 };
            (*cell_ptr).error = error;
    
            // Propagate to dependents
            self.update_dependents(&cell);
        }
    
        Ok(())
    }
    
    pub fn min_function(&self, range: &RangeFunction) -> Result<i32, CellError> {
        let mut min_val = i32::MAX;
        for row in range.top_left.row..=range.bottom_right.row {
            for col in range.top_left.col..=range.bottom_right.col {
                let cell = Cell { row, col };
               unsafe{  let cell_data= self.get_cell_value(row,col);
                    
                    match cell_data.error {
                        CellError::NoError => {
                            min_val = min(min_val, cell_data.value);
                        }
                        CellError::DivideByZero => return Err(CellError::DivideByZero),
                        CellError::DependencyError => return Err(CellError::DependencyError),
                    }
                
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
               
                   
                unsafe{  let cell_data= self.get_cell_value(row,col);
                        match cell_data.error {
                            CellError::NoError => {
                                max_val = max(max_val, cell_data.value);
                            }
                            CellError::DivideByZero => return Err(CellError::DivideByZero),
                            CellError::DependencyError => return Err(CellError::DependencyError),
                        }
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
               
                unsafe{  let cell_data= self.get_cell_value(row,col);
                      
                        match cell_data.error {
                            CellError::NoError => {
                                sum += cell_data.value;
                                count += 1;
                            }
                            CellError::DivideByZero => return Err(CellError::DivideByZero),
                            CellError::DependencyError => return Err(CellError::DependencyError),
                        }
                    
                
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
               
                    
                        unsafe{  let cell_data= self.get_cell_value(row,col);
                   
                        match cell_data.error {
                            CellError::NoError => {
                                sum += cell_data.value;
                            }
                            CellError::DivideByZero => return Err(CellError::DivideByZero),
                            CellError::DependencyError => return Err(CellError::DependencyError),
                        }
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
               
                unsafe{ let cell_data = self.get_cell_value(row,col); 
                     
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
               unsafe{ let cell_data = self.get_cell_value(cell.row,cell.col);
                    
                

                // Check for errors in the cell
                match cell_data.error {
                    CellError::NoError => return Ok(cell_data.value),
                    CellError::DivideByZero => return Err(CellError::DivideByZero),
                    CellError::DependencyError => return Err(CellError::DependencyError),
                }
            }}
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
