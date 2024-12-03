use std::io::{self, Write};

fn move_cursor_to_top_left() {
    // H: Cursor Position, e.g. <esc>[1;1H]
    write!(io::stdout(), "\x1b[H").expect("Error positioning cursor at top-left before draw");
}

pub fn clear_display() {
    // J: Erase in Display, 2: clear entire screen
    write!(io::stdout(), "\x1b[2J").expect("Error clearing screen");
    move_cursor_to_top_left();
    io::stdout().flush().expect("Error flushing stdout");
}

fn draw_rows(num_rows: u16) {
    for _ in 0..num_rows {
        write!(io::stdout(), "~\r\n").expect("Error drawing tildes (~)");
    }
}

pub fn refresh_screen(num_rows: u16) {
    // Escape sequences begin with escape characters `\x1b` (27) and '['
    // Escape sequence commands take arguments that come before the command itself
    // Arguments are separated by a ';'
    // https://vt100.net/docs/vt100-ug/chapter3.html
    clear_display();
    draw_rows(num_rows);
    move_cursor_to_top_left();

    io::stdout().flush().expect("Error flushing stdout");
}
