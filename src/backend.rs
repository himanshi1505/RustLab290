use crate::parser::*;
use crate::structs::*;
use std::cell::UnsafeCell;
use std::cmp::{max, min};
use std::ptr::eq;
use std::time::Duration;
use std::error::Error;
use std::thread;
use std::f64;

#[cfg(feature = "gui")]
use std::collections::VecDeque;

#[cfg(feature = "gui")]
use std::fs::File;
#[cfg(feature = "gui")]
use std::io::{BufReader, BufWriter};
use std::rc::Rc;

#[cfg(feature = "gui")]


// #[cfg(feature = "gui")]
use csv::{ReaderBuilder, WriterBuilder, Writer};
// use web_sys::cell_index;
// use web_sys::HtmlTableCellElement;
// #[cfg(feature = "gui")]


#[cfg(feature = "gui")]
fn number_to_column_header(number: usize) -> String {
    let mut num = number + 1;
    let mut result = String::new();
    while num > 0 {
        let rem = (num - 1) % 26;
        result.insert(0, (b'A' + rem as u8) as char);
        num = (num - 1) / 26;
    }
    result
}

#[derive(Debug)]
pub struct Backend {
    grid: UnsafeCell<Vec<Vec<CellData>>>,
   
    rows: usize,
    cols: usize,

    #[cfg(feature = "gui")]
    pub formula_strings: Vec<Vec<String>>,
    #[cfg(feature = "gui")]
    pub filename: String,
    #[cfg(feature = "gui")]
    pub copy_stack: Vec<Vec<i32>>,
     #[cfg(feature = "gui")]
    undo_stack: VecDeque<Vec<Vec<(CellData, String)>>>,
    #[cfg(feature = "gui")]
    redo_stack: VecDeque<Vec<Vec<(CellData, String)>>>,
    
}

impl Backend {

    pub fn get_cell_dependencies(&self, row: usize, col: usize) -> (Vec<(usize, usize)>, Vec<(usize, usize)>) {
        let mut parents = Vec::new();
        let mut children = Vec::new();

        unsafe {
            let cell_data = self.get_cell_value(row, col);

            // Collect children (dependents)
            for &(child_row, child_col) in &cell_data.dependents {
                children.push((child_row as usize, child_col as usize));
            }

            // Collect parents (cells this cell depends on)
            match &cell_data.function.data {
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
            filename: "default.csv".to_string(),
            #[cfg(feature = "gui")]
            copy_stack: vec![vec![0; 1]; 1],
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
            let start_cell = self.get_cell_value(start.row, start.col);
            start_cell.dirty_parents = 0;
            let mut stack = vec![start_cell];
    
            while let Some(current) = stack.pop() {
                let deps = &current.dependents; // Access the dependents vector
                for &(row, col) in deps.iter() {
                    let dep = self.get_cell_value(row as usize, col as usize); // Access the dependent cell
    
                    if dep.dirty_parents > 0 {
                        dep.dirty_parents = 0;
                        stack.push(dep);
                    }
                }
            }
        }
    }
    pub fn check_circular_dependency(&mut self, start: &Cell) -> bool {
        let mut found_cycle = false;

        unsafe {
        let start_cell = self.get_cell_value(start.row, start.col);
        let start_cell_ptr = start_cell as *const CellData;
        let mut stack = vec![start_cell_ptr];
        unsafe { (*start_cell).dirty_parents = 1; }
        
        while let Some(current_ptr) = stack.pop() {
            let current = unsafe { &*current_ptr };
            let deps = &current.dependents;
        
            // First pass: check for cycles and collect new deps to process
            let mut deps_to_check = Vec::new();
            for &dep_ptr in deps.iter() {
             

                if dep_ptr.0==start.row as i32 && dep_ptr.1==start.col as i32{ 
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
            let cell_data = self.get_cell_value(cell.row, cell.col);
    
            match &old_function.data {
                FunctionData::RangeFunction(range) => {
                    for row in range.top_left.row..=range.bottom_right.row {
                        for col in range.top_left.col..=range.bottom_right.col {
                            let parent_data = self.get_cell_value(row, col);
                            let deps = &mut parent_data.dependents;
                            deps.retain(|&(r, c)| !(r == cell.row as i32 && c == cell.col as i32));
                        }
                    }
                }
    
                FunctionData::BinaryOp(bin_op) => {
                    if let OperandData::Cell(dep) = bin_op.first.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut parent_data.dependents;
                        deps.retain(|&(r, c)| !(r == cell.row as i32 && c == cell.col as i32));
                    }
                    if let OperandData::Cell(dep) = bin_op.second.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut parent_data.dependents;
                        deps.retain(|&(r, c)| !(r == cell.row as i32 && c == cell.col as i32));
                    }
                }
    
                FunctionData::SleepValue(operand) => {
                    if let OperandData::Cell(dep) = operand.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut parent_data.dependents;
                        deps.retain(|&(r, c)| !(r == cell.row as i32 && c == cell.col as i32));
                    }
                }
    
                FunctionData::Value(_) => {} // No dependencies to remove
            }
    
            // Add new dependencies
            match &cell_data.function.data {
                FunctionData::RangeFunction(range) => {
                    for row in range.top_left.row..=range.bottom_right.row {
                        for col in range.top_left.col..=range.bottom_right.col {
                            let parent_data = self.get_cell_value(row, col);
                            let deps = &mut parent_data.dependents;
                            deps.push((cell.row as i32, cell.col as i32));
                        }
                    }
                }
    
                FunctionData::BinaryOp(bin_op) => {
                    if let OperandData::Cell(dep) = bin_op.first.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut parent_data.dependents;
                        deps.push((cell.row as i32, cell.col as i32));
                    }
                    if let OperandData::Cell(dep) = bin_op.second.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut parent_data.dependents;
                        deps.push((cell.row as i32, cell.col as i32));
                    }
                }
    
                FunctionData::SleepValue(operand) => {
                    if let OperandData::Cell(dep) = operand.data {
                        let parent_data = self.get_cell_value(dep.row, dep.col);
                        let deps = &mut parent_data.dependents;
                        deps.push((cell.row as i32, cell.col as i32));
                    }
                }
    
                FunctionData::Value(_) => {} // No dependencies to add
            }
        }
    }

    /// Sets dirty parent counts for topological sorting
    //check if stack has the copied values or references??
    pub fn set_dirty_parents(&mut self, cell: &Cell, stack: &mut Vec<*mut CellData>) {
        unsafe {
            let root_data = self.get_cell_value(cell.row, cell.col);
            let root_ptr = root_data as *mut CellData;
    
            (*root_ptr).dirty_parents = 0;
            stack.push(root_ptr);
    
            while let Some(current_ptr) = stack.pop() {
                let current = &*current_ptr;
                let deps = &current.dependents; // Access the dependents vector
    
                for &(row, col) in deps.iter() {
                    let child_data = self.get_cell_value(row as usize, col as usize);
                    let child_ptr = child_data as *mut CellData;
    
                    if (*child_ptr).dirty_parents == 0 {
                        stack.push(child_ptr);
                    }
                    (*child_ptr).dirty_parents += 1;
                }
            }
        }
    }
    

    /// Recursively update dependent cells using topological sort
    pub fn update_dependents(&mut self, cell: &Cell) {
        let mut dirty_stack = Vec::new();
        self.set_dirty_parents(cell, &mut dirty_stack);
    
        let mut process_stack = Vec::new();
    
        unsafe {
            let cell_data = self.get_cell_value(cell.row, cell.col);
    
            // Process the dependents of the initial cell
            for &(row, col) in cell_data.dependents.iter() {
                let child_data = self.get_cell_value(row as usize, col as usize);
                child_data.dirty_parents -= 1;
                if child_data.dirty_parents == 0 {
                    process_stack.push((row as usize, col as usize));
                }
            }
    
            // Process the stack of dependent cells
            while let Some((row, col)) = process_stack.pop() {
                let current_data = self.get_cell_value(row, col);
                let (new_value, error) = self.evaluate_expression(&current_data.function);
                current_data.value = new_value;
                current_data.error = error;
    
                for &(dep_row, dep_col) in current_data.dependents.iter() {
                    let dependent_data = self.get_cell_value(dep_row as usize, dep_col as usize);
                    dependent_data.dirty_parents -= 1;
                    if dependent_data.dirty_parents == 0 {
                        process_stack.push((dep_row as usize, dep_col as usize));
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

                #[cfg(feature = "gui")]
                
               {
                self.formula_strings[cell.row][cell.col] = "=".to_owned() + &expression.to_string();
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
        #[cfg(feature = "gui")]
        
        {self.formula_strings[cell.row][cell.col] = expression.to_string();
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
    #[cfg(feature = "gui")]
    pub fn parse_load_or_save_cmd(expression: &str) -> Option<String> {
        crate::parser::parse_load_or_save_cmd(expression)
    }
    #[cfg(feature = "gui")]
    pub fn parse_cut_or_copy(&self, expression: &str) -> Result<(Cell, Cell), Box<dyn std::error::Error>> {
        crate::parser::parse_cut_or_copy(&self, expression)
    }
    #[cfg(feature = "gui")]
    pub fn parse_paste(&self, expression: &str) -> Result<Cell, Box<dyn std::error::Error>> {
        crate::parser::parse_paste(&self, expression)
    }
    #[cfg(feature = "gui")]
    pub fn parse_autofill(&self, expression: &str) -> Result<(Cell, Cell, Cell), Box<dyn std::error::Error>> {
        crate::parser::parse_autofill(&self, expression)
    }
    #[cfg(feature = "gui")]
    pub fn parse_sort(&self, expression: &str) -> Result<(Cell, Cell, bool), Box<dyn std::error::Error>> {
        crate::parser::parse_sort(&self, expression)
    }

    pub fn get_rows(&self) -> usize {
        self.rows
    }

    pub fn get_cols(&self) -> usize {
        self.cols
    }
    #[cfg(feature = "gui")]
    pub fn sort(&mut self, expression: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tup = self.parse_sort(expression);
        let (tl_cell, br_cell, a_or_d) = match tup {
            Ok((tl, br, a_or_d)) => (tl, br, a_or_d),
            Err(err) => return Err(err),
        };
        let tl = (tl_cell.row, tl_cell.col);
        let br = (br_cell.row, br_cell.col);
        let mut grid_ref = unsafe {&mut *self.grid.get()};
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
    pub fn undo_callback(&mut self) {
        if let Some(prev_state) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(self.create_snapshot());
            self.apply_snapshot(prev_state);
        }
    }

    #[cfg(feature = "gui")]
    // Redo last undone action
    pub fn redo_callback(&mut self) {
        if let Some(next_state) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(self.create_snapshot());
            self.apply_snapshot(next_state);
        }
    }

    #[cfg(feature = "gui")]
    // Helper: Create snapshot of current state
    pub fn create_snapshot(&self) -> Vec<Vec<(CellData, String)>> {
        let mut snapshot = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let mut row_data = Vec::with_capacity(self.cols);
            for col in 0..self.cols {
                unsafe{
                let cell_data = self.get_cell_value(row, col) ;
                row_data.push(
                    
                    (cell_data.clone(), self.formula_strings[row][col].clone())
                    
                );
            }
            }
            snapshot.push(row_data);
        }
        snapshot
    }

    #[cfg(feature = "gui")]
    // Helper: Apply state snapshot
    pub fn apply_snapshot(&mut self, snapshot: Vec<Vec<(CellData, String)>>) {
        for (row_idx, row) in snapshot.iter().enumerate() {
            for (col_idx, value) in row.iter().enumerate() {
                
                unsafe{
                    let cell_data = self.get_cell_value(row_idx, col_idx) ;
                    let mut cell_ptr = cell_data as *mut CellData;
                    (*cell_ptr).value = value.0.value;
                    (*cell_ptr).error = value.0.error;
                    (*cell_ptr).dependents = value.0.dependents.clone();
                    (*cell_ptr).function = value.0.function;
                    (*cell_ptr).dirty_parents = value.0.dirty_parents;
                    self.formula_strings[row_idx][col_idx] = value.1.clone();
                    // cell_ptr = *mut value;
                    // cell_data.value = value.0;
                    // cell_data.error = value.1;
                    // cell_data.dependents = value.2.clone();
                    // self.get_cell_value(row_idx, col_idx).value = value.0;
                    // self.get_cell_value(row_idx, col_idx).error = value.1;
                    // self.get_cell_value(row_idx, col_idx).dependents = value.2.clone();
                }
        }
    }
}

    #[cfg(feature = "gui")]
    // Helper: Clear undo/redo stacks
    pub fn clear_undo_redo(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    #[cfg(feature = "gui")]
    // Helper: Save current state to undo stack
    pub fn push_undo_state(&mut self) {
        if self.undo_stack.len() >= 100 {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(self.create_snapshot());
    }
    #[cfg(feature = "gui")]
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
        let v = unsafe { self.get_cell_value(tl.0, tl.1).value };
        let d = unsafe { self.get_cell_value(tl.0, tl.1).value - self.get_cell_value(tl.0 + 1, tl.1).value };
        let r  = unsafe { (self.get_cell_value(tl.0, tl.1).value as f64) / (self.get_cell_value(tl.0 + 1, tl.1).value as f64) };
        println!("v: {:?}, d: {:?}, r: {:?}", v, d, r);
        println!("tl_value: {:?}, br_value: {:?}", unsafe { self.get_cell_value(tl.0, tl.1).value }, unsafe { self.get_cell_value(tl.0 + 1, tl.1).value });
        let mut is_constant = true;
        let mut is_ap = true;
        let mut is_gp = true;
        let grid_ref = unsafe {&*self.grid.get()};
        println!("im hereee");
        for row in tl.0..=br.0 {
            for col in tl.1..=br.1 {
                if grid_ref[row][col].value != v {
                    is_constant = false;
                    break;
                }
            }
        }
        println!("is constant: {:?}", is_constant);
        if is_constant {
            println!("is constant");
            for row in br.0+1..=dest.0 {
                for col in br.1..=dest.1 {
                    let cell = Cell { row, col };
                    let res = self.set_cell_value(cell, v.to_string().as_str());
                    if let Err(err) = res {
                        println!("Error autofilling value: {:?}", err);
                    }
                }
            }
            return Ok(());
        } else {
            for row in tl.0..br.0 {
                for col in tl.1..=br.1 {
                    if (grid_ref[row][col].value as f64) / (grid_ref[row + 1][col].value as f64) != r {
                        is_gp = false;
                        break;
                    }
                }
            }
            print!("is gp: {:?}", is_gp);
            if is_gp {
                println!("is gp");
                for row in br.0+1..=dest.0 {
                    for col in br.1..=dest.1 {
                        let cell = Cell { row, col };
                        let res = self.set_cell_value(cell, &((grid_ref[row - 1][col].value as f64 / r) as i32).to_string());
                        if let Err(err) = res {
                            println!("Error autofilling value: {:?}", err);
                        }
                    }
                }
                return Ok(());
            }
            else {
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
                    for row in br.0+1..=dest.0 {
                        for col in br.1..=dest.1 {
                            let cell = Cell { row, col };
                            let res = self.set_cell_value(cell, &(grid_ref[row - 1][col].value - d).to_string());
                            if let Err(err) = res {
                                println!("Error autofilling value: {:?}", err);
                            }
                        }
                    }
                    return Ok(());
                }
                else {
                    return Err("Autofill not possible".to_string().into());
                }
            }
        }
    }
    #[cfg(feature = "gui")]
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
                row_data.push(unsafe { self.get_cell_value(row, col).value });
            }
            copied_data.push(row_data);
        }
        self.copy_stack = copied_data;
        Ok(())
    }
    #[cfg(feature = "gui")]
    pub fn paste(&mut self, expression: &str) -> Result<(), Box<dyn std::error::Error>> {
        let celll = self.parse_paste(expression);
        let tl_cell = match celll {
            Ok(tl) => tl,
            Err(err) => return Err(err),
        };
        let tl = (tl_cell.row, tl_cell.col);
        // println!("tl: {:?}", tl);
        let br = (tl.0 + self.copy_stack.len() - 1, tl.1 + self.copy_stack[0].len() - 1);
        // println!("br: {:?}", br);

        if br.0 >= self.rows || br.1 >= self.cols {
            return Err("Paste area exceeds grid size".to_string().into());
        }
        for row in tl.0..=br.0 {
            for col in tl.1..=br.1 {
                if row < self.rows && col < self.cols {
                    let cell = Cell { row, col };
                    // println!("row: {:?}, col: {:?}", row, col);
                    let res = self.set_cell_value(cell, &self.copy_stack[row - tl.0][col - tl.1].to_string());
                    let col_header = 
                    self.formula_strings[row][col] = self.copy_stack[row - tl.0][col - tl.1].to_string();
                    // unsafe {(*self.grid.get().wrapping_add(row).wrapping_add(col)).value = self.copy_stack[row - tl.0][col - tl.1];}
                    // unsafe {let cell = self.get_cell_value(row, col);
                    // cell.value = self.copy_stack[row - tl.0][col - tl.1];}
                    // if let Err(err) = res {
                    //     println!("Error pasting value: {:?}", err);
                    // }
                }
            }
        }
        Ok(())
    }
    #[cfg(feature = "gui")]
    pub fn save_to_csv(&self, save_cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
        let filename = match crate::backend::Backend::parse_load_or_save_cmd(save_cmd) {
            Some(path) => path,
            None => return Err("Invalid load command".to_string().into()),
        };
        let file = File::create(filename)?;
        let mut wtr = WriterBuilder::new().from_writer(BufWriter::new(file));
        let grid_ref = self.formula_strings.clone();
        for row in 0..self.rows {
            let mut record = Vec::new();
            for col in 0..self.cols {
                unsafe {record.push(self.get_cell_value(row, col).value.to_string())};
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
    pub fn load_csv(
        &mut self,
        load_cmd: &str,
        is_header_present: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let csv_path = match crate::backend::Backend::parse_load_or_save_cmd(load_cmd) {
            Some(path) => path,
            None => return Err("Invalid load command".to_string().into()),
        };
        let reader_result = ReaderBuilder::new().has_headers(is_header_present).from_path(csv_path);
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
        let no_of_cols = csv_data
            .get(0)
            .map_or(0, |row| row.len());
        *self = Backend::new(no_of_rows, no_of_cols);
        self.get_rows_col().0 = no_of_rows;
        self.get_rows_col().1 = no_of_cols;
        // println!("Rows: {}, Cols: {}", self.get_rows_col().0, self.get_rows_col().1);

        for (row_idx, row) in csv_data.iter().enumerate() {
            for (col_idx, field) in row.iter().enumerate() {
                if row_idx < self.rows && col_idx < self.cols {
                    let cell = Cell { row: row_idx, col: col_idx };
                    let res =  self.set_cell_value(cell, field);
                    if let Err(err) = res {
                        return Err("Invalid cell value".to_string().into());
                    }
                }
            }
        }

        Ok(())
    }
    #[cfg(feature = "gui")]
    pub fn load_csv_string(&mut self, csv_data: &str, is_header_present: bool) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = ReaderBuilder::new()
            .has_headers(is_header_present)
            .from_reader(csv_data.as_bytes());
    
        let mut rows: Vec<Vec<String>> = Vec::new();
        for result in reader.records() {
            let record = result?;
            rows.push(record.iter().map(|s| s.trim().to_string()).collect());
        }
    
        let num_rows = rows.len();
        let num_cols = rows.get(0).map_or(0, |r| r.len());
        *self = Backend::new(num_rows, num_cols);
        self.get_rows_col().0 = num_rows;
        self.get_rows_col().1 = num_cols;
    
        for (row_idx, row) in rows.iter().enumerate() {
            for (col_idx, val) in row.iter().enumerate() {
                let cell = Cell { row: row_idx, col: col_idx };
                self.set_cell_value(cell, val);
            }
        }
    
        Ok(())
    }
    #[cfg(feature = "gui")]
    pub fn to_csv_string(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut writer = WriterBuilder::new().from_writer(Vec::new());
        
        for row in 0..self.rows {
            let mut record = Vec::new();
            for col in 0..self.cols {
                unsafe {
                    record.push(self.get_cell_value(row, col).value.to_string());
                }
            }
            writer.write_record(&record)?;
        }
        
        Ok(String::from_utf8(writer.into_inner()?)?)
    }
    #[cfg(feature = "gui")]
    pub fn load_csv_from_str(&mut self, data: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(data.as_bytes());
        
        let mut csv_data: Vec<Vec<String>> = Vec::new();
    
        for record in rdr.records() {
            let record = record?;
            let row: Vec<String> = record.iter()
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
                    let cell = Cell { row: row_idx, col: col_idx };
                    self.set_cell_value(cell, field);
                }
            }
        }
        
        Ok(())
    }
    #[cfg(feature = "gui")]
    // #[server]
    pub fn save_to_csv_internal(&self) -> Result<(), Box<dyn Error>> {
        let filename = self.filename.clone();
        let mut wtr = Writer::from_path(filename)?;
        let grid_ref = self.formula_strings.clone();
        for row in 0..self.rows {
            let mut record = Vec::new();
            for col in 0..self.cols {
                record.push(grid_ref[row][col].clone());
            }
            wtr.write_record(&record)?;
        }
        wtr.flush()?;
        Ok(())
    }

    #[cfg(feature = "gui")]
    // #[server]
    pub fn save_as_internal(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        self.filename = filename.to_string();
        self.save_to_csv_internal()
    }
}



    // #[cfg(feature = "gui")]
    // // Save to CSV file
    // pub fn save_to_csv(&self, filename: &str) -> Result<(), csv::Error> {
    //     let file = File::create(filename)?;
    //     let mut wtr = WriterBuilder::new().from_writer(BufWriter::new(file));

    //     for row in 0..self.rows {
    //         let mut record = Vec::new();
    //         for col in 0..self.cols {
    //             let cell = Cell { row, col };
    //             if let Some(cell_data) = self.get_cell_value(&cell) {
    //                 record.push(cell_data.borrow().value.to_string());
    //             } else {
    //                 record.push(String::new());
    //             }
    //         }
    //         wtr.write_record(&record)?;
    //     }
    //     wtr.flush()?;
    //     Ok(())
    // }

    // #[cfg(feature = "gui")]
    // // Load from CSV file
    // pub fn load_from_csv(&mut self, filename: &str) -> Result<(), csv::Error> {
    //     let file = File::open(filename)?;
    //     let mut rdr = ReaderBuilder::new()
    //         .has_headers(false)
    //         .from_reader(BufReader::new(file));

    //     self.clear_undo_redo();
    //     let mut new_state = self.create_snapshot();

    //     for (row_idx, result) in rdr.records().enumerate() {
    //         let record = result?;
    //         for (col_idx, field) in record.iter().enumerate() {
    //             if row_idx < self.rows && col_idx < self.cols {
    //                 let value = field.parse().unwrap_or(0);
    //                 new_state[row_idx][col_idx] = value;
    //             }
    //         }
    //     }

    //     self.push_undo_state();
    //     self.apply_snapshot(new_state);
    //     Ok(())
    // }

    // #[cfg(feature = "gui")]
    // // Undo last action
    // pub fn undo(&mut self) {
    //     if let Some(prev_state) = self.undo_stack.pop_back() {
    //         self.redo_stack.push_back(self.create_snapshot());
    //         self.apply_snapshot(prev_state);
    //     }
    // }

    // #[cfg(feature = "gui")]
    // // Redo last undone action
    // pub fn redo(&mut self) {
    //     if let Some(next_state) = self.redo_stack.pop_back() {
    //         self.undo_stack.push_back(self.create_snapshot());
    //         self.apply_snapshot(next_state);
    //     }
    // }

    // #[cfg(feature = "gui")]
    // // Helper: Create snapshot of current state
    // fn create_snapshot(&self) -> Vec<Vec<i32>> {
    //     let mut snapshot = Vec::with_capacity(self.rows);
    //     for row in 0..self.rows {
    //         let mut row_data = Vec::with_capacity(self.cols);
    //         for col in 0..self.cols {
    //             let cell = Cell { row, col };
    //             row_data.push(
    //                 self.get_cell_value(&cell)
    //                     .map(|c| c.borrow().value)
    //                     .unwrap_or(0),
    //             );
    //         }
    //         snapshot.push(row_data);
    //     }
    //     snapshot
    // }

    // #[cfg(feature = "gui")]
    // // Helper: Apply state snapshot
    // fn apply_snapshot(&mut self, snapshot: Vec<Vec<i32>>) {
    //     for (row_idx, row) in snapshot.iter().enumerate() {
    //         for (col_idx, &value) in row.iter().enumerate() {
    //             let cell = Cell {
    //                 row: row_idx,
    //                 col: col_idx,
    //             };
    //             if let Some(cell_data) = self.get_cell_value(&cell) {
    //                 cell_data.borrow_mut().value = value;
    //             }
    //         }
    //     }
    // }

    // #[cfg(feature = "gui")]
    // // Helper: Clear undo/redo stacks
    // fn clear_undo_redo(&mut self) {
    //     self.undo_stack.clear();
    //     self.redo_stack.clear();
    // }

    // #[cfg(feature = "gui")]
    // // Helper: Save current state to undo stack
    // fn push_undo_state(&mut self) {
    //     if self.undo_stack.len() >= 100 {
    //         self.undo_stack.pop_front();
    //     }
    //     self.undo_stack.push_back(self.create_snapshot());
    // }


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
