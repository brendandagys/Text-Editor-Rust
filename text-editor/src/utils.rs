use crate::editor_instance::Line;
use crate::globals::get_buffer_lock;
use crate::output::{move_cursor_to_top_left, AnsiEscapeCode};
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

pub fn set_panic_hook(original_termios: Termios) {
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
    write!(io::stdout(), "{}", AnsiEscapeCode::CursorReport.as_str())
        .expect("Failed to write Cursor Position Report command to stdout");

    flush_stdout();

    let mut buffer = *get_buffer_lock();
    let mut response = Vec::new();
    let mut stdin = io::stdin().lock();

    loop {
        let n = stdin.read(&mut buffer).expect("Failed to read from stdin");

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

    WindowSize {
        rows: rows - 2, // Account for status bar and status message bar
        columns: columns - 2,
    }
}

/// Executes a command to move the cursor to the bottom-right of the screen, then
/// retrieves the new cursor position to determine the terminal dimensions
fn get_window_size_fallback() -> WindowSize {
    // The following 2 commands stop the cursor from going past the screen edge
    let cursor_forward_command = "\x1b[999C".to_string(); // http://vt100.net/docs/vt100-ug/chapter3.html#CUF
    let cursor_down_command = "\x1b[999B".to_string(); // http://vt100.net/docs/vt100-ug/chapter3.html#CUD

    match write!(
        io::stdout(),
        "{}{}",
        cursor_forward_command,
        cursor_down_command
    ) {
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

pub fn watch_for_window_size_change(window_size_clone: Arc<RwLock<WindowSize>>) {
    thread::spawn(move || {
        let mut signals = Signals::new([SIGWINCH]).expect("Failed to register SIGWINCH signal");

        for _ in signals.forever() {
            *window_size_clone
                .write()
                .expect("Failed to obtain window size write lock") = get_window_size();
        }
    });
}

pub fn flush_stdout() {
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

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::editor_instance::Line;

    mod test_ctrl_key {
        use super::*;

        #[test]
        fn test_ctrl_key_uppercase_letters() {
            for i in 0..26 {
                let ch = (b'A' + i) as char;
                let expected = i + 1; // Ctrl+A -> 1, Ctrl+B -> 2, ..., Ctrl+Z -> 26
                assert_eq!(ctrl_key(ch), expected);
            }
        }

        #[test]
        fn test_ctrl_key_lowercase_letters() {
            for i in 0..26 {
                let ch = (b'a' + i) as char;
                let expected = i + 1; // Ctrl+a -> 1, Ctrl+b -> 2, ..., Ctrl+z -> 26
                assert_eq!(ctrl_key(ch), expected);
            }
        }

        #[test]
        fn test_ctrl_key_with_non_alphabetic_characters() {
            // Test with space and other characters
            let test_cases = vec![
                (' ', 0),         // Ctrl+Space should map to 0
                ('0', 48 & 0x1f), // Ctrl+0 should be 48 & 0x1f
                ('1', 49 & 0x1f), // Ctrl+1 should be 49 & 0x1f
                ('!', 33 & 0x1f), // Ctrl+! should be 33 & 0x1f
                ('@', 64 & 0x1f), // Ctrl+@ should be 64 & 0x1f
                ('#', 35 & 0x1f), // Ctrl+# should be 35 & 0x1f
            ];

            for (ch, expected) in test_cases {
                assert_eq!(ctrl_key(ch), expected);
            }
        }
    }

    mod test_lines_to_string {
        use super::*;

        #[test]
        fn test_lines_to_string_empty() {
            let lines: Vec<Line> = Vec::new();
            assert_eq!(lines_to_string(&lines), "");
        }

        #[test]
        fn test_lines_to_string_single_line() {
            let lines = vec![Line {
                text: String::from("Line 1"),
                render: String::from("Line 1"),
                highlight: vec![],
                index: 0,
                has_open_multiline_comment: false,
            }];

            assert_eq!(lines_to_string(&lines), "Line 1\n");
        }

        #[test]
        fn test_lines_to_string_multiple_lines() {
            let lines = vec![
                Line {
                    text: String::from("Line 1"),
                    render: String::from("Line 1"),
                    highlight: vec![],
                    index: 0,
                    has_open_multiline_comment: false,
                },
                Line {
                    text: String::from("Line 2"),
                    render: String::from("Line 2"),
                    highlight: vec![],
                    index: 1,
                    has_open_multiline_comment: false,
                },
            ];

            assert_eq!(lines_to_string(&lines), "Line 1\nLine 2\n");
        }

        #[test]
        fn test_lines_to_string_special_characters() {
            let lines = vec![
                Line {
                    text: String::from("Line with spaces    "),
                    render: String::from("Line with spaces    "),
                    highlight: vec![],
                    index: 0,
                    has_open_multiline_comment: false,
                },
                Line {
                    text: String::from("Line\twith\ttabs"),
                    render: String::from("Line\twith\ttabs"),
                    highlight: vec![],
                    index: 1,
                    has_open_multiline_comment: false,
                },
                Line {
                    text: String::from("Line\nwith\nnewlines"),
                    render: String::from("Line\nwith\nnewlines"),
                    highlight: vec![],
                    index: 2,
                    has_open_multiline_comment: false,
                },
            ];

            assert_eq!(
                lines_to_string(&lines),
                "Line with spaces    \nLine\twith\ttabs\nLine\nwith\nnewlines\n"
            );
        }
    }

    mod test_get_file_name_from_path {
        use super::*;

        #[test]
        fn test_get_file_name_from_path_basic() {
            let file_path = "/home/user/documents/file.txt";
            assert_eq!(get_file_name_from_path(file_path), "file.txt");
        }

        #[test]
        fn test_get_file_name_from_path_root_directory() {
            let file_path = "/file.txt";
            assert_eq!(get_file_name_from_path(file_path), "file.txt");
        }

        #[test]
        fn test_get_file_name_from_path_no_file_name() {
            let file_path = "/home/user/documents/";
            assert_eq!(get_file_name_from_path(file_path), "");
        }

        #[test]
        fn test_get_file_name_from_path_empty() {
            assert_eq!(get_file_name_from_path(""), "");
        }
    }
}
