// src/web/app.rs
use leptos::*;
use crate::{backend::Backend, structs::Cell};

#[component]
pub fn WebApp() -> impl IntoView {
    let (rows, cols) = (100, 100);
    let backend = RwSignal::new(Backend::new(rows, cols));
    let (selected_cell, set_selected_cell) = create_signal::<Option<Cell>>(None);
    let (formula_input, set_formula_input) = create_signal("".to_string());

    // Update formula bar when cell selection changes
    create_effect(move |_| {
        if let Some(cell) = selected_cell() {
            if let Some(cell_data) = backend.get().get_cell_value(&cell) {
                let func = &cell_data.borrow().function;
                set_formula_input.set(match func.type_ {
                    FunctionType::Constant => cell_data.borrow().value.to_string(),
                    _ => format!("{:?}", func)
                });
            }
        }
    });

    view! {
        <div class="app-container">
            <TabBar backend=backend/>
            <FormulaBar 
                formula_input=formula_input
                set_formula_input=set_formula_input
                selected_cell=selected_cell
                backend=backend
            />
            <Grid 
                backend=backend
                selected_cell=selected_cell
                set_selected_cell=set_selected_cell
            />
            <CommandBar on_command=handle_command />
        </div>
    }
      
    let handle_command = move |cmd: String| {
        // Implement command handling logic here
        // This should integrate with your existing backend
        log::info!("Executing command: {}", cmd);
    };

    
    
}
