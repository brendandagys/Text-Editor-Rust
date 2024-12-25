use crate::editor_instance::Line;
use crate::globals::get_buffer_lock;
use crate::output::move_cursor_to_top_left;
use crate::WindowSize;
use crate::{output::clear_display, terminal::disable_raw_mode};
use signal_hook::consts::SIGWINCH;
use signal_hook::iterator::Signals;
use std::cmp::min;
use std::io::{self, Read, Write};
use std::sync::{Arc, RwLock};
use std::{panic, thread};
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

pub fn ctrl_key(k: char) -> u8 {
    (k as u8) & 0x1f // Ctrl key strips bits 5 and 6 from 7-bit ASCII
}

pub fn set_panic_hook(original_termios: Termios) -> () {
    let default_panic_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        clear_display();
        move_cursor_to_top_left();
        disable_raw_mode(original_termios);

        default_panic_hook(info);
    }));
}

/// Fallback for when `termion.terminal_size()` can not detect terminal dimensions
fn get_cursor_position() -> WindowSize {
    let mut stdout = io::stdout();

    // Cursor Position Report (reply is like `\x1b[24;80R`)
    stdout
        .write(b"\x1b[6n")
        .expect("Failed to write Cursor Position Report command to stdout");

    flush_stdout();

    let mut buffer = *get_buffer_lock();
    let mut response = Vec::new();

    loop {
        let n = io::stdin()
            .lock()
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
        .expect("Failed to parse response from Cursor Position Report")
        .parse::<u32>()
        .expect("Failed to parse row to u16");

    let columns = parts
        .next()
        .expect("Failed to parse response from Cursor Position Report")
        .parse::<u16>()
        .expect("Failed to parse col to u16");

    WindowSize { rows, columns }
}

/// Executes a command to move the cursor to the bottom-right of the screen, then
/// retrieves the new cursor position to determine the terminal dimensions
fn get_window_size_fallback() -> WindowSize {
    let mut stdout = io::stdout();

    // The following 2 commands stop the cursor from going past the screen edge
    let cursor_forward_command = "\x1b[999C".to_string(); // http://vt100.net/docs/vt100-ug/chapter3.html#CUF
    let cursor_down_command = "\x1b[999B".to_string(); // http://vt100.net/docs/vt100-ug/chapter3.html#CUD

    match write!(stdout, "{}{}", cursor_forward_command, cursor_down_command) {
        Ok(_) => {
            flush_stdout();
            get_cursor_position()
        }
        Err(e) => {
            panic!(
                "Failed to write to stdout while executing cursor-move commands: {:?}",
                e
            );
        }
    }
}

pub fn get_window_size() -> WindowSize {
    match terminal_size() {
        Ok((columns, rows)) => {
            if min(rows, columns) == 0 {
                return get_window_size_fallback();
            }

            WindowSize {
                rows: (rows - 2).into(), // Subtract 2 for status bar and status message
                columns,
            }
        }
        Err(_) => get_window_size_fallback(),
    }
}

pub fn watch_for_window_size_change(window_size_clone: Arc<RwLock<WindowSize>>) -> () {
    thread::spawn(move || {
        let mut signals = Signals::new(&[SIGWINCH]).expect("Failed to register SIGWINCH signal");

        for _ in signals.forever() {
            *window_size_clone
                .write()
                .expect("Failed to obtain window size write lock") = get_window_size();
        }
    });
}

pub fn flush_stdout() -> () {
    io::stdout().flush().expect("Failed to flush stdout");
}

pub fn lines_to_string(lines: &Vec<Line>) -> String {
    let mut string = String::new();

    for line in lines {
        string.push_str(&line.text);
        string.push('\n');
    }

    string
}

pub fn get_file_name_from_path(file_path: &str) -> String {
    file_path
        .split('/')
        .last()
        .expect("Failed to parse file from provided file path")
        .into()
}
