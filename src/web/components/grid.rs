// src/web/components/grid.rs
use leptos::*;
use crate::{backend::Backend, structs::Cell};

#[component]
pub fn Grid(
    backend: RwSignal<Backend>,
    selected_cell: RwSignal<Option<Cell>>,
    set_selected_cell: WriteSignal<Option<Cell>>,
) -> impl IntoView {
    let (rows, cols) = move || {
        let b = backend.get();
        (b.rows, b.cols)
    };

    view! {
        <div class="grid-container">
            <div class="grid-header">
                <div class="corner-cell"></div>
                {move || (0..cols()).map(|col| view! { 
                    <div class="col-header">
                        {column_to_letter(col)}
                    </div> 
                }).collect_view()}
            </div>
            
            {move || (0..rows()).map(|row| view! {
                <div class="grid-row">
                    <div class="row-header">{row + 1}</div>
                    {move || (0..cols()).map(|col| {
                        let cell = Cell { row, col };
                        let is_selected = move || selected_cell() == Some(cell);
                        
                        view! {
                            <CellComponent 
                                cell=cell
                                is_selected=is_selected
                                set_selected_cell=set_selected_cell
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
    set_selected_cell: WriteSignal<Option<Cell>>,
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
            class:selected=move || is_selected()
            on:click=move |_| set_selected_cell.set(Some(cell))
        >
            {cell_value}
        </button>
    }
}

fn column_to_letter(mut col: usize) -> String {
    let mut letters = String::new();
    while col >= 0 {
        letters.insert(0, (b'A' + (col % 26) as u8) as char);
        col = col / 26 - 1;
    }
    letters
}
