#[cfg(feature = "gui")]
mod app;
mod backend;
mod frontend;
mod parser;
mod structs;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "gui")]
mod main_gui;

#[cfg(feature = "gui")]
fn main() {
    main_gui::main();
}

#[cfg(feature = "cli")]
fn main() {
    cli::main();
}
mod tests {
    // #[test]
    // #[cfg(feature = "gui")]
    // fn test_main_gui() {
    //     // Ensure the `main_gui` module is accessible and callable
    //     use super::main_gui;
    //     // Call a function from `main_gui` to ensure it compiles and runs
    //     // Replace `main_gui::main()` with a testable function if available
    //     assert!(std::panic::catch_unwind(|| main_gui::main()).is_ok());
    // }

    #[test]
    #[cfg(feature = "cli")]
    fn test_main_cli() {
        // Ensure the `cli` module is accessible and callable
        use super::backend;
        // Call a function from `cli` to ensure it compiles and runs
        // Replace `cli::main()` with a testable function if available
        assert!(std::panic::catch_unwind(|| backend::Backend::new(10, 10)).is_ok());
    }
}
