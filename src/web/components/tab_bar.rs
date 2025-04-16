// src/web/components/tab_bar.rs
use leptos::*;
use crate::backend::Backend;

#[component]
pub fn TabBar(backend: RwSignal<Backend>) -> impl IntoView {
    let save = move |_| {
        // Implement save functionality
        log::info!("Saving spreadsheet");
    };

    view! {
        <div class="tab-bar">
            <button on:click=save>"Save"</button>
            <button>"Save As"</button>
            <button>"Open"</button>
            <button>"Undo"</button>
            <button>"Redo"</button>
        </div>
    }
}
