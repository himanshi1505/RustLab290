use crate::frontend::Frontend;
//use std::env;
use std::process;

pub fn run_cli(args: Vec<String>) -> Result<(), String> {
    let mut rows = 100;
    let mut cols = 100;

    if args.len() == 3 {
        match args[1].parse::<usize>() {
            Ok(r) => rows = r,
            Err(_) => return Err(format!("Invalid argument for rows: {}", args[1])),
        }

        match args[2].parse::<usize>() {
            Ok(c) => cols = c,
            Err(_) => return Err(format!("Invalid argument for columns: {}", args[2])),
        }
    } else if args.len() > 1 {
        return Err(format!("Usage: {} [rows columns]", args[0]));
    }

    if !(1..=999).contains(&rows) || !(1..=18278).contains(&cols) {
        return Err(format!(
            "Invalid argument for rows or columns: {} {}",
            rows, cols
        ));
    }

    let mut frontend = Frontend::new(rows, cols);
    frontend.print_board();
    frontend.run();

    Ok(())
}

// #[cfg_attr(tarpaulin, skip)]
pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(err) = run_cli(args) {
        eprintln!("{}", err);
        process::exit(1);
    }
}
#[cfg(feature = "cli")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_cli_invalid_rows() {
        let args = vec![
            "spreadsheet".to_string(),
            "invalid".to_string(),
            "20".to_string(),
        ];
        let result = run_cli(args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid argument for rows: invalid");
    }

    #[test]
    fn test_run_cli_invalid_columns() {
        let args = vec![
            "spreadsheet".to_string(),
            "10".to_string(),
            "invalid".to_string(),
        ];
        let result = run_cli(args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid argument for columns: invalid");
    }

    #[test]
    fn test_run_cli_out_of_bounds_rows() {
        let args = vec![
            "spreadsheet".to_string(),
            "1000".to_string(),
            "20".to_string(),
        ];
        let result = run_cli(args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Invalid argument for rows or columns: 1000 20"
        );
    }

    #[test]
    fn test_run_cli_out_of_bounds_columns() {
        let args = vec![
            "program_name".to_string(),
            "10".to_string(),
            "20000".to_string(),
        ];
        let result = run_cli(args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Invalid argument for rows or columns: 10 20000"
        );
    }

    #[test]
    fn test_run_cli_usage_error() {
        let args = vec!["program_name".to_string(), "10".to_string()];
        let result = run_cli(args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Usage: program_name [rows columns]");
    }
}
