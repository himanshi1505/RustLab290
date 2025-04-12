use std::thread;
use std::time::Duration;
use std::cmp::{min, max};
use std::f64;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub row: i32,
    pub col: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellError {
    NoError,
    DivideByZero,
    DependencyError, // depends on cell which has div by zero
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandType {
    Cell,
    Int,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Operand {
    pub type_: OperandType,
    pub cell: Option<Cell>,
    pub value: Option<i32>,
}

impl Operand {
    pub fn new_cell(cell: Cell) -> Self {
        Operand {
            type_: OperandType::Cell,
            cell: Some(cell),
            value: None,
        }
    }

    pub fn new_int(value: i32) -> Self {
        Operand {
            type_: OperandType::Int,
            cell: None,
            value: Some(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinaryOp {
    pub first: Operand,
    pub second: Operand,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeFunction {
    pub top_left: Cell,
    pub bottom_right: Cell,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionType {
    Constant,
    Min,
    Max,
    Avg,
    Sum,
    Stdev,
    Sleep,
    Plus,    // Identity function can be written as A1+0
    Minus,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Function {
    pub type_: FunctionType,
    // Using Option since Rust doesn't have unions like C
    pub range_function: Option<RangeFunction>,
    pub binary_op: Option<BinaryOp>,
    pub value: Option<i32>,
}

impl Function {
    pub fn new_range_function(type_: FunctionType, range: RangeFunction) -> Self {
        assert!(matches!(type_, 
            FunctionType::Min | 
            FunctionType::Max | 
            FunctionType::Avg | 
            FunctionType::Sum | 
            FunctionType::Stdev
        ));
        
        Function {
            type_,
            range_function: Some(range),
            binary_op: None,
            value: None,
        }
    }

    pub fn new_binary_op(type_: FunctionType, op: BinaryOp) -> Self {
        assert!(matches!(type_,
            FunctionType::Plus |
            FunctionType::Minus |
            FunctionType::Multiply |
            FunctionType::Divide
        ));
        
        Function {
            type_,
            range_function: None,
            binary_op: Some(op),
            value: None,
        }
    }

    pub fn new_constant(value: i32) -> Self {
        Function {
            type_: FunctionType::Constant,
            range_function: None,
            binary_op: None,
            value: Some(value),
        }
    }

    pub fn new_sleep(value: i32) -> Self {
        Function {
            type_: FunctionType::Sleep,
            range_function: None,
            binary_op: None,
            value: Some(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CellData {
    /// The value of the cell
    pub value: i32,

    /// Cells that depend on this one
    pub dependents: Vec<Cell>,

    /// Cells this one depends on
    pub function: Function,

    pub error: CellError,

    /// Number of parent cells that must be updated before this one
    pub dirty_parents: i32,

    /// Useful for DFS
    pub visited: bool,
}


// Function implementations

pub fn min_function(cells: &Vec<Vec<CellData>>, range_function: RangeFunction) -> Result<i32, bool> {
    let mut min_val = i32::MAX;
    
    for i in range_function.top_left.row..=range_function.bottom_right.row {
        for j in range_function.top_left.col..=range_function.bottom_right.col {
            if cells[i as usize][j as usize].error != CellError::NoError {
                return Err(true);
            }
            min_val = min(min_val, cells[i as usize][j as usize].value);
        }
    }
    
    Ok(min_val)
}

pub fn max_function(cells: &Vec<Vec<CellData>>, range_function: RangeFunction) -> Result<i32, bool> {
    let mut max_val = i32::MIN;
    
    for i in range_function.top_left.row..=range_function.bottom_right.row {
        for j in range_function.top_left.col..=range_function.bottom_right.col {
            if cells[i as usize][j as usize].error != CellError::NoError {
                return Err(true);
            }
            max_val = max(max_val, cells[i as usize][j as usize].value);
        }
    }
    
    Ok(max_val)
}

pub fn avg_function(cells: &Vec<Vec<CellData>>, range_function: RangeFunction) -> Result<i32, bool> {
    let mut sum = 0;
    let mut count = 0;
    
    for i in range_function.top_left.row..=range_function.bottom_right.row {
        for j in range_function.top_left.col..=range_function.bottom_right.col {
            if cells[i as usize][j as usize].error != CellError::NoError {
                return Err(true);
            }
            sum += cells[i as usize][j as usize].value;
            count += 1;
        }
    }
    
    Ok(sum / count)
}

pub fn sum_function(cells: &Vec<Vec<CellData>>, range_function: RangeFunction) -> Result<i32, bool> {
    let mut sum = 0;
    
    for i in range_function.top_left.row..=range_function.bottom_right.row {
        for j in range_function.top_left.col..=range_function.bottom_right.col {
            if cells[i as usize][j as usize].error != CellError::NoError {
                return Err(true);
            }
            sum += cells[i as usize][j as usize].value;
        }
    }
    
    Ok(sum)
}

pub fn stdev_function(cells: &Vec<Vec<CellData>>, range_function: RangeFunction) -> Result<i32, bool> {
    let mut sum = 0;
    let mut count = 0;
    
    // First pass: calculate mean
    for i in range_function.top_left.row..=range_function.bottom_right.row {
        for j in range_function.top_left.col..=range_function.bottom_right.col {
            if cells[i as usize][j as usize].error != CellError::NoError {
                return Err(true);
            }
            sum += cells[i as usize][j as usize].value;
            count += 1;
        }
    }
    
    let mean = sum / count;
    
    // Second pass: calculate sum of squares
    let mut sum_of_squares = 0;
    for i in range_function.top_left.row..=range_function.bottom_right.row {
        for j in range_function.top_left.col..=range_function.bottom_right.col {
            let diff = cells[i as usize][j as usize].value - mean;
            sum_of_squares += diff * diff;
        }
    }
    
    // Calculate standard deviation
    let variance = sum_of_squares / count;
    let std_dev = (variance as f64).sqrt() as i32;
    
    Ok(std_dev)
}

pub fn sleep_function(sleep_value: i32) -> i32 {
    thread::sleep(Duration::from_secs(sleep_value as u64));
    sleep_value
}

fn get_operand_value(cells: &Vec<Vec<CellData>>, operand: &BinaryOp, is_first: bool) -> Result<i32, bool> {
    let op = if is_first { &operand.first } else { &operand.second };
    
    match op.type_ {
        OperandType::Cell => {
            let cell = op.cell.unwrap();
            if cells[cell.row as usize][cell.col as usize].error != CellError::NoError {
                return Err(true);
            }
            Ok(cells[cell.row as usize][cell.col as usize].value)
        },
        OperandType::Int => Ok(op.value.unwrap()),
    }
}

pub fn plus_op(cells: &Vec<Vec<CellData>>, binary_op: BinaryOp) -> Result<i32, bool> {
    let first = get_operand_value(cells, &binary_op, true)?;
    let second = get_operand_value(cells, &binary_op, false)?;
    
    Ok(first + second)
}

pub fn minus_op(cells: &Vec<Vec<CellData>>, binary_op: BinaryOp) -> Result<i32, bool> {
    let first = get_operand_value(cells, &binary_op, true)?;
    let second = get_operand_value(cells, &binary_op, false)?;
    
    Ok(first - second)
}

pub fn multiply_op(cells: &Vec<Vec<CellData>>, binary_op: BinaryOp) -> Result<i32, bool> {
    let first = get_operand_value(cells, &binary_op, true)?;
    let second = get_operand_value(cells, &binary_op, false)?;
    
    Ok(first * second)
}

pub fn divide_op(cells: &Vec<Vec<CellData>>, binary_op: BinaryOp) -> Result<i32, bool> {
    let first = get_operand_value(cells, &binary_op, true)?;
    let second = get_operand_value(cells, &binary_op, false)?;
    
    if second == 0 {
        return Err(true);
    }
    
    Ok(first / second)
}
