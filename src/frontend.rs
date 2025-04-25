//! # Spreadsheet Frontend Module
#[cfg(feature = "cli")]
use crate::backend::*;

use crate::parser::*;
use crate::structs::*;

#[cfg(feature = "cli")]
use std::cmp::min;
#[cfg(feature = "cli")]
use std::io::{self, Write};
#[cfg(feature = "cli")]
use std::time::Instant;

#[cfg(feature = "gui")]
use crate::backend::Backend;

const MAX_WIDTH: usize = 10;
/// Represents the frontend of the spreadsheet application, handling user input and output.
pub struct Frontend {
    backend: Backend,
    rows: usize,
    cols: usize,
    #[cfg(feature = "cli")]
    cell_width: usize,
    do_print: bool,
    top_left: Cell,
}
/// PartialEq implementation for Frontend, used for GUI comparisons.
#[cfg(feature = "gui")]
impl PartialEq for Frontend {
    fn eq(&self, other: &Self) -> bool {
        self.rows == other.rows && self.cols == other.cols
    }
}
/// Implements the Frontend struct, which manages the spreadsheet interface.
/// This struct is responsible for user interactions, including command parsing and output formatting.
impl Frontend {
    /// Creates a new Frontend instance.
    pub fn new(rows: usize, cols: usize) -> Self {
        let backend = Backend::new(rows, cols);

        Self {
            backend,
            rows,
            cols,
            #[cfg(feature = "cli")]
            cell_width: 12,
            do_print: true,
            top_left: Cell { row: 0, col: 0 },
        }
    }
    /// Returns mutable access to the backend.
    #[cfg(feature = "gui")]
    pub fn get_backend_mut(&mut self) -> &mut Backend {
        &mut self.backend
    }
    /// Converts a column number to a letter-based column header (A, B, ..., Z, AA, ...).
    #[cfg(feature = "cli")]
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
    /// Prints the current visible portion of the spreadsheet.
    #[cfg(feature = "cli")]
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
                unsafe {
                    let data = self.backend.get_cell_value(row, col);

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
    /// Removes extra spaces from a string.
    #[cfg(feature = "cli")]
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
    /// Handles frontend commands like navigation, some gui extensions and output control.
    /// This function interprets commands entered by the user and performs the corresponding actions.
    /// It supports commands for scrolling, loading, saving, copying, cutting, pasting, and autofilling.
    /// It also includes commands for enabling and disabling output.
    /// #Usage:
    /// - `disable_output`: Disables output to the console.
    /// - `enable_output`: Enables output to the console.
    /// - `q`: Exits the program.   
    /// - `w`: Scrolls up.
    /// - `s`: Scrolls down.
    /// - `a`: Scrolls left.
    /// - `d`: Scrolls right.
    /// - `scroll_to <cell>`: Scrolls to a specific cell.
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
                } else {
                    return false;
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("load(") => {
                let res = Backend::load_csv(&mut self.backend, cmd, false);
                match res {
                    Ok(_) => {
                        return true;
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("save(") => {
                println!("save");
                let res = Backend::save_to_csv(&self.backend, cmd);
                match res {
                    Ok(_) => {
                        return true;
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("copy(") => {
                // self.backend.push_undo_state();
                println!("copy");
                let res = Backend::copy(&mut self.backend, cmd);
                match res {
                    Ok(_) => {
                        return true;
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("cut(") => {
                self.backend.push_undo_state();
                let res = Backend::cut(&mut self.backend, cmd);
                match res {
                    Ok(_) => {
                        return true;
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("paste(") => {
                self.backend.push_undo_state();
                let res = Backend::paste(&mut self.backend, cmd);
                match res {
                    Ok(_) => {
                        return true;
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("autofill") => {
                self.backend.push_undo_state();
                let res = Backend::autofill(&mut self.backend, cmd);
                match res {
                    Ok(_) => {
                        return true;
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
            #[cfg(feature = "gui")]
            cmd if cmd.starts_with("sort") => {
                println!("sort");
                self.backend.push_undo_state();
                let res = Backend::sort(&mut self.backend, cmd);
                match res {
                    Ok(_) => {
                        return true;
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
            _ => return false,
        }
        true
    }
    /// Runs a command entered by the user.
    pub fn run_command(&mut self, input: &str) -> bool {
        if input
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            if let Some(eq_pos) = input.find('=') {
                //#[cfg(feature = "gui")]
                //let formula = input[eq_pos..].trim();
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
                                self.backend.formula_strings[row_num][col_num] =
                                    expr_str.to_string();
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
    /// Processes a command entered by the user in the command line interface.
    #[cfg(feature = "cli")]
    pub fn process_command(&mut self, input: &str) -> (String, f64) {
        let mut status = "ok".to_string();
        let start = Instant::now();
        Self::remove_spaces(&mut input.to_string());
        let input = input.trim();
        if input.is_empty() {
            return (status, 0.0);
        }
        if self.run_command(input) {
            status = "ok".to_string();
        } else {
            status = "err".to_string();
        }
        let time_taken = start.elapsed().as_secs_f64();
        self.print_board();
        (status, time_taken)
    }
    /// Runs the command line interface for the spreadsheet.
    #[cfg(feature = "cli")]
    pub fn run(&mut self) {
        let mut status = "ok".to_string();
        let mut time_taken = 0.0;

        loop {
            print!("[{:.1}] ({}) > ", time_taken, status);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                continue;
            }

            // Use the process_command function to handle the input
            let result = self.process_command(&input);
            status = result.0;
            time_taken = result.1;
        }
    }

    // pub fn run(&mut self) {
    //     let mut status = "ok";
    //     let mut time_taken = 0.0;

    //     loop {
    //         print!("[{:.1}] ({}) > ", time_taken, status);
    //         io::stdout().flush().unwrap();

    //         let mut input = String::new();
    //         if io::stdin().read_line(&mut input).is_err() {
    //             continue;
    //         }
    //         let start = Instant::now();
    //         Self::remove_spaces(&mut input);
    //         let input = input.trim();
    //         if input.is_empty() {
    //             continue;
    //         }
    //         if self.run_command(input) {
    //             status = "ok";
    //         } else {
    //             status = "err";
    //         }
    //         time_taken = start.elapsed().as_secs_f64();
    //         self.print_board();
    //     }
    // }
}

#[cfg(test)]
#[cfg(feature = "cli")]
mod tests {
    use super::*;
    use crate::structs::Cell;

    #[test]
    fn test_run_frontend_command_navigation_2() {
        //const MAX_WIDTH: usize = 10;

        let mut backend = Frontend::new(50, 50); // Create a backend with 50x50 grid
        backend.top_left = Cell { row: 20, col: 20 };

        // Test "w" command (move up)
        backend.run_frontend_command("w");
        assert_eq!(backend.top_left.row, 10); // 20 - MAX_WIDTH = 10

        // Test "w" command when already at the top
        backend.top_left.row = 5;
        backend.run_frontend_command("w");
        assert_eq!(backend.top_left.row, 0); // Should not go below 0

        // Test "s" command (move down)
        backend.top_left.row = 30;
        backend.run_frontend_command("s");
        assert_eq!(backend.top_left.row, 40); // 30 + MAX_WIDTH = 40

        // Test "s" command when near the bottom
        backend.top_left.row = 45;
        backend.run_frontend_command("s");
        assert_eq!(backend.top_left.row, 40); // Should not exceed rows - MAX_WIDTH

        // Test "a" command (move left)
        backend.top_left.col = 20;
        backend.run_frontend_command("a");
        assert_eq!(backend.top_left.col, 10); // 20 - MAX_WIDTH = 10

        // Test "a" command when already at the leftmost column
        backend.top_left.col = 5;
        backend.run_frontend_command("a");
        assert_eq!(backend.top_left.col, 0); // Should not go below 0

        // Test "d" command (move right)
        backend.top_left.col = 30;
        backend.run_frontend_command("d");
        assert_eq!(backend.top_left.col, 40); // 30 + MAX_WIDTH = 40

        // Test "d" command when near the rightmost column
        backend.top_left.col = 45;
        backend.run_frontend_command("d");
        assert_eq!(backend.top_left.col, 40); // Should not exceed cols - MAX_WIDTH
    }

    #[test]
    fn test_process_command_valid_set_cell_value() {
        let mut frontend = Frontend::new(5, 5);
        let (status, time_taken) = frontend.process_command("A1=42");
        assert_eq!(status, "ok");
        assert!(time_taken >= 0.0);

        unsafe {
            let cell_data = frontend.backend.get_cell_value(0, 0);
            assert_eq!((*cell_data).value, 42);
        }
    }

    #[test]
    fn test_process_command_invalid_command() {
        let mut frontend = Frontend::new(5, 5);
        let (status, time_taken) = frontend.process_command("invalid_command");
        assert_eq!(status, "err");
        assert!(time_taken >= 0.0);
    }

    #[test]
    fn test_process_command_empty_input() {
        let mut frontend = Frontend::new(5, 5);
        let (status, time_taken) = frontend.process_command("");
        assert_eq!(status, "ok");
        assert_eq!(time_taken, 0.0);
    }

    #[test]
    fn test_process_command_scroll_to_valid() {
        let mut frontend = Frontend::new(5, 5);
        let (status, time_taken) = frontend.process_command("scroll_to A1");
        assert_eq!(status, "ok");
        assert!(time_taken >= 0.0);
        assert_eq!(frontend.top_left, Cell { row: 0, col: 0 });
    }

    #[test]
    fn test_process_command_scroll_to_invalid() {
        let mut frontend = Frontend::new(5, 5);
        let (status, time_taken) = frontend.process_command("scroll_to InvalidCell");
        assert_eq!(status, "err");
        assert!(time_taken >= 0.0);
    }

    #[test]
    fn test_process_command_disable_output() {
        let mut frontend = Frontend::new(5, 5);
        let (status, time_taken) = frontend.process_command("disable_output");
        assert_eq!(status, "ok");
        assert!(time_taken >= 0.0);
        assert!(!frontend.do_print);
    }

    #[test]
    fn test_process_command_enable_output() {
        let mut frontend = Frontend::new(5, 5);
        frontend.process_command("disable_output");
        let (status, time_taken) = frontend.process_command("enable_output");
        assert_eq!(status, "ok");
        assert!(time_taken >= 0.0);
        assert!(frontend.do_print);
    }

    #[test]
    fn test_new_frontend() {
        let frontend = Frontend::new(5, 5);
        assert_eq!(frontend.rows, 5);
        assert_eq!(frontend.cols, 5);
        assert_eq!(frontend.cell_width, 12);
        assert!(frontend.do_print);
        assert_eq!(frontend.top_left, Cell { row: 0, col: 0 });
    }

    #[test]
    fn test_number_to_column_header() {
        assert_eq!(Frontend::number_to_column_header(0), "A");
        assert_eq!(Frontend::number_to_column_header(25), "Z");
        assert_eq!(Frontend::number_to_column_header(26), "AA");
        assert_eq!(Frontend::number_to_column_header(701), "ZZ");
    }

    #[test]
    fn test_print_board_no_output() {
        // Lines 77, 79-81
        let mut frontend = Frontend::new(5, 5);
        frontend.do_print = false;
        frontend.print_board(); // Should not panic or print anything
    }

    #[test]
    fn test_print_board_with_output() {
        // Lines 83, 87, 89, 93
        let mut frontend = Frontend::new(5, 5);
        frontend
            .backend
            .set_cell_value(Cell { row: 0, col: 0 }, "42")
            .unwrap();
        frontend.print_board(); // Should print the board with "42" in cell A1
    }

    #[test]
    fn test_remove_spaces() {
        let mut input = "  Hello   World  ".to_string();
        Frontend::remove_spaces(&mut input);
        assert_eq!(input, "HelloWorld");

        let mut input = "  Rust   Programming  ".to_string();
        Frontend::remove_spaces(&mut input);
        assert_eq!(input, "RustProgramming");

        let mut input = "   ".to_string();
        Frontend::remove_spaces(&mut input);
        assert_eq!(input, "");
    }

    #[test]
    fn test_run_frontend_command_disable_output() {
        let mut frontend = Frontend::new(5, 5);
        frontend.run_frontend_command("disable_output");
        assert!(!frontend.do_print);
    }

    #[test]
    fn test_run_frontend_command_enable_output() {
        let mut frontend = Frontend::new(5, 5);
        frontend.run_frontend_command("disable_output");
        frontend.run_frontend_command("enable_output");
        assert!(frontend.do_print);
    }

    #[test]
    fn test_run_frontend_command_navigation() {
        let mut frontend = Frontend::new(20, 20);

        frontend.run_frontend_command("w");
        assert_eq!(frontend.top_left.row, 0);

        frontend.run_frontend_command("s");
        assert_eq!(frontend.top_left.row, 10);

        frontend.run_frontend_command("a");
        assert_eq!(frontend.top_left.col, 0);

        frontend.run_frontend_command("d");
        assert_eq!(frontend.top_left.col, 10);
    }

    #[test]
    fn test_run_command_set_cell_value() {
        let mut frontend = Frontend::new(5, 5);
        let result = frontend.run_command("A1=42");
        assert!(result);

        unsafe {
            let cell_data = frontend.backend.get_cell_value(0, 0);
            assert_eq!((*cell_data).value, 42);
        }
    }

    #[test]
    fn test_run_command_invalid_command() {
        let mut frontend = Frontend::new(5, 5);
        let result = frontend.run_command("invalid_command");
        assert!(!result);
    }

    #[test]
    fn test_run_command_empty_input() {
        // Lines 293-295
        let mut frontend = Frontend::new(5, 5);
        let result = frontend.run_command("");
        assert!(!result);
    }

    #[test]
    fn test_run_command_scroll_to_valid() {
        // Lines 298-299, 301-302
        let mut frontend = Frontend::new(5, 5);
        let result = frontend.run_command("scroll_to A1");
        assert!(result);
        assert_eq!(frontend.top_left, Cell { row: 0, col: 0 });
    }

    #[test]
    fn test_run_command_scroll_to_invalid() {
        // Lines 305-308, 311-312
        let mut frontend = Frontend::new(5, 5);
        let result = frontend.run_command("scroll_to InvalidCell");
        assert!(!result);
    }
}
