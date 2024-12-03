use crate::globals::get_buffer_lock;
use crate::{output::clear_display, terminal::disable_raw_mode};
use std::io::{self, Read, StdinLock, Write};
use std::panic;
use std::{cmp::min, error::Error};
use termion::terminal_size;
use termios::Termios;

#[allow(dead_code)]
pub fn debug_input(key: u8) {
    if key.is_ascii_control() {
        println!("{}\r", key);
    } else {
        println!("{} ('{}')\r", key as char, key)
    }
}

pub fn panic_with_error(e: impl Error, message: &str) -> ! {
    panic!("{message} | {:?}", e)
}

pub fn set_panic_hook(original_termios: Termios) -> () {
    let default_panic_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        clear_display();
        disable_raw_mode(original_termios);

        default_panic_hook(info);
    }));
}

/// Fallback for when `termion.terminal_size()` can not detect terminal dimensions
fn get_cursor_position(stdin_lock: &mut StdinLock) -> (u16, u16) {
    let mut stdout = io::stdout();

    // Cursor Position Report (reply is like `\x1b[24;80R`)
    stdout
        .write(b"\x1b[6n")
        .expect("Failed to write Cursor Position Report command to stdout");

    stdout.flush().expect("Failed to flush stdout");

    let mut buffer = *get_buffer_lock();
    let mut response = Vec::new();

    loop {
        let n = stdin_lock
            .read(&mut buffer)
            .expect("Failed to read from stdin");

        if buffer[0] == b'R' {
            break;
        }

        response.extend_from_slice(&buffer[..n]);
    }

    // Parse the response, e.g., "\x1b[60;118R" (row;column)
    let response_str = String::from_utf8(response)
        .expect("Invalid UTF-8 in terminal response to Cursor Position Report");

    let mut parts = response_str.trim_start_matches("\x1b[").split(';');

    let rows = parts
        .next()
        .expect("Response from Cursor Position Report could not be parsed")
        .parse::<u16>()
        .expect("Failed to parse row into a u16");

    let columns = parts
        .next()
        .expect("Response from Cursor Position Report could not be parsed")
        .parse::<u16>()
        .expect("Failed to parse col into a u16");

    (columns, rows)
}

/// Executes a command to move the cursor to the bottom-right of the screen, then
/// retrieves the new cursor position to determine the terminal dimensions
fn get_window_size_fallback(stdin_lock: &mut StdinLock) -> (u16, u16) {
    let mut stdout = io::stdout();

    // The following 2 commands stop the cursor from going past the screen edge
    let cursor_forward_command = "\x1b[999C".to_string(); // http://vt100.net/docs/vt100-ug/chapter3.html#CUF
    let cursor_down_command = "\x1b[999B".to_string(); // http://vt100.net/docs/vt100-ug/chapter3.html#CUD

    match write!(stdout, "{}{}", cursor_forward_command, cursor_down_command) {
        Ok(_) => {
            stdout.flush().expect("Failed to flush stdout");
            get_cursor_position(stdin_lock)
        }
        Err(e) => {
            panic_with_error(
                e,
                "Failed to write to stdout while executing cursor-move commands",
            );
        }
    }
}

pub fn get_window_size(stdin_lock: &mut StdinLock) -> (u16, u16) {
    match terminal_size() {
        Ok((columns, rows)) => {
            if min(columns, rows) == 0 {
                return get_window_size_fallback(stdin_lock);
            }

            (columns, rows)
        }
        Err(_) => get_window_size_fallback(stdin_lock),
    }
}
