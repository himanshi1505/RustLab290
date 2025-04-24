use yew::prelude::*;
//use std::collections::VecDeque;
//use std::cell::UnsafeCell;
use std::rc::Rc;
use std::cell::RefCell;
use web_sys::FileReader;


use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
//use gloo::file::callbacks::read_as_text;


//use gloo::file::File;
use gloo::utils::window;



//use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, HtmlInputElement, Event, ProgressEvent, Url};
use web_sys::{Blob, BlobPropertyBag, HtmlInputElement, Event, ProgressEvent, Url};
//use crate::backend::Backend;
use crate::frontend::Frontend;
//use crate::structs::{Cell, Operand, OperandType, OperandData, CellData, Function, CellError};
use crate::structs::CellError;
// Added ThemeType enum to track current theme
#[derive(Clone, PartialEq)]
pub enum ThemeType {
    Light,
    Dark,
}

// Define color constants for both themes
struct ThemeColors {
    background: &'static str,
    text: &'static str,
    border: &'static str,
    header_bg: &'static str,
    cell_bg: &'static str,
    command_bar_bg: &'static str,
    selected_cell_bg: &'static str,
    parent_cell_bg: &'static str,
    child_cell_bg: &'static str,
}

impl ThemeColors {
    fn light() -> Self {
        Self {
            background: "#ffffff",
            text: "#000000",
            border: "#dddddd",
            header_bg: "#f0f0f0",
            cell_bg: "#ffffff",
            command_bar_bg: "#f4f4f4",
            selected_cell_bg: "#e6f3ff",
            parent_cell_bg: "#ffeecc",
            child_cell_bg: "#ccffcc",
        }
    }

    fn dark() -> Self {
        Self {
            background: "#1e1e1e",
            text: "#e0e0e0",
            border: "#444444",
            header_bg: "#2d2d2d",
            cell_bg: "#1e1e1e",
            command_bar_bg: "#2d2d2d",
            selected_cell_bg: "#264f78",
            parent_cell_bg: "#664428",
            child_cell_bg: "#2e6644",
        }
    }

    fn get(theme: &ThemeType) -> Self {
        match theme {
            ThemeType::Light => Self::light(),
            ThemeType::Dark => Self::dark(),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct GridProps {
    pub frontend: UseStateHandle<Rc<RefCell<Frontend>>>,
    pub update_trigger: UseStateHandle<i32>,
    pub selected_cell: UseStateHandle<(usize, usize)>,
    pub rows: usize,
    pub cols: usize,
    pub theme: ThemeType, // Add theme prop
}

#[derive(Properties, PartialEq)]
pub struct FormulaBarProps {
    pub frontend: UseStateHandle<Rc<RefCell<Frontend>>>,
    pub selected_cell: UseStateHandle<(usize, usize)>,
    pub theme: ThemeType, // Add theme prop
}

#[derive(Properties, PartialEq)]
pub struct CommandBarProps {
    pub frontend: UseStateHandle<Rc<RefCell<Frontend>>>,
    pub update_trigger: UseStateHandle<i32>,
    pub theme: ThemeType, // Add theme prop
}


#[derive(Properties, PartialEq)]
pub struct TabBarProps {
    pub frontend: UseStateHandle<Rc<RefCell<Frontend>>>,
    pub update_trigger: UseStateHandle<i32>,
    pub rows: usize,
    pub cols: usize,
    pub theme: UseStateHandle<ThemeType>, // Use UseStateHandle for theme
}

#[function_component(App)]
pub fn app() -> Html {
    let rows = 100;  // Large number of rows for scrolling
    let cols = 50;   // Large number of columns for scrolling
    let frontend = use_state(|| Rc::new(RefCell::new(Frontend::new(rows, cols))));
    let update_trigger = use_state(|| 0);
    let selected_cell = use_state(|| (0, 0));
    let theme = use_state(|| ThemeType::Light); // Initialize with light theme
    
    // Get theme colors
    let colors = ThemeColors::get(&theme);
    
    html! {
        <div style={format!("
            display: flex; 
            flex-direction: column; 
            height: 100vh;
            overflow: hidden;
            background-color: {};
            color: {};
        ", colors.background, colors.text)}>
            <TabBar 
                frontend={frontend.clone()} 
                update_trigger={update_trigger.clone()}
                rows={rows}
                cols={cols}
                theme={theme.clone()}
            />
            <FormulaBar 
                frontend={frontend.clone()}
                selected_cell={selected_cell.clone()}
                theme={(*theme).clone()}
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
                    theme={(*theme).clone()}
                />
            </div>
            <CommandBar 
                frontend={frontend.clone()} 
                update_trigger={update_trigger.clone()}
                theme={(*theme).clone()}
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
    
    // Get theme colors
    let colors = ThemeColors::get(&props.theme);
    
    // Fixed dimensions - set width to accommodate "WWW" comfortably
    const CELL_WIDTH: &str = "80px";  // Wide enough for "WWW"
    const CELL_HEIGHT: &str = "24px";
    
    // Get the current relationships for the selected cell using backend function
    let (parent_cells, child_cells) = {
        let (row, col) = *selected_cell;
        backend.get_cell_dependencies(row, col)
    };
    
    // Function to convert column index to letter (0 -> A, 1 -> B, etc.)
    fn col_to_letter(col: usize) -> String {
        let mut result = String::new();
        let mut n = col as i32;
        while n >= 0 {
            result.insert(0, (b'A' + (n % 26) as u8) as char);
            if n < 26 { break; }
            n = n / 26 - 1;
        }
        result
    }
    
    // Function to determine cell background color based on relationships
    fn get_cell_background_color(
        row: usize, 
        col: usize, 
        selected: (usize, usize), 
        parents: &[(usize, usize)], 
        children: &[(usize, usize)],
        colors: &ThemeColors,
    ) -> &'static str {
        if (row, col) == selected {
            colors.selected_cell_bg
        } else if parents.contains(&(row, col)) {
            colors.parent_cell_bg
        } else if children.contains(&(row, col)) {
            colors.child_cell_bg
        } else {
            colors.cell_bg
        }
    }
    
    html! {
        <div style="overflow: auto; height: 100%; width: 100%;">
            <table style={format!(
                "border-collapse: collapse;
                table-layout: fixed;
                width: calc(30px + {} * {});",
                props.cols, CELL_WIDTH
            )}>
                <thead>
                    <tr style="height: {CELL_HEIGHT};">
                        <th style={format!("
                            width: 30px;
                            background: {};
                            border: 1px solid {};
                            position: sticky;
                            top: 0;
                            z-index: 2;
                            color: {};
                        ", colors.header_bg, colors.border, colors.text)}></th>
                        {(0..props.cols).map(|col| {
                            let letter = col_to_letter(col);
                            html! {
                                <th 
                                    key={format!("col-{}", col)}
                                    style={format!("
                                        width: {CELL_WIDTH};
                                        background: {};
                                        border: 1px solid {};
                                        text-align: center;
                                        font-weight: bold;
                                        position: sticky;
                                        top: 0;
                                        z-index: 1;
                                        overflow: hidden;
                                        text-overflow: ellipsis;
                                        color: {};
                                    ", colors.header_bg, colors.border, colors.text)}
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
                                <td style={format!("
                                    width: 30px;
                                    background: {};
                                    border: 1px solid {};
                                    text-align: center;
                                    font-weight: bold;
                                    position: sticky;
                                    left: 0;
                                    z-index: 1;
                                    color: {};
                                ", colors.header_bg, colors.border, colors.text)}>
                                    {row + 1}
                                </td>
                                {(0..props.cols).map(|col| {
                                    let key = format!("{}-{}", row, col);
                                    let celldata = unsafe { 
                                        backend.get_cell_value(row, col)
                                    };
                                     let val = unsafe{if (*celldata).error == CellError::NoError {
                                            (*celldata).value.to_string()
                                        } else {
                                            "ERR".to_string()
                                        }
                                    };
                                    // let val = unsafe { 
                                    //     backend.get_cell_value(row, col).value.to_string()
                                    // };
                                    
                                    // Get background color based on relationships
                                    let bg_color = get_cell_background_color(
                                        row, 
                                        col, 
                                        *selected_cell, 
                                        &parent_cells, 
                                        &child_cells,
                                        &colors,
                                    );
                                    
                                    let cell_style = format!("
                                        width: {CELL_WIDTH};
                                        height: {CELL_HEIGHT};
                                        border: 1px solid {};
                                        padding: 2px;
                                        background-color: {};
                                        text-align: left;
                                        vertical-align: middle;
                                        overflow: hidden;
                                        text-overflow: ellipsis;
                                        white-space: nowrap;
                                        color: {};
                                    ", colors.border, bg_color, colors.text);

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

#[function_component(FormulaBar)]
pub fn formula_bar(props: &FormulaBarProps) -> Html {
    let frontend = props.frontend.clone();
    let mut frontend = frontend.borrow_mut();
    let selected_cell = props.selected_cell.clone();
    
    // Get theme colors
    let colors = ThemeColors::get(&props.theme);
    
    // Get the formula for the selected cell
    let formula = {
        let backend = frontend.get_backend_mut();
        let (row, col) = *selected_cell;
        backend.formula_strings[row][col].clone()
    };

    html! {
        <div style={format!("padding: 10px; border-bottom: 1px solid {}; background-color: {}", 
                colors.border, colors.background)}>
            <input 
                type="text" 
                placeholder="=SUM(A1:A5)" 
                style={format!("width: 100%; background-color: {}; color: {}; border: 1px solid {}", 
                    colors.background, colors.text, colors.border)} 
                value={formula}
                readonly=true
            />
        </div>
    }
}

#[function_component(CommandBar)]
pub fn command_bar(props: &CommandBarProps) -> Html {
    //let input_value = use_state(|| String::new());
    let input_value = use_state(String::new);
    let status = use_state(|| None::<bool>);
    let input_ref = use_node_ref();

    // Get theme colors
    let colors = ThemeColors::get(&props.theme);

    // Clone fields you need from props before the closure
    let update_trigger = props.update_trigger.clone();
    let frontend = props.frontend.clone();

    let onkeypress = {
        let input_value = input_value.clone();
        let status = status.clone();
        let input_ref = input_ref.clone();
        let update_trigger = update_trigger.clone();
        let frontend = frontend.clone();

        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let current_value = (*input_value).clone();

                let mut frontend = frontend.borrow_mut();
                let result = frontend.run_command(&current_value);

                if result {
                    update_trigger.set(*update_trigger + 1);
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
        <div style={format!("padding: 10px; background-color: {}; display: flex; align-items: center;", 
                colors.command_bar_bg)}>
            { status_display }
            <input 
                ref={input_ref}
                type="text" 
                placeholder="Enter command here..." 
                style={format!("width: 100%; background-color: {}; color: {}; border: 1px solid {}", 
                    colors.cell_bg, colors.text, colors.border)}
                value={(*input_value).clone()}
                {oninput}
                {onkeypress}
            />
        </div>
    }
}

pub fn download_csv(content: String, filename: &str) {
    let array = js_sys::Array::new();
    array.push(&JsValue::from_str(&content));

    let blob = {
        let options = BlobPropertyBag::new();
        options.set_type("text/csv");
        Blob::new_with_str_sequence_and_options(&array, &options)
    }.unwrap();

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
   // let status_message = use_state(|| String::new());
    let status_message = use_state(String::new);
    let file_input_ref = use_node_ref();
    let rows = props.rows;
    let cols = props.cols;
    let theme = props.theme.clone();
    
    // Get theme colors
    let colors = ThemeColors::get(&theme);
    
    // Theme toggle buttons
    let light_theme_onclick = {
        let theme = theme.clone();
        Callback::from(move |_| {
            theme.set(ThemeType::Light);
        })
    };
    
    let dark_theme_onclick = {
        let theme = theme.clone();
        Callback::from(move |_| {
            theme.set(ThemeType::Dark);
        })
    };
    
    //Undo and Redo functionality
    let undo_onclick = {
        let frontend = frontend.clone();
        let status_message = status_message.clone();
        
        Callback::from(move |_: MouseEvent| {
            let mut frontend = frontend.borrow_mut();
            let backend = frontend.get_backend_mut();
            backend.undo_callback();
            status_message.set("Undo successful".to_string());
            
            // Clear message after 3 seconds
            let status_message = status_message.clone();
            gloo::timers::callback::Timeout::new(3000, move || {
                status_message.set(String::new());
            }).forget();
        })
    };
    
    let redo_onclick = {
        let frontend = frontend.clone();
        let status_message = status_message.clone();
        
        Callback::from(move |_: MouseEvent| {
            let mut frontend = frontend.borrow_mut();
            let backend = frontend.get_backend_mut();
            backend.redo_callback();
            status_message.set("Redo successful".to_string());
            
            // Clear message after 3 seconds
            let status_message = status_message.clone();
            gloo::timers::callback::Timeout::new(3000, move || {
                status_message.set(String::new());
            }).forget();
        })
    };
    
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
                        let val = if (*celldata).error == CellError::NoError {
                            (*celldata).value.to_string()
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

    // Button style based on current theme
    let button_style = format!(
        "padding: 5px 10px; margin: 0 2px; border: 1px solid {}; background-color: {}; color: {};",
        colors.border, colors.header_bg, colors.text
    );
    
    let _active_button_style = format!(
        "padding: 5px 10px; margin: 0 2px; border: 1px solid {}; background-color: {}; color: {}; font-weight: bold;",
        colors.border, 
        match *theme {
            ThemeType::Light => "#ffffff",
            ThemeType::Dark => "#111111"
        }, 
        colors.text
    );

    html! {
        <div style={format!("background-color: {}; padding: 0.1px; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid {}; width: 100%;",
                colors.header_bg, colors.border)}>
            <div style="display: flex; gap: 10px;">
            <button onclick={save_onclick}>{ "Save" }</button>
            <button onclick={load_onclick}>{ "Load" }</button>
            <button onclick={undo_onclick}>{ "Undo" }</button>
            <button onclick={redo_onclick}>{ "Redo" }</button>
            </div>
            
            <div style="display: flex; gap: 5px;">
                <button 
                    onclick={light_theme_onclick} 
                    style={format!("{} background-color: {};", 
                        button_style,
                        if matches!(*theme, ThemeType::Light) { "#ffffff" } else { colors.header_bg }
                    )}
                >
                    { "Light" }
                </button>
                <button 
                    onclick={dark_theme_onclick} 
                    style={format!("{} background-color: {};", 
                        button_style,
                        if matches!(*theme, ThemeType::Dark) { "#333333" } else { colors.header_bg }
                    )}
                >
                    { "Dark" }
                </button>
            </div>
            
            <input
                type="file"
                accept=".csv"
                ref={file_input_ref}
                onchange={on_file_change}
                style="display: none;"
            />
            <div style={format!("color: {};", colors.text)}>
                { if !status_message.is_empty() { &*status_message } else { "" } }
            </div>
        </div>
    }
}
