use std::{
    cmp::min,
    io::{self, Write},
};

use crate::{editor_instance::WindowSize, globals::VERSION, utils::flush_stdout};

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

/// Uses a String as a buffer to store all lines, before calling `write` once
/// Prints a welcome message in the middle of the screen using its row/column count
fn draw_rows(window_size: WindowSize) -> () {
    let mut buffer = String::new();
    let WindowSize {
        rows: num_rows,
        columns: num_columns,
    } = window_size;

    for row in 0..num_rows {
        if row == num_rows / 3 {
            let mut message = format!("Brendan's text editor --- version {VERSION}");
            message = message[..min(num_columns as usize, message.len())].to_string();

            let mut padding = (num_columns - message.len() as u16) / 2;

            if padding > 0 {
                buffer += "~";
                padding -= 1;
            }

            for _ in 0..padding {
                buffer += " ";
            }

            buffer += &message;
        } else {
            buffer += "~";
        }

        buffer += "\x1b[K"; // Erase In Line (2: whole, 1: to left, 0: to right [default])

        if row < num_rows - 1 {
            buffer += "\r\n";
        }
    }

    write!(io::stdout(), "{}", buffer).expect("Error writing to stdout during screen refresh");
    flush_stdout();
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

pub fn refresh_screen(window_size: WindowSize) -> () {
    // Escape sequences begin with escape characters `\x1b` (27) and '['
    // Escape sequence commands take arguments that come before the command itself
    // Arguments are separated by a ';'
    // https://vt100.net/docs/vt100-ug/chapter3.html
    hide_cursor();
    draw_rows(window_size);
    move_cursor_to_top_left();
    show_cursor();
}
