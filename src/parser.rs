//! # Spreadsheet Parser Module
use crate::backend::Backend;
use crate::structs::*;

#[cfg(feature = "gui")]
/// Parses a command to load or save a file.
pub fn parse_load_or_save_cmd(expression: &str) -> Option<String> {
    let start_pos = 5; // "LOAD("
    let content = &expression[start_pos..];
    let end_pos = content.find(')')?;
    let file_name = &content[..end_pos];

    if file_name.is_empty() {
        return None;
    }

    Some(file_name.to_string())
}
#[cfg(feature = "gui")]
/// Parses a command to sort a range of cells.
pub fn parse_sort(
    backend: &Backend,
    expression: &str,
) -> Result<(Cell, Cell, bool), Box<dyn std::error::Error>> {
    // println!("Parsing sort command: {}", expression);
    let start_pos = 6; // "SORTA( or SORTD("
    let a_or_d; // true for ascending, false for descending
    let posi: &str = &expression[4_usize..5_usize];
    // println!("{}", posi);
    if posi == "a" {
        a_or_d = true;
    } else if posi == "d" {
        a_or_d = false;
    } else {
        // println!("error");
        return Err("Invalid command".to_string().into());
    }
    let content = &expression[start_pos..];
    let end_pos = match content.find(')') {
        Some(pos) => pos,
        None => return Err("Invalid command".to_string().into()),
    };
    let range_str = &content[..end_pos];

    if let Some(separator_pos) = range_str.find(':') {
        let top_left_str = &range_str[..separator_pos];
        let bottom_right_str = &range_str[separator_pos + 1..];

        let top_left =
            match parse_cell_reference(top_left_str, backend.get_rows(), backend.get_cols()) {
                Some(cell) => cell,
                None => return Err("Invalid cell reference".to_string().into()),
            };
        let bottom_right =
            match parse_cell_reference(bottom_right_str, backend.get_rows(), backend.get_cols()) {
                Some(cell) => cell,
                None => return Err("Invalid cell reference".to_string().into()),
            };

        // Check if range is valid (top_left <= bottom_right)
        if top_left.row > bottom_right.row || top_left.col != bottom_right.col {
            return Err("Invalid range".to_string().into());
        }

        return Ok((top_left, bottom_right, a_or_d));
    }

    Err("Invalid command".to_string().into())
}
/// Parses a cell reference from a string and returns a Cell struct.
pub fn parse_cell_reference(reference: &str, rows: usize, cols: usize) -> Option<Cell> {
    let mut cell = Cell { row: 0, col: 0 };
    let chars: Vec<char> = reference.chars().collect();
    let mut i = 0;

    // Must start with a letter
    if chars.is_empty() || !chars[0].is_ascii_uppercase() {
        return None;
    }

    // Parse column (letters)
    while i < chars.len() && chars[i].is_ascii_uppercase() {
        cell.col = cell.col * 26 + (chars[i] as usize - 'A' as usize + 1);
        i += 1;
    }

    // Must have at least one number after letters
    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return None;
    }

    // Parse row (numbers)
    let digits = &reference[i..];
    match digits.parse() {
        Ok(row) => cell.row = row,
        Err(_) => return None,
    }

    // Convert to 0-based indexing
    cell.row -= 1;
    cell.col -= 1;

    // Check if cell is within grid bounds
    if cell.row >= rows || cell.col >= cols {
        return None;
    }

    Some(cell)
}
/// Parses a binary operation from two operands and returns a BinaryOp struct.
pub fn parse_binary_op(
    operand1: &str,
    operand2: &str,
    backend: &Backend,
    success: &mut bool,
) -> BinaryOp {
    *success = true;
    // Operand 1 processing
    let first = if operand1.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        // Check if it's an integer
        let mut value = 0;
        for c in operand1.chars() {
            if c.is_ascii_digit() {
                value = value * 10 + (c as i32 - '0' as i32);
            } else {
                *success = false;
                break;
            }
        }
        Operand {
            type_: OperandType::Int,
            data: OperandData::Value(value),
        }
    } else {
        // Assume it's a cell reference
        match parse_cell_reference(operand1, backend.get_rows_col().0, backend.get_rows_col().1) {
            Some(cell) => Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(cell),
            },
            None => {
                *success = false;
                Operand {
                    type_: OperandType::Int,
                    data: OperandData::Value(0),
                }
            }
        }
    };

    // Operand 2 processing
    let second = if operand2.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        // Check if it's an integer
        let mut value = 0;
        for c in operand2.chars() {
            if c.is_ascii_digit() {
                value = value * 10 + (c as i32 - '0' as i32);
            } else {
                *success = false;
                break;
            }
        }
        Operand {
            type_: OperandType::Int,
            data: OperandData::Value(value),
        }
    } else {
        // Assume it's a cell reference
        match parse_cell_reference(operand2, backend.get_rows_col().0, backend.get_rows_col().1) {
            Some(cell) => Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(cell),
            },
            None => {
                *success = false;
                Operand {
                    type_: OperandType::Int,
                    data: OperandData::Value(0),
                }
            }
        }
    };

    BinaryOp { first, second }
}
/// Parses a range function (MIN, MAX, AVG, SUM, STDEV) from a string and returns a Function struct.
fn parse_range_function(
    expression: &str,
    function_type: FunctionType,
    backend: &Backend,
) -> (Function, bool) {
    let start_pos = match function_type {
        FunctionType::Stdev => 6, // "STDEV("
        _ => 4,                   // "MIN(", "MAX(", "AVG(", "SUM("
    };

    let content = &expression[start_pos..];
    let end_pos = match content.find(')') {
        Some(pos) => pos,
        None => return (Function::new_constant(0), false),
    };
    let range_str = &content[..end_pos];

    if let Some(separator_pos) = range_str.find(':') {
        let top_left_str = &range_str[..separator_pos];
        let bottom_right_str = &range_str[separator_pos + 1..];

        let top_left =
            match parse_cell_reference(top_left_str, backend.get_rows(), backend.get_cols()) {
                Some(cell) => cell,
                None => return (Function::new_constant(0), false),
            };
        let bottom_right =
            match parse_cell_reference(bottom_right_str, backend.get_rows(), backend.get_cols()) {
                Some(cell) => cell,
                None => return (Function::new_constant(0), false),
            };

        // Check if range is valid (top_left <= bottom_right)
        if top_left.row > bottom_right.row || top_left.col > bottom_right.col {
            return (Function::new_constant(0), false);
        }

        let range = RangeFunction {
            top_left,
            bottom_right,
        };

        return (Function::new_range_function(function_type, range), true);
    }

    // Default return if parsing fails
    (Function::new_constant(0), false)
}
#[cfg(feature = "gui")]
/// Parses an autofill command from a string and returns the start, end, and destination cells.
pub fn parse_autofill(
    backend: &Backend,
    expression: &str,
) -> Result<(Cell, Cell, Cell), Box<dyn std::error::Error>> {
    // println!("Parsing autofill command: {}", expression);
    let start_pos = 9; // "AUTOFILL("
    let content = &expression[start_pos..];
    let end_pos = match content.find(')') {
        Some(pos) => pos,
        None => return Err("Invalid command".to_string().into()),
    };
    let range_str = &content[..end_pos];

    if let Some(separator_pos) = range_str.find(':') {
        let start_str = &range_str[..separator_pos];

        if let Some(comma_pos) = range_str.find(',') {
            let dest_str = &range_str[comma_pos + 1..];
            let dest = parse_cell_reference(dest_str, backend.get_rows(), backend.get_cols());
            let dest_cell = match dest {
                Some(cell) => cell,
                None => return Err("Invalid cell reference".to_string().into()),
            };

            let end_str = &range_str[separator_pos + 1..comma_pos];

            let start = parse_cell_reference(start_str, backend.get_rows(), backend.get_cols());
            let start_cell = match start {
                Some(cell) => cell,
                None => return Err("Invalid cell reference".to_string().into()),
            };
            let end = parse_cell_reference(end_str, backend.get_rows(), backend.get_cols());
            let end_cell = match end {
                Some(cell) => cell,
                None => return Err("Invalid cell reference".to_string().into()),
            };
            if start.is_some() && end.is_some() && dest.is_some() {
                return Ok((start_cell, end_cell, dest_cell));
            }
        }
    }

    Err("Invalid command".to_string().into())
}
#[cfg(feature = "gui")]
/// Parses a cut or copy command from a string and returns the start and end cells.
pub fn parse_cut_or_copy(
    backend: &Backend,
    expression: &str,
) -> Result<(Cell, Cell), Box<dyn std::error::Error>> {
    // println!("Parsing cut/copy command: {}", expression);
    let mut start_pos = 4;
    if expression.starts_with("copy(") {
        start_pos = 5;
    }

    let content = &expression[start_pos..];
    let end_pos = match content.find(')') {
        Some(pos) => pos,
        None => return Err("Invalid command".to_string().into()),
    };
    let range_str = &content[..end_pos];

    if let Some(separator_pos) = range_str.find(':') {
        let top_left_str = &range_str[..separator_pos];
        let bottom_right_str = &range_str[separator_pos + 1..];

        let top_left = parse_cell_reference(top_left_str, backend.get_rows(), backend.get_cols());
        let top_left_cell = match top_left {
            Some(cell) => cell,
            None => return Err("Invalid cell reference".to_string().into()),
        };
        let bottom_right =
            parse_cell_reference(bottom_right_str, backend.get_rows(), backend.get_cols());
        let bottom_right_cell = match bottom_right {
            Some(cell) => cell,
            None => return Err("Invalid cell reference".to_string().into()),
        };

        if top_left.is_some() && bottom_right.is_some() {
            return Ok((top_left_cell, bottom_right_cell));
        }
    }

    Err("Invalid command".to_string().into())
}
#[cfg(feature = "gui")]
/// Parses a paste command from a string and returns the destination cell.
pub fn parse_paste(
    backend: &Backend,
    expression: &str,
) -> Result<Cell, Box<dyn std::error::Error>> {
    // println!("Parsing paste command: {}", expression);
    let start_pos = 6; // "PASTE("
    let content = &expression[start_pos..];
    let end_pos = match content.find(')') {
        Some(pos) => pos,
        None => return Err("Invalid command".to_string().into()),
    };
    let cell_str = &content[..end_pos];
    let cell = parse_cell_reference(cell_str, backend.get_rows(), backend.get_cols());
    match cell {
        Some(cell) => Ok(cell),
        None => Err("Invalid cell reference".to_string().into()),
    }
    // println!("Parsed cell: {:?}", cell);
}
/// Parses a function from a string and returns a Function struct.
pub fn parse_expression(expression: &str, backend: &Backend) -> (Function, bool) {
    let mut success = false;
    // Check if it's possible to be a parenthesis function (>=4 is the size)
    // println!("{}", expression.len());
    if expression.is_empty() {
        success = false;
        return (Function::new_constant(0), success);
    }
    if expression.len() >= 4 {
        // Check for range functions
        if expression.starts_with("MIN(") {
            return parse_range_function(expression, FunctionType::Min, backend);
        } else if expression.starts_with("MAX(") {
            return parse_range_function(expression, FunctionType::Max, backend);
        } else if expression.starts_with("AVG(") {
            return parse_range_function(expression, FunctionType::Avg, backend);
        } else if expression.starts_with("SUM(") {
            return parse_range_function(expression, FunctionType::Sum, backend);
        } else if expression.starts_with("STDEV(") {
            return parse_range_function(expression, FunctionType::Stdev, backend);
        } else if let Some(content) = expression.strip_prefix("SLEEP(") {
            // Parse sleep function
            // println!("content: {:?}", content);
            let end_pos = match content.find(')') {
                Some(pos) => pos,
                None => return (Function::new_constant(0), false),
            };
            // println!("end_pos: {:?}", end_pos);
            let value_str = &content[..end_pos];
            // println!("value_str: {:?}", value_str);
            if value_str
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() || c == '-')
            {
                match value_str.parse::<i32>() {
                    Ok(value) => return (Function::new_sleep(value), true),
                    Err(_) => return (Function::new_constant(0), false),
                }
            } else {
                let cell =
                    match parse_cell_reference(value_str, backend.get_rows(), backend.get_cols()) {
                        Some(cell) => cell,
                        None => return (Function::new_constant(0), false),
                    };

                //let val = backend.get_cell_value(cell.row, cell.col);
                return (Function::new_sleep_cell(cell), true);
            }
        }
    }

    // Check for binary operations
    let mut pos = None;
    for (i, c) in expression.chars().enumerate() {
        if (c == '+' || c == '-' || c == '*' || c == '/') && i != 0 {
            pos = Some(i);
            break;
        }
    }

    if let Some(i) = pos {
        // This is a binary operation
        let operator = match expression.chars().nth(i) {
            Some(op) => op,
            None => return (Function::new_constant(0), false),
        };
        let operand1 = &expression[..i];
        let operand2 = &expression[i + 1..];

        let binary_op = parse_binary_op(operand1, operand2, backend, &mut success);
        if !success {
            return (Function::new_constant(0), false);
        }

        let function_type = match operator {
            '+' => FunctionType::Plus,
            '-' => FunctionType::Minus,
            '*' => FunctionType::Multiply,
            '/' => FunctionType::Divide,
            _ => return (Function::new_constant(0), false),
        };
        success = true;
        (Function::new_binary_op(function_type, binary_op), success)
    } else {
        // Not a binary op, could be a constant or a cell reference

        if match expression.chars().next() {
            Some(c) => c.is_ascii_digit() || c == '-',
            None => false,
        } {
            // First char is a number or a minus sign, it's a constant
            match expression.parse::<i32>() {
                Ok(value) => (Function::new_constant(value), true),
                Err(_) => (Function::new_constant(0), false),
            }
        } else {
            // Parse as cell reference
            let cell =
                match parse_cell_reference(expression, backend.get_rows(), backend.get_cols()) {
                    Some(cell) => cell,
                    None => return (Function::new_constant(0), false),
                };
            let operand1 = Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(cell),
            };
            let operand2 = Operand {
                type_: OperandType::Int,
                data: OperandData::Value(0),
            };
            let binary_op = BinaryOp {
                first: operand1,
                second: operand2,
            };
            success = true;
            (
                Function::new_binary_op(FunctionType::Plus, binary_op),
                success,
            )
        }
    }
}
#[cfg(feature = "cli")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Backend;
    use crate::structs::{
        Cell, Function, FunctionType, Operand, OperandData, OperandType, RangeFunction,
    };

    #[test]
    fn test_parse_cell_reference_valid() {
        let rows = 10;
        let cols = 26;

        assert_eq!(
            parse_cell_reference("A1", rows, cols),
            Some(Cell { row: 0, col: 0 })
        );
        assert_eq!(
            parse_cell_reference("B2", rows, cols),
            Some(Cell { row: 1, col: 1 })
        );
        assert_eq!(
            parse_cell_reference("Z10", rows, cols),
            Some(Cell { row: 9, col: 25 })
        );
    }

    #[test]
    fn test_parse_cell_reference_invalid() {
        let rows = 10;
        let cols = 10;

        assert_eq!(parse_cell_reference("1A", rows, cols), None); // Invalid format
        assert_eq!(parse_cell_reference("AA", rows, cols), None); // Missing row
        assert_eq!(parse_cell_reference("A11", rows, cols), None); // Out of bounds
        assert_eq!(parse_cell_reference("", rows, cols), None); // Empty string
    }

    #[test]
    fn test_parse_binary_op_valid() {
        let backend = Backend::new(10, 10);
        let mut success = false;

        let binary_op = parse_binary_op("A1", "42", &backend, &mut success);
        assert!(success);
        assert_eq!(
            binary_op.first,
            Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(Cell { row: 0, col: 0 }),
            }
        );
        assert_eq!(
            binary_op.second,
            Operand {
                type_: OperandType::Int,
                data: OperandData::Value(42),
            }
        );

        let binary_op = parse_binary_op("10", "20", &backend, &mut success);
        assert!(success);
        assert_eq!(
            binary_op.first,
            Operand {
                type_: OperandType::Int,
                data: OperandData::Value(10),
            }
        );
        assert_eq!(
            binary_op.second,
            Operand {
                type_: OperandType::Int,
                data: OperandData::Value(20),
            }
        );
    }

    #[test]
    fn test_parse_binary_op_invalid() {
        let backend = Backend::new(10, 10);
        let mut success = false;

        let binary_op = parse_binary_op("Invalid", "42", &backend, &mut success);
        assert!(!success);
        assert_eq!(
            binary_op.first,
            Operand {
                type_: OperandType::Int,
                data: OperandData::Value(0),
            }
        );

        let binary_op = parse_binary_op("A1", "Invalid", &backend, &mut success);
        assert!(!success);
        assert_eq!(
            binary_op.second,
            Operand {
                type_: OperandType::Int,
                data: OperandData::Value(0),
            }
        );
    }

    #[test]
    fn test_parse_range_function_valid() {
        let backend = Backend::new(10, 10);

        let (function, success) = parse_range_function("SUM(A1:B2)", FunctionType::Sum, &backend);
        assert!(success);
        assert_eq!(
            function.data,
            Function::new_range_function(
                FunctionType::Sum,
                RangeFunction {
                    top_left: Cell { row: 0, col: 0 },
                    bottom_right: Cell { row: 1, col: 1 },
                }
            )
            .data
        );

        let (function, success) = parse_range_function("AVG(A1:A10)", FunctionType::Avg, &backend);
        assert!(success);
        assert_eq!(
            function.data,
            Function::new_range_function(
                FunctionType::Avg,
                RangeFunction {
                    top_left: Cell { row: 0, col: 0 },
                    bottom_right: Cell { row: 9, col: 0 },
                }
            )
            .data
        );
    }

    #[test]
    fn test_parse_range_function_invalid() {
        let backend = Backend::new(10, 10);

        let (function, success) =
            parse_range_function("SUM(A1:Invalid)", FunctionType::Sum, &backend);
        assert!(!success);
        assert_eq!(function.data, Function::new_constant(0).data);

        let (function, success) = parse_range_function("SUM(A1:A11)", FunctionType::Sum, &backend);
        assert!(!success);
        assert_eq!(function.data, Function::new_constant(0).data);

        let (function, success) =
            parse_range_function("SUM(A1:B1:C1)", FunctionType::Sum, &backend);
        assert!(!success);
        assert_eq!(function.data, Function::new_constant(0).data);
    }

    #[test]
    fn test_parse_expression_constant() {
        let backend = Backend::new(10, 10);

        let (function, success) = parse_expression("42", &backend);
        assert!(success);
        assert_eq!(function.data, Function::new_constant(42).data);

        let (function, success) = parse_expression("-42", &backend);
        assert!(success);
        assert_eq!(function.data, Function::new_constant(-42).data);
    }

    #[test]
    fn test_parse_expression_cell_reference() {
        let backend = Backend::new(10, 10);

        let (function, success) = parse_expression("A1", &backend);
        assert!(success);
        assert_eq!(
            function.data,
            Function::new_binary_op(
                FunctionType::Plus,
                BinaryOp {
                    first: Operand {
                        type_: OperandType::Cell,
                        data: OperandData::Cell(Cell { row: 0, col: 0 }),
                    },
                    second: Operand {
                        type_: OperandType::Int,
                        data: OperandData::Value(0),
                    },
                }
            )
            .data
        );
    }

    #[test]
    fn test_parse_expression_binary_op() {
        let backend = Backend::new(10, 10);

        let (function, success) = parse_expression("A1+42", &backend);
        assert!(success);
        assert_eq!(
            function.data,
            Function::new_binary_op(
                FunctionType::Plus,
                BinaryOp {
                    first: Operand {
                        type_: OperandType::Cell,
                        data: OperandData::Cell(Cell { row: 0, col: 0 }),
                    },
                    second: Operand {
                        type_: OperandType::Int,
                        data: OperandData::Value(42),
                    },
                }
            )
            .data
        );
    }

    #[test]
    fn test_parse_expression_invalid() {
        let backend = Backend::new(10, 10);

        let (function, success) = parse_expression("Invalid", &backend);
        assert!(!success);
        assert_eq!(function.data, Function::new_constant(0).data);
    }
}
