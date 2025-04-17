//mod components;

use leptos::*;
use log::Level; // <-- FIXED: from the `log` crate
use crate::backend::Backend;
use crate::components::{Toolbar, FormulaBar, SpreadsheetGrid};

#[component]
fn App() -> impl IntoView {
    view! {
        <main>
            <Toolbar />
            <FormulaBar />
            <SpreadsheetGrid />
        </main>
    }
}

pub fn main() {
    _ = console_log::init_with_level(Level::Debug); // for browser console logging
    console_error_panic_hook::set_once(); // to see errors in browser devtools
    mount_to_body(|| view! { <App/> });  // mounts App component to the DOM
}
