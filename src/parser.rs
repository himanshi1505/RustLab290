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
        cell.col = cell.col * 26 + (chars[i] as u32 - 'A' as u32 + 1);  // Convert letters to column index
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
        Operand::new_int(value)
    } else {
        // Assume it's a cell reference
        let cell = parse_cell_reference(operand1);
        Operand::new_cell(cell)
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
        Operand::new_int(value)
    } else {
        // Assume it's a cell reference
        let cell = parse_cell_reference(operand2);
        Operand::new_cell(cell)
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
            let operand1 = Operand::new_cell(cell);
            let operand2 = Operand::new_int(0);
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
    use crate::cell::Cell;
    use crate::structs::*;

    // Helper function to create a cell
    fn cell(row: u32, col: u32) -> Cell {
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
        assert_eq!(result.value, Some(42));
        assert!(result.binary_op.is_none());
        assert!(result.range_function.is_none());
        
        let result = parse_expression("0");
        assert_eq!(result.type_, FunctionType::Constant);
        assert_eq!(result.value, Some(0));
        
        let result = parse_expression("999");
        assert_eq!(result.type_, FunctionType::Constant);
        assert_eq!(result.value, Some(999));
    }

    #[test]
    fn test_parse_cell_as_expression() {
        // Test parsing a single cell reference as an expression
        let result = parse_expression("A1");
        assert_eq!(result.type_, FunctionType::Plus);
        assert!(result.binary_op.is_some());
        
        let binary_op = result.binary_op.unwrap();
        assert_eq!(binary_op.first.type_, OperandType::Cell);
        assert_eq!(binary_op.first.cell, Some(cell(1, 1)));
        assert_eq!(binary_op.second.type_, OperandType::Int);
        assert_eq!(binary_op.second.value, Some(0));
        
        // Test another cell reference
        let result = parse_expression("Z26");
        assert_eq!(result.type_, FunctionType::Plus);
        assert!(result.binary_op.is_some());
        
        let binary_op = result.binary_op.unwrap();
        assert_eq!(binary_op.first.type_, OperandType::Cell);
        assert_eq!(binary_op.first.cell, Some(cell(26, 26)));
    }

    #[test]
    fn test_parse_binary_operations() {
        // Test addition
        let result = parse_expression("A1+B2");
        assert_eq!(result.type_, FunctionType::Plus);
        assert!(result.binary_op.is_some());
        
        let binary_op = result.binary_op.unwrap();
        assert_eq!(binary_op.first.type_, OperandType::Cell);
        assert_eq!(binary_op.first.cell, Some(cell(1, 1)));
        assert_eq!(binary_op.second.type_, OperandType::Cell);
        assert_eq!(binary_op.second.cell, Some(cell(2, 2)));
        
        // Test subtraction
        let result = parse_expression("C3-10");
        assert_eq!(result.type_, FunctionType::Minus);
        assert!(result.binary_op.is_some());
        
        let binary_op = result.binary_op.unwrap();
        assert_eq!(binary_op.first.type_, OperandType::Cell);
        assert_eq!(binary_op.first.cell, Some(cell(3, 3)));
        assert_eq!(binary_op.second.type_, OperandType::Int);
        assert_eq!(binary_op.second.value, Some(10));
        
        // Test multiplication
        let result = parse_expression("5*D4");
        assert_eq!(result.type_, FunctionType::Multiply);
        assert!(result.binary_op.is_some());
        
        let binary_op = result.binary_op.unwrap();
        assert_eq!(binary_op.first.type_, OperandType::Int);
        assert_eq!(binary_op.first.value, Some(5));
        assert_eq!(binary_op.second.type_, OperandType::Cell);
        assert_eq!(binary_op.second.cell, Some(cell(4, 4)));
        
        // Test division
        let result = parse_expression("E5/F6");
        assert_eq!(result.type_, FunctionType::Divide);
        assert!(result.binary_op.is_some());
        
        let binary_op = result.binary_op.unwrap();
        assert_eq!(binary_op.first.type_, OperandType::Cell);
        assert_eq!(binary_op.first.cell, Some(cell(5, 5)));
        assert_eq!(binary_op.second.type_, OperandType::Cell);
        assert_eq!(binary_op.second.cell, Some(cell(6, 6)));
    }

    #[test]
    fn test_parse_range_functions() {
        // Test MIN function
        let result = parse_expression("MIN(A1:B2)");
        assert_eq!(result.type_, FunctionType::Min);
        assert!(result.range_function.is_some());
        
        let range = result.range_function.unwrap();
        assert_eq!(range.top_left, cell(1, 1));
        assert_eq!(range.bottom_right, cell(2, 2));
        
        // Test MAX function
        let result = parse_expression("MAX(C3:D4)");
        assert_eq!(result.type_, FunctionType::Max);
        assert!(result.range_function.is_some());
        
        let range = result.range_function.unwrap();
        assert_eq!(range.top_left, cell(3, 3));
        assert_eq!(range.bottom_right, cell(4, 4));
        
        // Test AVG function
        let result = parse_expression("AVG(E5:F6)");
        assert_eq!(result.type_, FunctionType::Avg);
        assert!(result.range_function.is_some());
        
        let range = result.range_function.unwrap();
        assert_eq!(range.top_left, cell(5, 5));
        assert_eq!(range.bottom_right, cell(6, 6));
        
        // Test SUM function
        let result = parse_expression("SUM(G7:H8)");
        assert_eq!(result.type_, FunctionType::Sum);
        assert!(result.range_function.is_some());
        
        let range = result.range_function.unwrap();
        assert_eq!(range.top_left, cell(7, 7));
        assert_eq!(range.bottom_right, cell(8, 8));
        
        // Test STDEV function
        let result = parse_expression("STDEV(I9:J10)");
        assert_eq!(result.type_, FunctionType::Stdev);
        assert!(result.range_function.is_some());
        
        let range = result.range_function.unwrap();
        assert_eq!(range.top_left, cell(9, 9));
        assert_eq!(range.bottom_right, cell(10, 10));
    }

    #[test]
    fn test_parse_sleep_function() {
        // Test SLEEP function
        let result = parse_expression("SLEEP(5)");
        assert_eq!(result.type_, FunctionType::Sleep);
        assert_eq!(result.value, Some(5));
        
        let result = parse_expression("SLEEP(10)");
        assert_eq!(result.type_, FunctionType::Sleep);
        assert_eq!(result.value, Some(10));
    }

    #[test]
    fn test_edge_cases() {
        // Test empty string (should default to a constant 0)
        let result = parse_expression("");
        assert_eq!(result.type_, FunctionType::Constant);
        assert_eq!(result.value, Some(0));
        
        // Test malformed range function (missing colon)
        let result = parse_expression("SUM(A1B2)");
        assert_eq!(result.type_, FunctionType::Constant);
        assert_eq!(result.value, Some(0));
        
        // Test malformed function (missing closing parenthesis)
        let result = parse_expression("MIN(A1:B2");
        assert_eq!(result.type_, FunctionType::Min);
        assert!(result.range_function.is_some());
        
        // Test with extra spaces (assuming spaces are not handled specifically)
        // This may fail if the function doesn't handle spaces well
        let result = parse_expression("A1 + B2");
        assert_eq!(result.type_, FunctionType::Plus);
        assert!(result.binary_op.is_some());
    }
}