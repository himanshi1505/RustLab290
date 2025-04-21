#[cfg(feature = "gui")]
mod app;
mod backend;
mod frontend;
mod structs;
mod parser;

#[cfg(feature = "gui")]
mod main_gui;
#[cfg(feature = "cli")]
mod cli;

#[cfg(feature = "gui")]
fn main() {
    main_gui::main();
}

#[cfg(feature = "cli")]
fn main() {
    cli::main();
}


//mod backend;
//mod frontend;
//mod cell;
//mod structs;
//mod parser;
//
//use std::env;
//use std::process;
//use frontend::Frontend;
//fn main() {
//    let mut rows = 100;
//    let mut cols = 100;
//
//    let args: Vec<String> = env::args().collect();
//    if args.len() == 3 {
//        match args[1].parse::<usize>() {
//            Ok(r) => rows = r,
//            Err(_) => {
//                eprintln!("Invalid argument for rows: {}", args[1]);
//                process::exit(1);
//            }
//        }
//        
//        match args[2].parse::<usize>() {
//            Ok(c) => cols = c,
//            Err(_) => {
//                eprintln!("Invalid argument for columns: {}", args[2]);
//                process::exit(1);
//            }
//        }
//    } else if args.len() > 1 {
//        eprintln!("Usage: {} [rows columns]", args[0]);
//        process::exit(1);
//    }
//    if(rows>999 || rows<1 || cols>18278 || cols<1){
//        // eprintln!("Invalid argument for rows or columns: {} {}", rows, cols);
//        process::exit(1);
//    }
//    // println!("Initializing with {} rows and {} columns", rows, cols);
//    // TODO: Initialize frontend with rows and cols
//    // init_frontend(rows, cols);
//    let mut frontend = Frontend::new(rows, cols);
//    frontend.print_board();
//    frontend.run();
//
//}
