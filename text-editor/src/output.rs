use std::io::{self, Write};

use crate::utils::flush_stdout;

fn move_cursor_to_top_left() -> () {
    // H: Cursor Position, e.g. <esc>[1;1H]
    write!(io::stdout(), "\x1b[H").expect("Error positioning cursor at top-left before draw");
    flush_stdout();
}

pub fn clear_display() -> () {
    // J: Erase in Display, 2: clear entire screen
    write!(io::stdout(), "\x1b[2J").expect("Error clearing screen");
    move_cursor_to_top_left();
    flush_stdout();
}

fn draw_rows_to_buffer(num_rows: u16, buffer: &mut String) -> () {
    for row in 0..num_rows {
        *buffer += "~";

        if row < num_rows - 1 {
            *buffer += "\r\n";
        }
    }
}

pub fn refresh_screen(num_rows: u16) -> () {
    // Escape sequences begin with escape characters `\x1b` (27) and '['
    // Escape sequence commands take arguments that come before the command itself
    // Arguments are separated by a ';'
    // https://vt100.net/docs/vt100-ug/chapter3.html
    clear_display();

    let mut buffer = String::new();
    draw_rows_to_buffer(num_rows, &mut buffer);

    move_cursor_to_top_left();

    write!(io::stdout(), "{}", buffer).expect("Error writing to stdout during screen refresh");
    flush_stdout();
}
