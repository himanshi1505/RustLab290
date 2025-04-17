// src/web/tab_bar.rs
use leptos::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use crate::backend::Backend;

#[component]
pub fn TabBar(backend: RwSignal<Backend>) -> impl IntoView {
    let file_input = create_node_ref::<HtmlInputElement>();
    
    // Save to default filename
    let handle_save = move |_| {
        backend.update(|b| {
            b.save_to_csv("spreadsheet.csv")
                .unwrap_or_else(|e| log::error!("Save failed: {:?}", e));
        });
    };

    // Load from selected file
    let handle_load = move |_| {
        if let Some(input) = file_input.get() {
            let files = input.files().unwrap();
            if files.length() > 0 {
                let file = files.get(0).unwrap();
                let reader = web_sys::FileReader::new().unwrap();
                
                let cb = Closure::wrap(Box::new(move |e: web_sys::ProgressEvent| {
                    let target = e.target().unwrap();
                    let reader = target.dyn_into::<web_sys::FileReader>().unwrap();
                    let content = reader.result().unwrap().as_string().unwrap();
                    
                    backend.update(|b| {
                        b.load_from_csv_content(&content)
                            .unwrap_or_else(|e| log::error!("Load failed: {:?}", e));
                    });
                }) as Box<dyn FnMut(_)>);
                
                reader.set_onload(Some(cb.as_ref().unchecked_ref()));
                cb.forget();
                reader.read_as_text(&file).unwrap();
            }
        }
    };

    // Undo last action
    let handle_undo = move |_| {
        backend.update(|b| b.undo());
    };

    // Redo last undone action
    let handle_redo = move |_| {
        backend.update(|b| b.redo());
    };

    view! {
        <div class="tab-bar">
            <input 
                type="file" 
                accept=".csv" 
                node_ref=file_input 
                style="display: none;"
            />
            <button on:click=handle_save>"Save"</button>
            <button on:click=move |_| file_input.get().unwrap().click()>"Load"</button>
            <button on:click=handle_undo>"Undo"</button>
            <button on:click=handle_redo>"Redo"</button>
        </div>
    }
}
