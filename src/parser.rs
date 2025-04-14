use crate::structs::*;


// fn convert_to_int(expression: &str) -> i32 {
//     let mut result = 0;
    
//     for c in expression.chars() {
//         if c >= '0' && c <= '9' {
//             result = result * 10 + (c as i32 - '0' as i32);
//         } else {
//             break;
//         }
//     }
    
//     result
// }

pub fn parse_cell_reference(reference: &str) -> Cell {
    let mut cell = Cell { row: 0, col: 0 };
    let chars: Vec<char> = reference.chars().collect();
    let mut i = 0;
    
    // Parse column (letters)
    while i < chars.len() && chars[i] >= 'A' && chars[i] <= 'Z' {
        cell.col = cell.col * 26 + (chars[i] as usize - 'A' as usize + 1);  // Convert letters to column index
        i += 1;
    }
    
    // Parse row (numbers)
    if i < chars.len() && chars[i] >= '0' && chars[i] <= '9' {
        let digits = &reference[i..];
        cell.row = digits.parse().unwrap_or(0);  // Convert digits to row number
    }
    
    cell
}

pub fn parse_binary_op(operand1: &str, operand2: &str) -> BinaryOp { //will have to check whether BinaryOp works or if we have to take its reference 
    // Operand 1 processing
    let first = if match operand1.chars().next() {
        Some(c) => c.is_ascii_digit(),
        None => false,
    }{
        // Check if it's an integer
        let mut value = 0;
        for c in operand1.chars() {
            if c.is_ascii_digit() {
                value = value * 10 + (c as i32 - '0' as i32);
            } else {
                break;
            }
        }
        Operand {
            type_: OperandType::Int,
            data: OperandData::Value(value)
        }
    } else {
        // Assume it's a cell reference
        let cell = parse_cell_reference(operand1);
        Operand {
            type_: OperandType::Cell,
            data: OperandData::Cell(cell)
        }
    };

    // Operand 2 processing
    let second = if match operand2.chars().next() {
        Some(c) => c.is_ascii_digit(),
        None => false,
    } {
        // Check if it's an integer
        let mut value = 0;
        for c in operand2.chars() {
            if c.is_ascii_digit() {
                value = value * 10 + (c as i32 - '0' as i32);
            } else {
                break;
            }
        }
        Operand {
            type_: OperandType::Int,
            data: OperandData::Value(value)
        }
    } else {
        // Assume it's a cell reference
        let cell = parse_cell_reference(operand2);
        Operand {
            type_: OperandType::Cell,
            data: OperandData::Cell(cell)
        }
    };

    BinaryOp { first, second }
}

fn parse_range_function(expression: &str, function_type: FunctionType) -> Function {
    let start_pos = match function_type {
        FunctionType::Stdev => 6, // "STDEV("
        _ => 4,                  // "MIN(", "MAX(", "AVG(", "SUM("
    };
    
    let content = &expression[start_pos..];
    let end_pos = content.find(')').unwrap_or(content.len());
    let range_str = &content[..end_pos];
    
    if let Some(separator_pos) = range_str.find(':') {
        let top_left_str = &range_str[..separator_pos];
        let bottom_right_str = &range_str[separator_pos+1..];
        
        let top_left = parse_cell_reference(top_left_str);
        let bottom_right = parse_cell_reference(bottom_right_str);
        
        let range = RangeFunction {
            top_left,
            bottom_right,
        };
        
        return Function::new_range_function(function_type, range);
    }
    
    // Default return if parsing fails
    Function::new_constant(0)
}


//success param was not being used so removed it
pub fn parse_expression(expression: &str) -> Function {
    let mut success = false;
    // Check if it's possible to be a parenthesis function (>=4 is the size)
    println!("{}", expression.len());
    if (expression.len() == 0) {
        success = false;
        return Function::new_constant(0);
    }
    if expression.len() >= 4 {
        // Check for range functions
        if expression.starts_with("MIN(") {
            return parse_range_function(expression, FunctionType::Min);
        } else if expression.starts_with("MAX(") {
            return parse_range_function(expression, FunctionType::Max);
        } else if expression.starts_with("AVG(") {
            return parse_range_function(expression, FunctionType::Avg);
        } else if expression.starts_with("SUM(") {
            return parse_range_function(expression, FunctionType::Sum);
        } else if expression.starts_with("STDEV(") {
            return parse_range_function(expression, FunctionType::Stdev);
        } else if expression.starts_with("SLEEP(") {
            // Parse sleep function
            let content = &expression[6..];
            let end_pos = content.find(')').unwrap_or(content.len());
            let value_str = &content[..end_pos];
            let value = value_str.parse::<i32>().unwrap_or(0);
            return Function::new_sleep(value);
        }
    }
    
    // Check for binary operations
    let mut pos = None;
    for (i, c) in expression.chars().enumerate() {
        if c == '+' || c == '-' || c == '*' || c == '/' {
            pos = Some(i);
            break;
        }
    }
    
    if let Some(i) = pos {
        // This is a binary operation
        let operator = expression.chars().nth(i).unwrap();
        let operand1 = &expression[..i];
        let operand2 = &expression[i+1..];
        
        let binary_op = parse_binary_op(operand1, operand2);
        
        let function_type = match operator {
            '+' => FunctionType::Plus,
            '-' => FunctionType::Minus,
            '*' => FunctionType::Multiply,
            '/' => FunctionType::Divide,
            _ => panic!("Unexpected operator"),
        };
        // *success = true;
        return Function::new_binary_op(function_type, binary_op);
    } else {
        // Not a binary op, could be a constant or a cell reference
        if match expression.chars().next(){
            Some(c) => c.is_ascii_digit(),
            None => false,
        } {
            // First char is a number, it's a constant
            let value = expression.parse::<i32>().unwrap_or(0);
            // *success = true;
            return Function::new_constant(value);
        } else {
            // Parse as cell reference
            let cell = parse_cell_reference(expression);
            let operand1 = Operand {
                type_: OperandType::Cell,
                data: OperandData::Cell(cell)
            };
            let operand2 = Operand {
                type_: OperandType::Int,
                data: OperandData::Value(0)
            };
            let binary_op = BinaryOp {
                first: operand1,
                second: operand2,
            };
            // *success = true;
            Function::new_binary_op(FunctionType::Plus, binary_op)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::structs::*;

    // Helper function to create a cell
    fn cell(row: usize, col: usize) -> Cell {
        Cell { row, col }
    }

    #[test]
    fn test_parse_cell_reference() {
        // Test basic cell references
        assert_eq!(parse_cell_reference("A1"), cell(1, 1));
        assert_eq!(parse_cell_reference("B2"), cell(2, 2));
        assert_eq!(parse_cell_reference("Z10"), cell(10, 26));
        
        // Test multi-letter column references
        assert_eq!(parse_cell_reference("AA1"), cell(1, 27));
        assert_eq!(parse_cell_reference("AB5"), cell(5, 28));
        assert_eq!(parse_cell_reference("BA10"), cell(10, 53));
    }

    #[test]
    fn test_parse_constants() {
        // Test parsing constant values
        let result = parse_expression("42");
        assert_eq!(result.type_, FunctionType::Constant);
        if let FunctionData::Value(value) = result.data {
            assert_eq!(value, 42);
        } else {
            panic!("Expected Value variant");
        }
        
        let result = parse_expression("0");
        assert_eq!(result.type_, FunctionType::Constant);
        if let FunctionData::Value(value) = result.data {
            assert_eq!(value, 0);
        } else {
            panic!("Expected Value variant");
        }
        
        let result = parse_expression("999");
        assert_eq!(result.type_, FunctionType::Constant);
        if let FunctionData::Value(value) = result.data {
            assert_eq!(value, 999);
        } else {
            panic!("Expected Value variant");
        }
    }

    #[test]
    fn test_parse_cell_as_expression() {
        // Test parsing a single cell reference as an expression
        let result = parse_expression("A1");
        assert_eq!(result.type_, FunctionType::Plus);
        if let FunctionData::BinaryOp(binary_op) = result.data {
            assert_eq!(binary_op.first.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.first.data {
                assert_eq!(cell, Cell { row: 1, col: 1 });
            } else {
                panic!("Expected Cell variant");
            }
            assert_eq!(binary_op.second.type_, OperandType::Int);
        } else {
            panic!("Expected BinaryOp variant");
        }
        
        // Test another cell reference
        let result = parse_expression("Z26");
        assert_eq!(result.type_, FunctionType::Plus);
        if let FunctionData::BinaryOp(binary_op) = result.data {
            assert_eq!(binary_op.first.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.first.data {
                assert_eq!(cell, Cell { row: 26, col: 26 });
            } else {
                panic!("Expected Cell variant");
            }
        } else {
            panic!("Expected BinaryOp variant");
        }
    }

    #[test]
    fn test_parse_binary_operations() {
        // Test addition
        let result = parse_expression("A1+B2");
        assert_eq!(result.type_, FunctionType::Plus);
        if let FunctionData::BinaryOp(binary_op) = result.data {
            assert_eq!(binary_op.first.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.first.data {
                assert_eq!(cell, Cell { row: 1, col: 1 });
            } else {
                panic!("Expected Cell variant");
            }
            assert_eq!(binary_op.second.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.second.data {
                assert_eq!(cell, Cell { row: 2, col: 2 });
            } else {
                panic!("Expected Cell variant");
            }
        } else {
            panic!("Expected BinaryOp variant");
        }
        
        // Test subtraction
        let result = parse_expression("C3-10");
        assert_eq!(result.type_, FunctionType::Minus);
        if let FunctionData::BinaryOp(binary_op) = result.data {
            assert_eq!(binary_op.first.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.first.data {
                assert_eq!(cell, Cell { row: 3, col: 3 });
            } else {
                panic!("Expected Cell variant");
            }
            assert_eq!(binary_op.second.type_, OperandType::Int);
            if let OperandData::Value(value) = binary_op.second.data {
                assert_eq!(value, 10);
            } else {
                panic!("Expected Value variant");
            }
        } else {
            panic!("Expected BinaryOp variant");
        }
        
        // Test multiplication
        let result = parse_expression("5*D4");
        assert_eq!(result.type_, FunctionType::Multiply);
        if let FunctionData::BinaryOp(binary_op) = result.data {
            assert_eq!(binary_op.first.type_, OperandType::Int);
            if let OperandData::Value(value) = binary_op.first.data {
                assert_eq!(value, 5);
            } else {
                panic!("Expected Value variant");
            }
            assert_eq!(binary_op.second.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.second.data {
                assert_eq!(cell, Cell { row: 4, col: 4 });
            } else {
                panic!("Expected Cell variant");
            }
        } else {
            panic!("Expected BinaryOp variant");
        }
        
        // Test division
        let result = parse_expression("E5/F6");
        assert_eq!(result.type_, FunctionType::Divide);
        if let FunctionData::BinaryOp(binary_op) = result.data {
            assert_eq!(binary_op.first.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.first.data {
                assert_eq!(cell, Cell { row: 5, col: 5 });
            } else {
                panic!("Expected Cell variant");
            }
            assert_eq!(binary_op.second.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.second.data {
                assert_eq!(cell, Cell { row: 6, col: 6 });
            } else {
                panic!("Expected Cell variant");
            }
        } else {
            panic!("Expected BinaryOp variant");
        }
    }

    #[test]
    fn test_parse_range_functions() {
        // Test MIN function
        let result = parse_expression("MIN(A1:B2)");
        assert_eq!(result.type_, FunctionType::Min);
        if let FunctionData::RangeFunction(range) = result.data {
            assert_eq!(range.top_left, Cell { row: 1, col: 1 });
            assert_eq!(range.bottom_right, Cell { row: 2, col: 2 });
        } else {
            panic!("Expected RangeFunction variant");
        }
        
        // Test MAX function
        let result = parse_expression("MAX(C3:D4)");
        assert_eq!(result.type_, FunctionType::Max);
        if let FunctionData::RangeFunction(range) = result.data {
            assert_eq!(range.top_left, Cell { row: 3, col: 3 });
            assert_eq!(range.bottom_right, Cell { row: 4, col: 4 });
        } else {
            panic!("Expected RangeFunction variant");
        }
        
        // Test AVG function
        let result = parse_expression("AVG(E5:F6)");
        assert_eq!(result.type_, FunctionType::Avg);
        if let FunctionData::RangeFunction(range) = result.data {
            assert_eq!(range.top_left, Cell { row: 5, col: 5 });
            assert_eq!(range.bottom_right, Cell { row: 6, col: 6 });
        } else {
            panic!("Expected RangeFunction variant");
        }
        
        // Test SUM function
        let result = parse_expression("SUM(G7:H8)");
        assert_eq!(result.type_, FunctionType::Sum);
        if let FunctionData::RangeFunction(range) = result.data {
            assert_eq!(range.top_left, Cell { row: 7, col: 7 });
            assert_eq!(range.bottom_right, Cell { row: 8, col: 8 });
        } else {
            panic!("Expected RangeFunction variant");
        }
        
        // Test STDEV function
        let result = parse_expression("STDEV(I9:J10)");
        assert_eq!(result.type_, FunctionType::Stdev);
        if let FunctionData::RangeFunction(range) = result.data {
            assert_eq!(range.top_left, Cell { row: 9, col: 9 });
            assert_eq!(range.bottom_right, Cell { row: 10, col: 10 });
        } else {
            panic!("Expected RangeFunction variant");
        }
    }

    #[test]
    fn test_parse_sleep_function() {
        // Test SLEEP function
        let result = parse_expression("SLEEP(5)");
        assert_eq!(result.type_, FunctionType::Sleep);
        if let FunctionData::SleepValue(operand) = result.data {
            assert_eq!(operand.type_, OperandType::Int);
            if let OperandData::Value(value) = operand.data {
                assert_eq!(value, 5);
            } else {
                panic!("Expected Value variant");
            }
        } else {
            panic!("Expected SleepValue variant");
        }
        
        let result = parse_expression("SLEEP(10)");
        assert_eq!(result.type_, FunctionType::Sleep);
        if let FunctionData::SleepValue(operand) = result.data {
            assert_eq!(operand.type_, OperandType::Int);
            if let OperandData::Value(value) = operand.data {
                assert_eq!(value, 10);
            } else {
                panic!("Expected Value variant");
            }
        } else {
            panic!("Expected SleepValue variant");
        }
    }

    #[test]
    fn test_edge_cases() {
        // Test empty string (should default to a constant 0)
        let result = parse_expression("");
        assert_eq!(result.type_, FunctionType::Constant);
        if let FunctionData::Value(value) = result.data {
            assert_eq!(value, 0);
        } else {
            panic!("Expected Value variant");
        }
        
        // Test malformed range function (missing colon)
        let result = parse_expression("SUM(A1B2)");
        assert_eq!(result.type_, FunctionType::Constant);
        if let FunctionData::Value(value) = result.data {
            assert_eq!(value, 0);
        } else {
            panic!("Expected Value variant");
        }
        
        // Test malformed function (missing closing parenthesis)
        let result = parse_expression("MIN(A1:B2");
        assert_eq!(result.type_, FunctionType::Min);
        if let FunctionData::RangeFunction(range) = result.data {
            assert_eq!(range.top_left, Cell { row: 1, col: 1 });
            assert_eq!(range.bottom_right, Cell { row: 2, col: 2 });
        } else {
            panic!("Expected RangeFunction variant");
        }
        
        // Test with extra spaces (assuming spaces are not handled specifically)
        // This may fail if the function doesn't handle spaces well
        let result = parse_expression("A1 + B2");
        assert_eq!(result.type_, FunctionType::Plus);
        if let FunctionData::BinaryOp(binary_op) = result.data {
            assert_eq!(binary_op.first.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.first.data {
                assert_eq!(cell, Cell { row: 1, col: 1 });
            } else {
                panic!("Expected Cell variant");
            }
            assert_eq!(binary_op.second.type_, OperandType::Cell);
            if let OperandData::Cell(cell) = binary_op.second.data {
                assert_eq!(cell, Cell { row: 2, col: 2 });
            } else {
                panic!("Expected Cell variant");
            }
        } else {
            panic!("Expected BinaryOp variant");
        }
    }
}