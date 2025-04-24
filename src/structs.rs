//! # Spreadsheet Structs Module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Represents a cell in a spreadsheet with row and column indices.
pub struct Cell {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// CellError represents the possible errors that can occur in a cell.
pub enum CellError {
    NoError,
    DivideByZero,
    DependencyError, // depends on cell which has div by zero
}
/// Represents the possible errors that can occur during expression parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionError {
    CouldNotParse,
    CircularDependency,
}


///Represents possible operand types: Cell or Int.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandType {
    Cell,
    Int,
}

/// OperandData represents the data contained in an operand, which can be either a Cell or an integer value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandData {
    Cell(Cell),
    Value(i32),
}
/// Operand represents a single operand in an expression, stores it type and data.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Operand {
    pub type_: OperandType,
    pub data: OperandData,
}
/// BinaryOp represents a binary operation between two operands and stores them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinaryOp {
    pub first: Operand,
    pub second: Operand,
}

/// RangeFunction represents a range of cells in a spreadsheet, defined by its top-left and bottom-right corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeFunction {
    pub top_left: Cell,
    pub bottom_right: Cell,
}

/// FunctionType represents the type of function being used in a cell, such as Min, Max, Avg, etc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionType {
    Constant,
    Min,
    Max,
    Avg,
    Sum,
    Stdev,
    Sleep,
    Plus, // Identity function can be written as A1+0
    Minus,
    Multiply,
    Divide,
}
/// FunctionData represents the data associated with a function, which can be a range of cells, a binary operation, sleep value or a constant value.

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
/// Function represents a function in a cell, stores its type and data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Function {
    pub type_: FunctionType,
    pub data: FunctionData,
}

/// Function methods
impl Function {
    ///Creates a new range Function instance with the given type and data.
    pub fn new_range_function(type_: FunctionType, range: RangeFunction) -> Self {
        assert!(matches!(
            type_,
            FunctionType::Min
                | FunctionType::Max
                | FunctionType::Avg
                | FunctionType::Sum
                | FunctionType::Stdev
        ));

        Function {
            type_,
            data: FunctionData::RangeFunction(range),
        }
    }
     ///Creates a new binary Function instance with the given type and data.
    pub fn new_binary_op(type_: FunctionType, op: BinaryOp) -> Self {
        assert!(matches!(
            type_,
            FunctionType::Plus
                | FunctionType::Minus
                | FunctionType::Multiply
                | FunctionType::Divide
        ));

        Function {
            type_,
            data: FunctionData::BinaryOp(op),
        }
    }
    /// Creates a new constant Function instance with the given type and data.
    pub fn new_constant(value: i32) -> Self {
        Function {
            type_: FunctionType::Constant,
            data: FunctionData::Value(value),
        }
    }
    /// Creates a new sleep Function instance with the given type and data.
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
/// CellData represents the data associated with a cell in a spreadsheet, including its value, dependents, function, error state, and dirty parents count.
#[derive(Debug, Clone)]
pub struct CellData {
    pub value: i32,
    pub dependents: Vec<(i32, i32)>,
    pub function: Function,
    pub error: CellError,
    pub dirty_parents: i32,
}
/// CellData methods
impl Default for CellData {
    /// Creates a new CellData instance with default values.
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
