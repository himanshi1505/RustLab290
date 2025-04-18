#[cfg(feature = "gui")]
use leptos::*;

#[cfg(feature = "gui")]
#[component]
pub fn FormulaBar() -> impl IntoView {
    let (formula, set_formula) = create_signal("".to_string());

    view! {
        <div class="formula-bar">
            <input
                type="text"
                placeholder="A1 = B1 + C1"
                prop:value=move || formula.get()
                on:input=move |e| set_formula.set(event_target_value(&e))
            />
        </div>
    }
}

