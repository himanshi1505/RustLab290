use crate::structs::*;
use crate::backend::Backend;


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
    // println!("cell.row: {:?}", cell.row);
    // println!("cell.col: {:?}", cell.col);
    cell.row-=1;
    cell.col-=1;
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
pub fn parse_expression(expression: &str, backend: &Backend) -> Function {
    let mut success = false;
    // Check if it's possible to be a parenthesis function (>=4 is the size)
    // println!("{}", expression.len());
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
            // println!("content: {:?}", content);
            let end_pos = content.find(')').unwrap_or(content.len());
            // println!("end_pos: {:?}", end_pos); 
            let value_str = &content[..end_pos];
            // println!("value_str: {:?}", value_str);
            if(value_str.chars().next().unwrap().is_ascii_digit() || value_str.chars().next().unwrap() == '-' ){
                let value = value_str.parse::<i32>().unwrap_or(0);
                // println!("value: {:?}", value);
                return Function::new_sleep(value);
            }
            else{
                let cell = parse_cell_reference(value_str);
                if let Some(val) = backend.get_cell_value(&cell){
                    return Function::new_sleep(val.borrow().value);
                }
                
            }
            
        }
    }
    
    // Check for binary operations
    let mut pos = None;
    for (i, c) in expression.chars().enumerate() {
        
        if (c == '+' || c == '-' || c == '*' || c == '/' )&& i != 0{
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
           
            Some(c) => c.is_ascii_digit() || c == '-' ,
            None => false,
        } {
            // First char is a number or a minus sign, it's a constant
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

