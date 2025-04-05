static do_print : bool = true;
static rows : i32 = 0;
static cols : i32 = 0;



fn number_to_column_header(mut number: i32) -> String {
    number = number + 1;
    let mut buffer = String::new();

    while number > 0 {
        let rem = (number - 1) % 26;
        buffer.insert(0, (b'A' + rem as u8) as char);
        number = (number - 1) / 26;
    }
    buffer
}

fn print_board() {
    if do_print != false {
        
    }
}

fn init_frontend(row:i32, col: i32) {
    
}

fn main() {

}
