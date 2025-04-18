// src/web/command_bar.rs
use leptos::*;
use web_sys::MouseEvent;

#[component]
pub fn CommandBar(
    on_command: Callback<String>,
) -> impl IntoView {
    let (input_value, set_input_value) = create_signal("".to_string());

    view! {
        <div class="command-bar">
            <input
                type="text"
                class="command-input"
                placeholder="Enter command or cell formula (e.g. A1=5)"
                prop:value=input_value
                on:input=move |ev| set_input_value.set(event_target_value(&ev))
                on:keydown=move |ev| {
                    if ev.key() == "Enter" {
                        let cmd = input_value.get().trim().to_string();
                        if !cmd.is_empty() {
                            on_command.call(cmd);
                            set_input_value.set("".to_string());
                        }
                    }
                }
            />
            <button class="command-execute" on:click=move |_| {
                let cmd = input_value.get().trim().to_string();
                if !cmd.is_empty() {
                    on_command.call(cmd);
                    set_input_value.set("".to_string());
                }
            }>
                "Execute"
            </button>
        </div>
    }
}
