//mod app;
// mod backend;
// mod parser;
// mod structs;
// mod frontend;
use crate::app;
use yew::prelude::*;
pub fn main() {
    yew::Renderer::<app::App>::new().render();
}
