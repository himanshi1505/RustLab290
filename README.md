# COP290 Project: Spreadsheet Program in Rust

This project is a spreadsheet Program which can be run using terminal or using website implemented in Rust.

## Authors
- Himanshi Bhandari
- Eshita Zjigyasu
- Dhruv Pawar

## Project Structure
- `main.rs` : The main entry point of the application.
- `cli.rs`: The entry point of terminal application
- `frontend.rs`: Handles the user interface and interactions.
- `backend.rs`: Manages the backend logic and data processing.
- `parser.rs`: Parses input data and commands.
- `structs.rs`: Defines the structs used in the project.
- `main_gui.rs`: The entry point of website(gui) application
-  `app.rs`: Root Yew component for gui (manages state).
- `README.md`: This file, providing an overview of the project.
- `index.html` : Base HTML template (loads WASM).
- `styles.css` : Visual styling.

## Features
- Basic spreadsheet functionalities such as binary- addition, subtraction, multiplication, division,range- sum, min, max, stdev, avg.
- Cell referencing and formula evaluation.
- Sleep, enable and disable display, scroll_to, a, w, s, d to navigate.
## Website features and Usage
- Tab Bar - undo, redo, save, load, light and dark theme tabs
- Formula Bar - shows formula of the slected cell
- Grid with scroll bars - shows values
- Command Bar with status message 
- Terminal features
- Themes dark and light - click on theme tab buttons
- Coloured selected cell,immediate parent and children
- Cut copy paste of a range - cut(A1:A3), copy(A1:A4), paste(B1), for single cell - cut(A1:A1), copy(A1:A1)
- Autofill  - autofill(A1:A2, A4)
- Sort in ascending order - sorta(A1:A5)
- Sort in descending order - sortd(A1:A5)
- Undo - click on undo tab button, then click on some cell to see the updated grid
- Redo - click on redo tab button, then click on some cell to see the updated grid
- Save: downloads the files - click on save tab button
- Load: loads the file - click on load save button

Note: whenever you do a action in website click somewhere else to update the trigger and see the updated website

## How to Build
1. Compile the project using `cargo run {rows} {cols}`.
2. To run the website `trunk serve --no-default-features --features gui --port 8000`.
3. Port for website - 8080
4. To open rustdoc: cargo doc --open
5. pdflatex should be installed (has been used to make report.pdf from report.tex)

## Usage
- Follow the on-screen instructions to create and manipulate spreadsheets.
- Use standard spreadsheet formulas and cell references.

## Acknowledgements
We would like to thank our professors, TAs and peers at IIT Delhi for their support and guidance.
