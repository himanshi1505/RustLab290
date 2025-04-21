// Removed incorrect import
use yew::prelude::*;
use std::collections::VecDeque;
use std::cell::UnsafeCell;
use std::rc::Rc;
use std::cell::RefCell;

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
            <TabBar />
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
    
    // Fixed dimensions for cells
    const CELL_WIDTH: &str = "80px";
    const CELL_HEIGHT: &str = "24px";
    // Calculate total table size
    let table_style = format!("
        width: calc({} * {});
        height: calc({} * {});
    ", 
        CELL_WIDTH, props.cols,
        CELL_HEIGHT, props.rows
    );
    
    html! {
        <div style="overflow: auto; height: 100%; width: 100%;">
            <table 
                style={table_style}
                class="spreadsheet-grid"
            >
                <tbody>
                    { for (0..props.rows).map(|row| html! {
                        <tr key={row.to_string()} style="height: {CELL_HEIGHT};">
                            { for (0..props.cols).map(|col| {
                                let key = format!("{}-{}", row, col);
                                let mut val="0".to_string();
                                unsafe {
                                    let celldata =  backend.get_cell_value(row, col);
                                    if(celldata.error == CellError::NoError){
                                        val=celldata.value.to_string();
                                    } 
                                    else{
                                        val="ERR".to_string();
                                    } 
                                }
                                let is_selected = *selected_cell == (row, col);
                                
                                let cell_style = format!("
                                    width: {CELL_WIDTH};
                                    height: {CELL_HEIGHT};
                                    border: 1px solid #ddd;
                                    padding: 2px;
                                    text-align: left;
                                    vertical-align: middle;
                                    overflow: hidden;
                                    text-overflow: ellipsis;
                                    white-space: nowrap;
                                    background-color: {};
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
                                        class="spreadsheet-cell"
                                    >
                                        {val}
                                    </td>
                                }
                            })}
                        </tr>
                    })}
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
// pub fn tab_bar(props: &BackendProps) -> Html {
//     let undo_onclick = {
//         let backend = props.backend.clone();
//         Callback::from(move |_| {
//             let mut new_backend = (*backend).clone();
//             if new_backend.undo_callback().is_ok() {
//                 backend.set(new_backend);
//             }
//         })
//     };

//     let redo_onclick = {
//         let backend = props.backend.clone();
//         Callback::from(move |_| {
//             let mut new_backend = (*backend).clone();
//             if new_backend.redo().is_ok() {
//                 backend.set(new_backend);
//             }
//         })
//     };

//     html! {
//         <div style="background-color: #eee; padding: 10px;">
//             <button onclick={undo_onclick}>{ "Undo" }</button>
//             <button onclick={redo_onclick}>{ "Redo" }</button>
//         </div>
//     }
// }
#[function_component(TabBar)]
pub fn tab_bar() -> Html {
    html! {
        <div style="background-color: #eee; padding: 10px;">
            <button>{"Undo"}</button>
            <button>{"Redo"}</button>
            <button>{"Save"}</button>
        </div>
    }
}