// Removed incorrect import
use yew::prelude::*;
use std::collections::VecDeque;
use std::cell::UnsafeCell;
use std::rc::Rc;
use std::cell::RefCell;
use web_sys::FileReader;


use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use gloo::file::callbacks::read_as_text;


use gloo::file::File;
use gloo::utils::window;



use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, HtmlInputElement, Event, ProgressEvent, Url};
use crate::backend::Backend;
use crate::frontend::Frontend;
use crate::structs::{Cell, Operand, OperandType, OperandData, CellData, Function, CellError};

#[derive(Properties, PartialEq)]
pub struct GridProps {
    pub frontend: UseStateHandle<Rc<RefCell<Frontend>>>,
    pub update_trigger: UseStateHandle<i32>,
    pub selected_cell: UseStateHandle<(usize, usize)>,
    pub rows: usize,
    pub cols: usize,
}

#[derive(Properties, PartialEq)]
pub struct FormulaBarProps {
    pub frontend: UseStateHandle<Rc<RefCell<Frontend>>>,
    pub selected_cell: UseStateHandle<(usize, usize)>,
}

#[derive(Properties, PartialEq)]
pub struct CommandBarProps {
    pub frontend: UseStateHandle<Rc<RefCell<Frontend>>>,
    pub update_trigger: UseStateHandle<i32>,
}


#[derive(Properties, PartialEq)]
pub struct TabBarProps {
    pub frontend: UseStateHandle<Rc<RefCell<Frontend>>>,
    pub update_trigger: UseStateHandle<i32>,
    pub rows: usize,
    pub cols: usize,
}

#[function_component(App)]
pub fn app() -> Html {
    let rows = 100;  // Large number of rows for scrolling
    let cols = 50;   // Large number of columns for scrolling
    let frontend = use_state(|| Rc::new(RefCell::new(Frontend::new(rows, cols))));
    let update_trigger = use_state(|| 0);
    let selected_cell = use_state(|| (0, 0));
    
    html! {
        <div style="
            display: flex; 
            flex-direction: column; 
            height: 100vh;
            overflow: hidden;
        ">
            <TabBar 
                frontend={frontend.clone()} 
                update_trigger={update_trigger.clone()}
                rows={rows}
                cols={cols}
            />
            <FormulaBar 
                frontend={frontend.clone()}
                selected_cell={selected_cell.clone()}
            />
            <div style="
                flex: 1; 
                overflow: auto;
                position: relative;
            ">
                <Grid 
                    frontend={frontend.clone()} 
                    update_trigger={update_trigger.clone()}
                    selected_cell={selected_cell.clone()}
                    rows={rows}
                    cols={cols}
                />
            </div>
            <CommandBar 
                frontend={frontend.clone()} 
                update_trigger={update_trigger.clone()}
            />
        </div>
    }
}

#[function_component(Grid)]
pub fn grid(props: &GridProps) -> Html {
    let _ = &props.update_trigger; // track changes
    let frontend = props.frontend.clone();
    let mut frontend = frontend.borrow_mut();
    let selected_cell = props.selected_cell.clone();
    let backend = frontend.get_backend_mut();
    
   

    
     // Fixed dimensions - set width to accommodate "WWW" comfortably
     const CELL_WIDTH: &str = "80px";  // Wide enough for "WWW"
     const CELL_HEIGHT: &str = "24px";
     const HEADER_COLOR: &str = "#f0f0f0";
 
     // Function to convert column index to letter (0 -> A, 1 -> B, etc.)
     fn col_to_letter(col: usize) -> String {
         let mut result = String::new();
         let mut n = col;
         while n >= 0 {
             result.insert(0, (b'A' + (n % 26) as u8) as char);
             if n < 26 { break; }
             n = n / 26 - 1;
         }
         result
     }
 
     html! {
        <div style="overflow: auto; height: 100%; width: 100%;">
        <table style={format!(
            "border-collapse: collapse;
            table-layout: fixed;
            width: calc(30px + {} * {});",
            props.cols, CELL_WIDTH
        )}> <thead>
                     <tr style="height: {CELL_HEIGHT};">
                         <th style="
                             width: 30px;
                             background: {HEADER_COLOR};
                             border: 1px solid #ddd;
                             position: sticky;
                             top: 0;
                             z-index: 2;
                         "></th>
                         {(0..props.cols).map(|col| {
                             let letter = col_to_letter(col);
                             html! {
                                 <th 
                                     key={format!("col-{}", col)}
                                     style="
                                         width: {CELL_WIDTH};
                                         background: {HEADER_COLOR};
                                         border: 1px solid #ddd;
                                         text-align: center;
                                         font-weight: bold;
                                         position: sticky;
                                         top: 0;
                                         z-index: 1;
                                         overflow: hidden;
                                         text-overflow: ellipsis;
                                     "
                                 >
                                     {letter}
                                 </th>
                             }
                         }).collect::<Html>()}
                     </tr>
                 </thead>
                 <tbody>
                     {(0..props.rows).map(|row| {
                         html! {
                             <tr key={row.to_string()} style="height: {CELL_HEIGHT};">
                                 <td style="
                                     width: 30px;
                                     background: {HEADER_COLOR};
                                     border: 1px solid #ddd;
                                     text-align: center;
                                     font-weight: bold;
                                     position: sticky;
                                     left: 0;
                                     z-index: 1;
                                 ">
                                     {row + 1}
                                 </td>
                                 {(0..props.cols).map(|col| {
                                     let key = format!("{}-{}", row, col);
                                     let val = unsafe { 
                                         backend.get_cell_value(row, col).value.to_string()
                                     };
                                     let is_selected = *selected_cell == (row, col);
                                     
                                     let cell_style = format!("
                                         width: {CELL_WIDTH};
                                         height: {CELL_HEIGHT};
                                         border: 1px solid #ddd;
                                         padding: 2px;
                                         background-color: {};
                                         text-align: left;
                                         vertical-align: middle;
                                         overflow: hidden;
                                         text-overflow: ellipsis;
                                         white-space: nowrap;
                                     ", if is_selected { "#e6f3ff" } else { "white" });
 
                                     let onclick = {
                                         let selected_cell = selected_cell.clone();
                                         Callback::from(move |_| {
                                             selected_cell.set((row, col));
                                         })
                                     };
 
                                     html! {
                                         <td 
                                             key={key}
                                             style={cell_style}
                                             {onclick}
                                         >
                                             {val}
                                         </td>
                                     }
                                 }).collect::<Html>()}
                             </tr>
                         }
                     }).collect::<Html>()}
                 </tbody>
             </table>
         </div>
     }
 }
// #[function_component(FormulaBar)]
// pub fn formula_bar() -> Html {
//     let formula = use_state(|| String::new());
    
//     let oninput = {
//         let formula = formula.clone();
//         Callback::from(move |e: InputEvent| {
//             let input: web_sys::HtmlInputElement = e.target_unchecked_into();
//             formula.set(input.value());
//         })
//     };

//     html! {
//         <div style="padding: 10px; border-bottom: 1px solid #ccc;">
//             <input 
//                 type="text" 
//                 placeholder="=SUM(A1:A5)" 
//                 style="width: 100%;"
//                 value={(*formula).clone()}
//                 oninput={oninput}
//             />
//         </div>
//     }
// }
//#[function_component(FormulaBar)]
// pub fn formula_bar() -> Html {
//     html! {
//         <div style="padding: 10px; border-bottom: 1px solid #ccc;">
//             <input type="text" placeholder="=SUM(A1:A5)" style="width: 100%;" />
//         </div>
//     }
// }
#[function_component(FormulaBar)]
pub fn formula_bar(props: &FormulaBarProps) -> Html {
    let frontend = props.frontend.clone();
    let mut frontend = frontend.borrow_mut();
    let selected_cell = props.selected_cell.clone();
    
    // Get the formula for the selected cell
    let formula = {
       
        let backend = frontend.get_backend_mut();
        let (row, col) = *selected_cell;
        backend.formula_strings[row][col].clone()
    };

    html! {
        <div style="padding: 10px; border-bottom: 1px solid #ccc;">
            <input 
                type="text" 
                placeholder="=SUM(A1:A5)" 
                style="width: 100%;" 
                value={formula}
                readonly=true
            />
        </div>
    }
}
// #[function_component(CommandBar)]
// pub fn command_bar() -> Html {
//     html! {
//         <div style="padding: 10px; background-color: #f4f4f4;">
//             <input type="text" placeholder="Enter command here..." style="width: 100%;" />
//         </div>
//     }
// }

#[function_component(CommandBar)]
pub fn command_bar(props: &CommandBarProps) -> Html {
    let input_value = use_state(|| String::new());
    let status = use_state(|| None::<bool>);
    let input_ref = use_node_ref();

    // 👇 Clone fields you need from props before the closure
    let update_trigger = props.update_trigger.clone();
    let frontend = props.frontend.clone();

    let onkeypress = {
        let input_value = input_value.clone();
        let status = status.clone();
        let input_ref = input_ref.clone();
        let update_trigger = update_trigger.clone(); // 👈 move into closure
        let frontend = frontend.clone();

        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let current_value = (*input_value).clone();

                let mut frontend = frontend.borrow_mut();
                let result = frontend.run_command(&current_value);

                if result {
                    update_trigger.set(*update_trigger + 1); // ✅ now safe
                }

                status.set(Some(result));

                if let Some(input) = input_ref.cast::<web_sys::HtmlInputElement>() {
                    input.set_value("");
                    input_value.set(String::new());
                }
            }
        })
    };

    let oninput = {
        let input_value = input_value.clone();

        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            input_value.set(input.value());
        })
    };

    let status_display = match *status {
        Some(true) => html! { <span style="color: green; margin-right: 10px; min-width: 40px;">{ "OK" }</span> },
        Some(false) => html! { <span style="color: red; margin-right: 10px; min-width: 40px;">{ "Error" }</span> },
        None => html! { <span style="margin-right: 10px; min-width: 40px;"></span> },
    };

    html! {
        <div style="padding: 10px; background-color: #f4f4f4; display: flex; align-items: center;">
            { status_display }
            <input 
                ref={input_ref}
                type="text" 
                placeholder="Enter command here..." 
                style="width: 100%;"
                value={(*input_value).clone()}
                {oninput}
                {onkeypress}
            />
        </div>
    }
}


// #[function_component(TabBar)]
// pub fn tab_bar(props: &TabBarProps) -> Html {
//     let frontend = props.frontend.clone();
//     let update_trigger = props.update_trigger.clone();
//     let status_message = use_state(|| String::new()); // New state for status messages

//     let save_onclick = {
//         let frontend = frontend.clone();
//         let update_trigger = update_trigger.clone();
//         let status_message = status_message.clone();
//         Callback::from(move |_| {
//             let mut frontend = frontend.borrow_mut();
//             let backend = frontend.get_backend_mut();
//             match backend.save_to_csv("save.csv") {
//                 Ok(_) => {
//                     status_message.set("File saved successfully".to_string());
//                     update_trigger.set(*update_trigger + 1);
//                 }
//                 Err(e) => {
//                     status_message.set(format!("Save failed: {}", e));
//                 }
//             }
//         })
//     };

//     let load_onclick = {
//         let frontend = frontend.clone();
//         let update_trigger = update_trigger.clone();
//         let status_message = status_message.clone();
//         Callback::from(move |_| {
//             let mut frontend = frontend.borrow_mut();
//             let backend = frontend.get_backend_mut();
//             match backend.load_csv("save.csv", false) {
//                 Ok(_) => {
//                     status_message.set("File loaded successfully".to_string());
//                     update_trigger.set(*update_trigger + 1);
//                 }
//                 Err(e) => {
//                     status_message.set(format!("Load failed: {}", e));
//                 }
//                  // Clear the message after displaying
            
//             }
//             status_message.set("".to_string());
//         })
//     };

//     html! {
//         <div style="background-color: #eee; padding: 10px; display: flex; align-items: center; gap: 10px;">
//             <button onclick={save_onclick}>{ "Save" }</button>
//             <button onclick={load_onclick}>{ "Load" }</button>
//             <div style={format!("margin-left: 10px; color: {};", if status_message.contains("failed") { "#ff0000" } else { "#00ff00" })}>
//                 { &*status_message }
               
//             </div>
//         </div>
//     }
// }

pub fn download_csv(content: String, filename: &str) {
    let array = js_sys::Array::new();
    array.push(&JsValue::from_str(&content));

    let blob = Blob::new_with_str_sequence_and_options(
        &array,
        BlobPropertyBag::new().type_("text/csv"),
    ).unwrap();

    let url = Url::create_object_url_with_blob(&blob).unwrap();

    let document = window().document().unwrap();
    let a = document.create_element("a").unwrap().dyn_into::<web_sys::HtmlAnchorElement>().unwrap();
    a.set_href(&url);
    a.set_download(filename);
    a.click();
    Url::revoke_object_url(&url).unwrap();
}
#[function_component(TabBar)]
pub fn tab_bar(props: &TabBarProps) -> Html {
    let frontend = props.frontend.clone();
    let update_trigger = props.update_trigger.clone();
    let status_message = use_state(|| String::new());
    let file_input_ref = use_node_ref();
    let rows = props.rows;
    let cols = props.cols;

    // Save functionality
    let save_onclick = {
        let frontend = frontend.clone();
        let status_message = status_message.clone();
        
        Callback::from(move |_| {
            let mut frontend = frontend.borrow_mut();
            let backend = frontend.get_backend_mut();
            
            // Generate CSV content
            let mut csv = String::new();
            for row in 0..rows {
                let mut line = Vec::new();
                for col in 0..cols {
                    unsafe {
                        let celldata = backend.get_cell_value(row, col);
                        let val = if celldata.error == CellError::NoError {
                            celldata.value.to_string()
                        } else {
                            "Error".to_string()
                        };
                        line.push(val);
                    }
                }
                csv.push_str(&line.join(","));
                csv.push('\n');
            }
            
            // Trigger download
            download_csv(csv, "spreadsheet.csv");
            status_message.set("File saved successfully".to_string());
            
            // Clear message after 3 seconds
            let status_message = status_message.clone();
            gloo::timers::callback::Timeout::new(3000, move || {
                status_message.set(String::new());
            }).forget();
        })
    };
    
    // Load functionality
    let load_onclick = {
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };
    
    let on_file_change = {
        let frontend = frontend.clone();
        let update_trigger = update_trigger.clone();
        let status_message = status_message.clone();
    
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(file_list) = input.files() {
                if file_list.length() > 0 {
                    let file = file_list.get(0).unwrap();
                    let reader = FileReader::new().unwrap();
    
                    let frontend = frontend.clone();
                    let update_trigger = update_trigger.clone();
                    let status_message = status_message.clone();
    
                    // Clone the `reader` to avoid moving it
                    let reader_clone = reader.clone();
                    let onload = Closure::wrap(Box::new(move |_e: ProgressEvent| {
                        if let Ok(result) = reader_clone.result() {
                            if let Some(text) = result.as_string() {
                                let mut frontend = frontend.borrow_mut();
                                let backend = frontend.get_backend_mut();
    
                                match backend.load_csv_from_str(&text) {
                                    Ok(_) => {
                                        status_message.set("File loaded successfully".to_string());
                                        update_trigger.set(*update_trigger + 1);
                                    }
                                    Err(e) => {
                                        status_message.set(format!("Load failed: {}", e));
                                    }
                                }
    
                                // Clear message after 3 seconds
                                let status_message = status_message.clone();
                                gloo::timers::callback::Timeout::new(3000, move || {
                                    status_message.set(String::new());
                                })
                                .forget();
                            }
                        }
                    }) as Box<dyn FnMut(_)>);
    
                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                    reader.read_as_text(&file).unwrap();
                    onload.forget();
                }
            }
        })
    };

    html! {
        <div style="background-color: #eee; padding: 10px; display: flex; align-items: center; gap: 10px;">
            <button onclick={save_onclick}>{ "Save" }</button>
            <button onclick={load_onclick}>{ "Load" }</button>
            <input
                type="file"
                accept=".csv"
                ref={file_input_ref}
                onchange={on_file_change}
                style="display: none;"
            />
            <div style={format!(
                "margin-left: 10px; color: {}; transition: opacity 0.5s;",
                if status_message.contains("failed") { "#ff0000" } else { "#00aa00" }
            )}>
                { if !status_message.is_empty() { &*status_message } else { "" } }
            </div>
        </div>
    }
}