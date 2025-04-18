// src/web/tab_bar.rs
use leptos::prelude::*;
use crate::backend::Backend;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[component]
pub fn TabBar(backend: RwSignal<Backend>) -> impl IntoView {
    let file_input = create_node_ref::<HtmlInputElement>();
    
    let handle_save = move |_| {
        backend.update(|b| b.save_to_csv("spreadsheet.csv"));
    };

    let handle_load = move |_| {
        if let Some(input) = file_input.get() {
            let files = input.files().unwrap();
            if files.length() > 0 {
                let file = files.get(0().unwrap());
                let reader = web_sys::FileReader::new().unwrap();
                
                reader.onload(Closure::wrap(Box::new(move |e: web_sys::ProgressEvent| {
                    let content = e.target().unwrap().dyn_into::<web_sys::FileReader>().unwrap().result().unwrap().as_string().unwrap();
                    backend.update(|b| b.load_from_csv_content(&content));
                }) as Box<dyn FnMut(_)>));
                
                reader.read_as_text(&file).unwrap();
            }
        }
    };

    view! {
        <div class="tab-bar">
            <input type="file" accept=".csv" node_ref=file_input style="display: none;"/>
            <button on:click=handle_save>"Save"</button>
            <button on:click=move |_| file_input.get().unwrap().click()>"Load"</button>
            <button on:click=move |_| backend.update(|b| b.undo())>"Undo"</button>
            <button on:click=move |_| backend.update(|b| b.redo())>"Redo"</button>
        </div>
    }
}
