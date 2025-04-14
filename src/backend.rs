use crate::structs::*;
use crate::parser::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::thread;
use std::time::Duration;
use std::cmp::{min, max};
use std::f64;

pub struct Backend {
    pub grid: Vec<Vec<Rc<RefCell<CellData>>>>,
    rows: usize,
    cols: usize,
}

impl Backend {
    pub fn new(rows: usize, cols: usize) -> Self {  //init backend
        let mut grid = Vec::new();
    
        for _ in 0..rows {
            let mut row = Vec::new();
            for _ in 0..cols {
                row.push(Rc::new(RefCell::new(CellData::default())));
            }
            grid.push(row);
        }
    
        Backend { grid, rows, cols }
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
    

    pub fn reset_found(&mut self, start: &Cell) {
        if let Some(cell_data_rc) = self.get_cell_value(start) {
            let mut cell_data = cell_data_rc.borrow_mut();
            cell_data.dirty_parents = 0;
            
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
        let mut stack =vec![Rc::clone(cell_data_rc)];
        let mut cell_data = cell_data_rc.borrow_mut();
        cell_data.dirty_parents = 1;
    

    while let Some(current_rc) = stack.pop() {
        // Get all dependents of current cell
      
            let current_cell_data = current_rc.borrow();
            // To avoid borrow checker issues, collect cells to process first
            let mut deps_to_check = Vec::new();
            let mut cycle_detected = false;
            
            // First pass: check for cycles and collect cells to process
            for dep in &current_cell_data.dependents {
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

    
pub fn check_circular_dependency(&mut self,  cell: &Cell) -> bool{
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
                            parent_data.dependents.retain(|c| !Rc::ptr_eq(c, cell_data_rc));
                        }
                    }
                }

            }
            FunctionData::BinaryOp(bin_op) => {
                // Remove first operand dependency
                if let OperandData::Cell(dep) = bin_op.first.data {
                    if let Some(parent_data_rc) = self.get_cell_value(&dep) {
                        let mut parent_data = parent_data_rc.borrow_mut();
                        parent_data.dependents.retain(|c| !Rc::ptr_eq(c, cell_data_rc));
                    }
                }
                // Remove second operand dependency
                if let OperandData::Cell(dep) = bin_op.second.data {
                    if let Some(parent_data_rc) = self.get_cell_value(&dep) {
                        let mut parent_data = parent_data_rc.borrow_mut();
                        parent_data.dependents.retain(|c| !Rc::ptr_eq(c, cell_data_rc));
                    }
                }
            }
            FunctionData::SleepValue(operand) => {
                if let OperandData::Cell(dep) = operand.data {
                    if let Some(parent_data_rc) = self.get_cell_value(&dep) {
                        let mut parent_data = parent_data_rc.borrow_mut();
                        parent_data.dependents.retain(|c| !Rc::ptr_eq(c, cell_data_rc));
                    }
                }
            }
            FunctionData::Value(_) => {} // Constants have no dependencies
        }}

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
    

    while let Some(current_rc) = stack.pop() {
       
            let mut current = current_rc.borrow_mut();
            
            for child_data_rc in &current.dependents {
               
                    let mut child_data = child_data_rc.borrow_mut();
                    if (child_data.dirty_parents) == 0 {
                       stack.push (Rc::clone(child_data_rc));
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
            FunctionType::Plus | 
            FunctionType::Minus | 
            FunctionType::Multiply | 
            FunctionType::Divide => {
                if let FunctionData::BinaryOp(bin_op) = func.data {
                    matches!(bin_op.first.type_, OperandType::Int) &&
                    matches!(bin_op.second.type_, OperandType::Int)
                } else {
                    false
                }
            }
            FunctionType::Constant => true,
            _ => false
        }
    }

  
   /// Evaluates a function and returns (value, error)
   pub fn evaluate_expression(&self, func: &Function) -> (i32, CellError) {
    match func.data {
        FunctionData::BinaryOp(bin_op) => {
            match func.type_ {
                FunctionType::Plus => {
                    match self.plus_op(&bin_op) {
                        Ok(value) => (value, CellError::NoError),
                        Err(error) => (0, error)
                    }
                },
                FunctionType::Minus => {
                    match self.minus_op(&bin_op) {
                        Ok(value) => (value, CellError::NoError),
                        Err(error) => (0, error)
                    }
                },
                FunctionType::Multiply => {
                    match self.multiply_op(&bin_op) {
                        Ok(value) => (value, CellError::NoError),
                        Err(error) => (0, error)
                    }
                },
                FunctionType::Divide => {
                    match self.divide_op(&bin_op) {
                        Ok(value) => (value, CellError::NoError),
                        Err(error) => (0, error)
                    }
                },
                _ => (0, CellError::DependencyError),
            }
        }
        FunctionData::RangeFunction(range) => {
            match func.type_ {
                FunctionType::Min => {
                    match self.min_function(&range) {
                        Ok(value) => (value, CellError::NoError),
                        Err(error) => (0, error)
                    }
                },
                FunctionType::Max => {
                    match self.max_function(&range) {
                        Ok(value) => (value, CellError::NoError),
                        Err(error) => (0, error)
                    }
                },
                FunctionType::Avg => {
                    match self.avg_function(&range) {
                        Ok(value) => (value, CellError::NoError),
                        Err(error) => (0, error)
                    }
                },
                FunctionType::Sum => {
                    match self.sum_function(&range) {
                        Ok(value) => (value, CellError::NoError),
                        Err(error) => (0, error)
                    }
                },
                FunctionType::Stdev => {
                    match self.stdev_function(&range) {
                        Ok(value) => (value, CellError::NoError),
                        Err(error) => (0, error)
                    }
                },
                _ => (0, CellError::DependencyError),
            }
        }
        FunctionData::SleepValue(operand) => {
            match self.sleep_function(&operand) {
                Ok(value) => (value, CellError::NoError),
                Err(error) => (0, error)
            }
        }
        FunctionData::Value(value) => {
            (value, CellError::NoError)
        }
    }
}
pub fn set_cell_value(
    &mut self,
    cell: Cell,
    expression: &str,
) -> Result<(), ExpressionError> {
    // Parse the expression
    let (new_function, success) = self.parse_expression(expression);
    if !success {
        return Err(ExpressionError::CouldNotParse);
    }

    // Get cell data
    let cell_data_rc = self.get_cell_value(&cell).ok_or(ExpressionError::CouldNotParse)?;
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
        }
        
        self.update_graph(&cell, &old_function);
        self.update_dependents(&cell);
        return Ok(());
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
            cell_data.function = old_function;  // No need to clone again
        }
        
        self.update_graph(&cell, &new_function);
        return Err(ExpressionError::CircularDependency);
    }

    // Evaluate new value
    let (new_value, error) = self.evaluate_expression(&new_function);
    
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
                if let Some(cell_data_rc) = self.get_cell_value(&cell){
                    let cell_data = cell_data_rc.borrow();
                    min_val = min(min_val, cell_data.value);
                }
                else{
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
                if let Some(cell_data_rc) = self.get_cell_value(&cell){
                    let cell_data = cell_data_rc.borrow();
                    max_val = max(max_val, cell_data.value);
                }
                else{
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
                if let Some(cell_data_rc) = self.get_cell_value(&cell){
                    let cell_data = cell_data_rc.borrow();
                    sum += cell_data.value;
                    count += 1;
                }
                else{
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
                if let Some(cell_data_rc) = self.get_cell_value(&cell){
                    let cell_data = cell_data_rc.borrow();
                    sum += cell_data.value;
                    
                }
                else{
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
                    let value = cell_data.value;
                    values.push(value);
                    sum += value;
                    count += 1;
                } else {
                    return Err(CellError::DependencyError);
                }
            }
        }
        
        if count == 0 {
            return Err(CellError::DivideByZero);
        }
        
        // Calculate mean
        let mean = sum / count as i32;
        
        // Second pass: calculate variance
        let mut variance_sum = 0;
        for value in values {
            variance_sum += (value - mean).pow(2);
        }
        
        let variance = variance_sum / count as i32;
        
        // Return standard deviation as integer (floored)
        Ok((variance as f64).sqrt() as i32)
    }

    pub fn sleep_function(&self, operand: &Operand) -> Result<i32, CellError> {
        let value = self.get_operand_value(operand)?;
        thread::sleep(Duration::from_secs(value as u64));
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
            OperandData::Cell(cell) => self.get_cell_value(&cell)
                .ok_or(CellError::DependencyError)
                .map(|d| d.borrow().value),
            OperandData::Value(value) => Ok(value),
        }
    }

    pub fn parse_expression(&self, expression: &str) -> (Function, bool) {
        (crate::parser::parse_expression(expression), true)
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
