use crate::backend::*;
use crate::parser::*;
use crate::structs::*;
// use std::arch::x86_64::_CMP_UNORD_Q;
use std::cmp::min;
use std::io::{self, Write};
use std::time::Instant;
#[cfg(feature = "gui")]
use std::cell::RefCell;


const MAX_WIDTH: usize = 10;

pub struct Frontend {
    backend: Backend,
    rows: usize,
    cols: usize,
    cell_width: usize,
    do_print: bool,
    top_left: Cell,
}

#[cfg(feature = "gui")]
impl PartialEq for Frontend {
    fn eq(&self, other: &Self) -> bool {
        self.rows == other.rows && self.cols == other.cols
    }
}
impl Frontend {
    pub fn new(rows: usize, cols: usize) -> Self {
        let backend = Backend::new(rows, cols);
      
        Self {
            backend,
            rows,
            cols,
            cell_width: 12,
            do_print: true,
            top_left: Cell { row: 0, col: 0 },
        }
    }

    #[cfg(feature = "gui")]
    pub fn get_backend_mut(&mut self) -> &mut Backend {
        &mut self.backend
    }
    #[cfg(feature = "gui")]
    pub fn get_rows_col(&self) -> (usize,usize) {
        (self.rows,self.cols)
    }
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

    pub fn print_board(&self) {
        if !self.do_print {
            return;
        }
        let row_width = min(MAX_WIDTH, self.rows - self.top_left.row);
        let col_width = min(MAX_WIDTH, self.cols - self.top_left.col);

        print!("{:<width$}", "", width = self.cell_width);
        for col in self.top_left.col..(self.top_left.col + col_width) {
            print!(
                "{:<width$}",
                Self::number_to_column_header(col),
                width = self.cell_width
            );
        }
        println!();

        for row in self.top_left.row..(self.top_left.row + row_width) {
            print!("{:<width$}", row + 1, width = self.cell_width);
            for col in self.top_left.col..(self.top_left.col + col_width) {
               
                unsafe{let data = self.backend.get_cell_value(row,col);
           
                       
                        // println!("data.error: {:?}", data.error);
                        match (*data).error {
                            CellError::NoError => {
                                print!("{:<width$}", (*data).value, width = self.cell_width);
                            }
                            _ => {
                                // println!("in printing ERR");
                                print!("{:<width$}", "ERR", width = self.cell_width);
                            }
                        }
                    
                    } 
                
            }
            println!();
        }
    }

    fn remove_spaces(s: &mut String) {
        let mut cleaned = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].is_whitespace() {
                if i > 0
                    && i + 1 < chars.len()
                    && chars[i - 1].is_alphanumeric()
                    && chars[i + 1].is_alphanumeric()
                {
                    cleaned.push(' ');
                }
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
            } else {
                cleaned.push(chars[i]);
                i += 1;
            }
        }
        *s = cleaned;
    }

    fn run_frontend_command(&mut self, cmd: &str) -> bool {
        match cmd {
            "disable_output" => self.do_print = false,
            "enable_output" => self.do_print = true,
            "q" => std::process::exit(0),
            "w" => {
                if self.top_left.row >= MAX_WIDTH {
                    self.top_left.row -= MAX_WIDTH;
                } else {
                    self.top_left.row = 0;
                }
            }
            "s" => {
                if self.top_left.row + 2 * MAX_WIDTH <= self.rows {
                    self.top_left.row += MAX_WIDTH;
                } else {
                    self.top_left.row = self.rows - MAX_WIDTH;
                }
            }
            "a" => {
                if self.top_left.col >= MAX_WIDTH {
                    self.top_left.col -= MAX_WIDTH;
                } else {
                    self.top_left.col = 0;
                }
            }
            "d" => {
                if self.top_left.col + 2 * MAX_WIDTH <= self.cols {
                    self.top_left.col += MAX_WIDTH;
                } else {
                    self.top_left.col = self.cols - MAX_WIDTH;
                }
            }
            #[cfg(feature = "gui")]
            "undo" => {
                self.backend.undo_callback();
            }
            #[cfg(feature = "gui")]
            "redo" => {
                self.backend.redo_callback();
            }
            cmd if cmd.starts_with("scroll_to ") => {
                let cell_str = cmd.trim_start_matches("scroll_to ").trim();
                let (rows, cols) = self.backend.get_rows_col();
                if let Some(cell) = parse_cell_reference(cell_str, rows, cols) {
                    self.top_left = cell;
                } 
                else {
                    return false;
                }
            } 
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("load(") => {
                let res = backend::Backend::load_csv(&mut self.backend, cmd, false);
                match res {
                    Ok(_) => {return true;}
                    Err(_) => {return false;}
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("save(") => {
                println!("save");
                let res = backend::Backend::save_to_csv(&self.backend, cmd);
                match res {
                    Ok(_) => {return true;}
                    Err(_) => {return false;}
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("copy(") => {
                // self.backend.push_undo_state();
                println!("copy");
                let res = backend::Backend::copy(&mut self.backend, cmd);
                match res {
                    Ok(_) => {return true;}
                    Err(_) => {return false;}
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("cut(") => {
                self.backend.push_undo_state();
                let res = backend::Backend::cut(&mut self.backend, cmd);
                match res {
                    Ok(_) => {return true;}
                    Err(_) => {return false;}
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("paste(") => {
                self.backend.push_undo_state();
                let res = backend::Backend::paste(&mut self.backend, cmd);
                match res {
                    Ok(_) => {return true;}
                    Err(_) => {return false;}
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("autofill") => {
                self.backend.push_undo_state();
                let res = backend::Backend::autofill(&mut self.backend, cmd);
                match res {
                    Ok(_) => {return true;}
                    Err(_) => {return false;}
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("sort") => {
                println!("sort");
                self.backend.push_undo_state();
                let res = backend::Backend::sort(&mut self.backend, cmd);
                match res {
                    Ok(_) => {return true;}
                    Err(_) => {return false;}
                }
            }
            _ => return false,
        }
        true
    }

    pub fn run_command(&mut self, input: &str) -> bool {
        if input
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            if let Some(eq_pos) = input.find('=') {
                #[cfg(feature = "gui")]
                let formula = input[eq_pos..].trim();
                let (cell_str, expr_str) = input.split_at(eq_pos);
                let (rows, cols) = self.backend.get_rows_col();
                #[cfg(feature = "gui")]
                self.backend.push_undo_state();
                if let Some(cell) = parse_cell_reference(cell_str, rows, cols) {
                    #[cfg(feature = "gui")]
                    let row_num = cell.row;
                    #[cfg(feature = "gui")]
                    let col_num = cell.col;
                    
                    
                    let expr = &expr_str[1..]; // skip '='

                    match self.backend.set_cell_value(cell, expr) {
                        Ok(_) => {
                            #[cfg(feature = "gui")]
                            
                            {
                                self.backend.formula_strings[row_num][col_num] = expr_str.to_string();
                            }
                            true
                        }
                        Err(_) => false,
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            self.run_frontend_command(input)
        }
    }

    pub fn run(&mut self) {
        let mut status = "ok";
        let mut time_taken = 0.0;

        loop {
            print!("[{:.1}] ({}) > ", time_taken, status);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                continue;
            }
            let start = Instant::now();
            Self::remove_spaces(&mut input);
            let input = input.trim();
            if input.is_empty() {
                continue;
            }
            if self.run_command(input) {
                status = "ok";
            } else {
                status = "err";
            }
            time_taken = start.elapsed().as_secs_f64();
            self.print_board();
        }
    }
}

// use std::io::{self, Write, BufRead};
// use std::process::exit;
// use std::time::{Instant, Duration};
// use std::cmp::min;

// use crate::backend::{self, Cell, CellError, ExpressionError};
// use crate::parser::{self, parse_cell_reference, parse_expression};

// const MAX_WIDTH: usize = 10;

// // Static variables from C code
// struct FrontendState {
//     rows: usize,
//     cols: usize,
//     cell_width: usize,
//     row_width: usize,
//     col_width: usize,
//     do_print: bool,
//     top_left: Cell,
// }

// impl FrontendState {
//     fn new() -> Self {
//         FrontendState {
//             rows: 0,
//             cols: 0,
//             cell_width: 12,
//             row_width: 0,
//             col_width: 0,
//             do_print: true,
//             top_left: Cell { row: 0, col: 0 },
//         }
//     }
// }

// static mut STATE: FrontendState = FrontendState {
//     rows: 0,
//     cols: 0,
//     cell_width: 12,
//     row_width: 0,
//     col_width: 0,
//     do_print: true,
//     top_left: Cell { row: 0, col: 0 },
// };

// /// Converts a 0-based column number to a letter-based column header (A, B, ..., Z, AA, ...)
// fn number_to_column_header(number: usize) -> String {
//     // Adding 1 to mimic the C implementation (1-based)
//     let mut num = number + 1;
//     let mut result = String::new();

//     while num > 0 {
//         let rem = (num - 1) % 26;
//         result.insert(0, (b'A' + rem as u8) as char);
//         num = (num - 1) / 26;
//     }

//     result
// }

// /// Prints the current visible portion of the spreadsheet
// fn print_board() {
//     unsafe {
//         if !STATE.do_print {
//             return;
//         }

//         STATE.row_width = min(MAX_WIDTH, STATE.rows - STATE.top_left.row);
//         STATE.col_width = min(MAX_WIDTH, STATE.cols - STATE.top_left.col);

//         // Print empty cell at top-left corner
//         print!("{:<width$}", "", width = STATE.cell_width);

//         let max_col = STATE.top_left.col + STATE.col_width - 1;
//         let max_row = STATE.top_left.row + STATE.row_width - 1;

//         // Print column headers
//         for i in STATE.top_left.col..=max_col {
//             let header = number_to_column_header(i);
//             print!("{:<width$}", header, width = STATE.cell_width);
//         }
//         println!();

//         // Print rows
//         for i in STATE.top_left.row..=max_row {
//             // Print row number
//             print!("{:<width$}", i + 1, width = STATE.cell_width);

//             // Print cells
//             for j in STATE.top_left.col..=max_col {
//                 let mut error = CellError::NoError;
//                 let cell = Cell { row: i, col: j };
//                 let value = backend::get_cell_value(cell, &mut error);

//                 if error != CellError::NoError {
//                     print!("{:<width$}", "ERR", width = STATE.cell_width);
//                 } else {
//                     print!("{:<width$}", value, width = STATE.cell_width);
//                 }
//             }
//             println!();
//         }
//     }
// }

// /// Removes extra spaces from a string, keeping only single spaces between alphanumeric characters
// fn remove_spaces(s: &str) -> String {
//     let chars: Vec<char> = s.chars().collect();
//     let mut result = Vec::new();
//     let mut i = 0;

//     while i < chars.len() {
//         if chars[i].is_whitespace() {
//             // Check if we need to keep a space (alphanumeric on both sides)
//             if !result.is_empty() && result[result.len() - 1].is_alphanumeric()
//                && i + 1 < chars.len() && chars[i + 1].is_alphanumeric() {
//                 result.push(' ');
//             }
//             // Skip consecutive spaces
//             while i < chars.len() && chars[i].is_whitespace() {
//                 i += 1;
//             }
//         } else {
//             result.push(chars[i]);
//             i += 1;
//         }
//     }

//     result.into_iter().collect()
// }

// /// Handles frontend commands like navigation and output control
// fn run_frontend_command(command: &str) -> bool {
//     unsafe {
//         // Handle single-character navigation commands
//         if command.len() == 1 {
//             match command.chars().next().unwrap() {
//                 'w' => {
//                     if STATE.top_left.row >= MAX_WIDTH {
//                         STATE.top_left.row -= MAX_WIDTH;
//                     }
//                 },
//                 's' => {
//                     if STATE.top_left.row + 2 * MAX_WIDTH <= STATE.rows {
//                         STATE.top_left.row += MAX_WIDTH;
//                     }
//                 },
//                 'd' => {
//                     if STATE.top_left.col + 2 * MAX_WIDTH <= STATE.cols {
//                         STATE.top_left.col += MAX_WIDTH;
//                     }
//                 },
//                 'a' => {
//                     if STATE.top_left.col >= MAX_WIDTH {
//                         STATE.top_left.col -= MAX_WIDTH;
//                     }
//                 },
//                 'q' => {
//                     exit(0);
//                 },
//                 _ => return false,
//             }
//             return true;
//         }

//         // Handle other commands
//         if command == "disable_output" {
//             STATE.do_print = false;
//             true
//         } else if command == "enable_output" {
//             STATE.do_print = true;
//             true
//         } else if command.starts_with("scroll_to ") {
//             let cell_address = &command[10..];
//             if let Ok(cell) = parse_cell_reference(cell_address) {
//                 STATE.top_left = cell;
//                 true
//             } else {
//                 false
//             }
//         } else {
//             false
//         }
//     }
// }

// /// Checks if an expression potentially contains errors
// fn does_expression_contain_error(expression: &str) -> bool {
//     if expression.is_empty() {
//         return true;
//     }

//     let chars: Vec<char> = expression.chars().collect();
//     for i in 1..chars.len() {
//         if chars[i].is_whitespace() &&
//            i > 0 && chars[i-1].is_alphanumeric() &&
//            i+1 < chars.len() && chars[i+1].is_alphanumeric() {
//             return true;
//         }
//     }

//     false
// }

// /// Runs a command entered by the user
// fn run_command(command: &str) -> bool {
//     // If first character is a letter A-Z, interpret as cell expression
//     if let Some(first_char) = command.chars().next() {
//         if first_char >= 'A' && first_char <= 'Z' {
//             // Find the end of the cell reference
//             let mut cell_len = 0;
//             for c in command.chars() {
//                 if c == '=' {
//                     break;
//                 }
//                 cell_len += 1;
//             }

//             // Parse cell reference
//             if let Ok(cell) = parse_cell_reference(&command[..cell_len]) {
//                 // Check if the next character is '='
//                 if command.len() > cell_len && command.chars().nth(cell_len) == Some('=') {
//                     // Extract the expression after '='
//                     let expression = &command[cell_len + 1..];

//                     // Parse and set the cell value
//                     let err = backend::set_cell_value(cell, expression);

//                     match err {
//                         ExpressionError::None => true,
//                         _ => false
//                     }
//                 } else {
//                     false
//                 }
//             } else {
//                 false
//             }
//         } else {
//             // Otherwise, interpret as a frontend command
//             run_frontend_command(command)
//         }
//     } else {
//         false
//     }
// }

// /// Gets a line of input from the user
// fn get_line() -> String {
//     let mut input = String::new();
//     io::stdin().read_line(&mut input).expect("Failed to read line");
//     input.trim_end().to_string() // Remove trailing newline
// }

// /// Main console loop
// fn run_console() -> ! {
//     let mut status = "ok".to_string();
//     let mut time_taken = 0.0;

//     loop {
//         print!("[{:.1}] ({}) > ", time_taken, status);
//         io::stdout().flush().unwrap();

//         let buffer = get_line();
//         let start = Instant::now();

//         let clean_buffer = remove_spaces(&buffer);
//         if clean_buffer.is_empty() {
//             continue;
//         }

//         if run_command(&clean_buffer) {
//             status = "ok".to_string();
//         } else {
//             status = "err".to_string();
//         }

//         let end = Instant::now();
//         time_taken = (end - start).as_secs_f64();

//         print_board();
//     }
// }

// /// Initializes the frontend
// pub fn init_frontend(row: usize, col: usize) {
//     unsafe {
//         STATE.rows = row;
//         STATE.cols = col;
//         backend::init_backend(row, col);

//         STATE.top_left.row = 0;
//         STATE.top_left.col = 0;
//         STATE.row_width = min(MAX_WIDTH, row);
//         STATE.col_width = min(MAX_WIDTH, col);

//         print_board();
//         run_console();
//     }
// }

// // use crate::cell::Cell;

// // static mut DO_PRINT : bool = true;
// // static mut ROWS : i32 = 0;
// // static mut COLS : i32 = 0;
// // static mut TOP_LEFT: Cell = Cell { row: 0, col: 0 };

// // fn number_to_column_header(mut number: i32) -> String {
// //     number = number + 1;
// //     let mut buffer = String::new();

// //     while number > 0 {
// //         let rem = (number - 1) % 26;
// //         buffer.insert(0, (b'A' + rem as u8) as char);
// //         number = (number - 1) / 26;
// //     }
// //     buffer
// // }

// // fn print_board() {
// //     if !DO_PRINT {
// //         return;
// //     }

// //     let row_width = usize::min(MAX_WIDTH, ROWS - top_left.row);
// //     let col_width = usize::min(MAX_WIDTH, COLS - top_left.col);
// //     let max_col = top_left.col + col_width - 1;
// //     let max_row = top_left.row + row_width - 1;

// //     print!("{:width$}", "", width = CELL_WIDTH);

// //     for i in top_left.col..=max_col {
// //         let header = number_to_column_header(i);
// //         print!("{:width$}", header, width = CELL_WIDTH);
// //     }
// //     println!();

// //     for i in top_left.row..=max_row {
// //         print!("{:width$}", i + 1, width = CELL_WIDTH);

// //         for j in top_left.col..=max_col {
// //             let cell = Cell { row: i, col: j };
// //             let (value, error) = get_cell_value(cell);

// //             if error != CellError::NoError {
// //                 print!("{:width$}", "ERR", width = CELL_WIDTH);
// //             } else {
// //                 print!("{:width$}", value, width = CELL_WIDTH);
// //             }
// //         }
// //         println!();
// //     }
// // }

// // fn init_frontend(row:i32, col: i32) {

// // }

// // fn main() {

// // }
