// src/lib.rs

pub mod backend;
pub mod parser;
pub mod structs;

// Re-export commonly used items for convenience
pub use backend::Backend;
pub use parser::*;
pub use structs::*;
