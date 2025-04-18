// src/web/grid.rs

use leptos::prelude::*;
use crate::{backend::Backend, structs::Cell};

#[component]
pub fn Grid(
    backend: RwSignal<Backend>,
    selected_cell: RwSignal<Option<Cell>>,
    set_selected_cell: WriteSignal<Option<Cell>>,
    selection_start: ReadSignal<Option<Cell>>,
    current_selection: ReadSignal<Option<Cell>>,
    on_mouse_down: Callback<Cell>,
    on_mouse_over: Callback<Cell>,
) -> impl IntoView {
    let rows = move || backend.get().rows;
    let cols = move || backend.get().cols;

    view! {
        <div class="grid-container">
            <div class="grid-header">
                <div class="corner-cell"></div>
                {(0..cols()).map(|col| view! { 
                    <div class="col-header">{column_to_letter(col)}</div> 
                }).collect_view()}
            </div>
            
            {(0..rows()).map(|row| view! {
                <div class="grid-row">
                    <div class="row-header">{row + 1}</div>
                    {(0..cols()).map(|col| {
                        let cell = Cell { row, col };
                        let is_selected = move || selected_cell.get() == Some(cell);
                        let in_selection = move || is_in_selection(cell, selection_start.get(), current_selection.get());
                        
                        view! {
                            <CellComponent 
                                cell=cell
                                is_selected=is_selected
                                in_selection=in_selection
                                set_selected_cell=set_selected_cell
                                on_mouse_down=on_mouse_down
                                on_mouse_over=on_mouse_over
                                backend=backend
                            />
                        }
                    }).collect_view()}
                </div>
            }).collect_view()}
        </div>
    }
}

#[component]
fn CellComponent(
    cell: Cell,
    is_selected: impl Fn() -> bool + 'static,
    in_selection: impl Fn() -> bool + 'static,
    set_selected_cell: WriteSignal<Option<Cell>>,
    on_mouse_down: Callback<Cell>,
    on_mouse_over: Callback<Cell>,
    backend: RwSignal<Backend>,
) -> impl IntoView {
    let cell_value = move || {
        backend.with(|b| 
            b.get_cell_value(&cell)
                .map(|rc| rc.borrow().value.to_string())
                .unwrap_or_else(|| "ERR".into())
        )
    };

    view! {
        <button
            class:selected=is_selected
            class:selection=in_selection
            on:mousedown=move |_| on_mouse_down(cell)
            on:mouseover=move |_| on_mouse_over(cell)
            on:click=move |_| set_selected_cell.set(Some(cell))
        >
            {cell_value}
        </button>
    }
}

fn is_in_selection(cell: Cell, start: Option<Cell>, end: Option<Cell>) -> bool {
    match (start, end) {
        (Some(s), Some(e)) => {
            let min_row = s.row.min(e.row);
            let max_row = s.row.max(e.row);
            let min_col = s.col.min(e.col);
            let max_col = s.col.max(e.col);
            cell.row >= min_row && cell.row <= max_row &&
            cell.col >= min_col && cell.col <= max_col
        }
        _ => false
    }
}

fn column_to_letter(mut col: usize) -> String {
    let mut letters = String::new();
    loop {
        let remainder = col % 26;
        letters.insert(0, (b'A' + remainder as u8) as char);
        col = col / 26;
        if col == 0 { break; }
        col -= 1;
    }
    letters
}
