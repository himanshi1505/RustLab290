// src/web/components/formula_bar.rs
use leptos::*;
use crate::{backend::Backend, structs::Cell};

#[component]
pub fn FormulaBar(
    formula_input: ReadSignal<String>,
    set_formula_input: WriteSignal<String>,
    selected_cell: ReadSignal<Option<Cell>>,
    backend: RwSignal<Backend>,
) -> impl IntoView {
    let submit_formula = move |_| {
        if let Some(cell) = selected_cell() {
            let expr = formula_input.get();
            let _ = backend.with(|b| b.set_cell_value(cell, &expr));
        }
    };

    view! {
        <div class="formula-bar">
            <input
                type="text"
                value=formula_input
                on:input=move |ev| set_formula_input.set(event_target_value(&ev))
                on:keydown=move |ev| {
                    if ev.key() == "Enter" { submit_formula(()) }
                }
            />
            <button on:click=submit_formula>"Update"</button>
        </div>
    }
}
