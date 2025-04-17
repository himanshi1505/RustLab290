use leptos::*;
use crate::{backend::Backend, structs::Cell};
use web_sys::{MouseEvent, KeyboardEvent};

#[component]
pub fn FormulaBar(
    formula_input: ReadSignal<String>,
    set_formula_input: WriteSignal<String>,
    selected_cell: ReadSignal<Option<Cell>>,
    backend: RwSignal<Backend>,
) -> impl IntoView {
    let submit_formula = move |_event: MouseEvent| {
        if let Some(cell) = selected_cell.get() {
            let expr = formula_input.get();
            backend.update(|b| {
                let _ = b.set_cell_value(cell, &expr);
            });
        }
    };

    let handle_keydown = move |ev: KeyboardEvent| {
        if ev.key() == "Enter" {
            if let Some(cell) = selected_cell.get() {
                let expr = formula_input.get();
                backend.update(|b| {
                    let _ = b.set_cell_value(cell, &expr);
                });
            }
        }
    };

    view! {
        <div class="formula-bar">
            <input
                type="text"
                prop:value=formula_input
                on:input=move |ev| set_formula_input.set(event_target_value(&ev))
                on:keydown=handle_keydown
            />
            <button on:click=submit_formula>"Update"</button>
        </div>
    }
}
