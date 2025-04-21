//mod components;

use leptos::*;
use log::Level; // <-- FIXED: from the `log` crate
use crate::backend::Backend;
use crate::server_logic;
use crate::components::{Toolbar, FormulaBar, SpreadsheetGrid, CommandBar};

#[component]
fn App() -> impl IntoView {
    let backend = create_rw_signal(Backend::new(10, 10));

    view! {
        <main>
            <Toolbar backend=backend/>
            <FormulaBar />
            <SpreadsheetGrid />
            <CommandBar />
        </main>
    }
}

#[cfg(target_arch = "wasm32")]
pub fn main() {
    _ = console_log::init_with_level(Level::Debug); // for browser console logging
    console_error_panic_hook::set_once(); // to see errors in browser devtools
    mount_to_body(|| view! { <App/> });  // mounts App component to the DOM
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    use actix_web::*;
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use crate::app::*; // replace with actual module path

    simple_logger::init_with_level(log::Level::Debug).expect("logging failed");

    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr.clone();
    let leptos_options = conf.leptos_options;

    let routes = generate_route_list(|| view! { <App/> });

    HttpServer::new(move || {
        App::new().service(
            LeptosRoutes::new(leptos_options.clone(), routes.clone(), || {
                view! { <App/> }
            })
        )
    })
    .bind(&addr)?
    .run()
    .await
}

