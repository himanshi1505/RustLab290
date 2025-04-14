use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellError {
    NoError,
    DivideByZero,
    DependencyError, // depends on cell which has div by zero
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionError {
    CouldNotParse,
    CircularDependency,
    NoError,
}

//checked

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandType {
    Cell,
    Int,
}
//checked

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandData {
    Cell(Cell),
    Value(i32),
}
//checked

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Operand {
    pub type_: OperandType,
    pub data: OperandData,
}
//checked


// impl Operand {
//     pub fn new_cell(cell: Cell) -> Self {
//         Operand {
//             type_: OperandType::Cell,
//             cell: Some(cell),
//             value: None,
//         }
//     }

//     pub fn new_int(value: i32) -> Self {
//         Operand {
//             type_: OperandType::Int,
//             cell: None,
//             value: Some(value),
//         }
//     }
// }
//not currently required 

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinaryOp {
    pub first: Operand,
    pub second: Operand,
}
//checked

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeFunction {
    pub top_left: Cell,
    pub bottom_right: Cell,
}
//checked

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
//checked

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionData {
    /// Used for MinFunction, MaxFunction, AvgFunction, SumFunction, StdevFunction
    RangeFunction(RangeFunction),

    /// Used for PlusOp, MinusOp, MultiplyOp, DivideOp
    BinaryOp(BinaryOp),

    /// Used for SleepFunction
    SleepValue(Operand),

    /// Used for Constant
    Value(i32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Function {
    pub type_: FunctionType,
    pub data: FunctionData,
}
//checked

impl Function {
    //what to do when it matches with none
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
            data: FunctionData::RangeFunction(range),
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
            data: FunctionData::BinaryOp(op),
        }
    }

    pub fn new_constant(value: i32) -> Self {
        Function {
            type_: FunctionType::Constant,
            data: FunctionData::Value(value),
        }
    }

    pub fn new_sleep(value: i32) -> Self {
        Function {
            type_: FunctionType::Sleep,
            data: FunctionData::SleepValue(Operand {
                type_: OperandType::Int,
                data: OperandData::Value(value),
            }),
        }
    }
}

#[derive(Debug)]
pub struct CellData {
    pub value: i32,
    pub dependents: Vec<Rc<RefCell<CellData>>>,
    pub function: Function,
    pub error: CellError,
    pub dirty_parents: i32,
}
impl Default for CellData{
    fn default() -> Self {
        CellData {
            value: 0,
            dependents: Vec::new(),
            function: Function {
                type_: FunctionType::Constant,
                data: FunctionData::Value(0),
            },
            error: CellError::NoError,
            dirty_parents: 0,
        }
    }
}


