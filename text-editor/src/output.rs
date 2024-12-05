use std::io::{self, Write};

use crate::utils::flush_stdout;

pub fn move_cursor_to_top_left() -> () {
    // H: Cursor Position, e.g. <esc>[1;1H]
    write!(io::stdout(), "\x1b[H").expect("Error positioning cursor at top-left before draw");
    flush_stdout();
}

pub fn clear_display() -> () {
    // J: Erase in Display, 2: clear entire screen
    write!(io::stdout(), "\x1b[2J").expect("Error clearing screen");
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

fn hide_cursor() -> () {
    // l: Reset mode
    write!(io::stdout(), "\x1b[?25l").expect("Error hiding cursor");
    flush_stdout();
}

fn show_cursor() -> () {
    // h: Set mode
    write!(io::stdout(), "\x1b[?25h").expect("Error hiding cursor");
    flush_stdout();
}

pub fn refresh_screen(num_rows: u16) -> () {
    // Escape sequences begin with escape characters `\x1b` (27) and '['
    // Escape sequence commands take arguments that come before the command itself
    // Arguments are separated by a ';'
    // https://vt100.net/docs/vt100-ug/chapter3.html
    hide_cursor();
    clear_display();
    move_cursor_to_top_left();

    let mut buffer = String::new();
    draw_rows_to_buffer(num_rows, &mut buffer);

    move_cursor_to_top_left();
    show_cursor();

    write!(io::stdout(), "{}", buffer).expect("Error writing to stdout during screen refresh");
    flush_stdout();
}
