use leptos::*;

#[component]
pub fn Toolbar() -> impl IntoView {
    view! {
        <div class="toolbar">
            <button>"New"</button>
            <button>"Open"</button>
            <button>"Save"</button>
            <button>"Save As"</button>
            <button>"Undo"</button>
            <button>"Redo"</button>
        </div>
    }
}

