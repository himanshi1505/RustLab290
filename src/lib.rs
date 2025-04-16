// src/lib.rs

pub mod structs;
pub mod parser;
pub mod backend;

// Re-export commonly used items for convenience
pub use structs::*;
pub use parser::*;
pub use backend::Backend;
