//mod app;
// mod backend;
// mod parser;
// mod structs;
// mod frontend;
use crate::app;
use yew::prelude::*;
pub fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<app::App>::new().render();
}
