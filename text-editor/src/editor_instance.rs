use std::{
    cmp::min,
    io::{self, Write},
};

use crate::{
    globals::VERSION,
    input::{EditorKey, Key},
    output::{clear_display, move_cursor_to_top_left},
    terminal::disable_raw_mode,
    utils::{flush_stdout, get_window_size},
    WindowSize,
};
use termios::Termios;

#[derive(Clone, Copy)]
pub struct CursorPosition {
    pub x: u16,
    pub y: u16,
}

pub enum CursorMovement {
    Up,
    Down,
    Left,
    Right,
}

pub struct EditorInstance {
    original_termios: Termios,
    pub window_size: WindowSize,
    pub cursor_position: CursorPosition,
}

fn ctrl_key(k: char) -> u8 {
    (k as u8) & 0x1f // Ctrl key strips bits 5 and 6 from 7-bit ASCII
}

impl EditorInstance {
    pub fn new(original_termios: Termios) -> Self {
        EditorInstance {
            original_termios,
            window_size: get_window_size(),
            cursor_position: CursorPosition { x: 0, y: 0 },
        }
    }

    pub fn move_cursor(&mut self, direction: CursorMovement) -> () {
        match direction {
            CursorMovement::Left => {
                self.cursor_position.x = if self.cursor_position.x > 0 {
                    self.cursor_position.x - 1
                } else {
                    0
                }
            }
            CursorMovement::Down => {
                self.cursor_position.y = min(self.cursor_position.y + 1, self.window_size.rows)
            }
            CursorMovement::Up => {
                self.cursor_position.y = if self.cursor_position.y > 0 {
                    self.cursor_position.y - 1
                } else {
                    0
                }
            }
            CursorMovement::Right => {
                self.cursor_position.x = min(self.cursor_position.x + 1, self.window_size.columns)
            }
        }
    }

    pub fn process_key(&mut self, key: Key) -> () {
        match key {
            Key::U8(b'h') | Key::Custom(EditorKey::ArrowLeft) => {
                self.move_cursor(CursorMovement::Left)
            }
            Key::U8(b'j') | Key::Custom(EditorKey::ArrowDown) => {
                self.move_cursor(CursorMovement::Down)
            }
            Key::U8(b'k') | Key::Custom(EditorKey::ArrowUp) => self.move_cursor(CursorMovement::Up),
            Key::U8(b'l') | Key::Custom(EditorKey::ArrowRight) => {
                self.move_cursor(CursorMovement::Right)
            }

            Key::Custom(EditorKey::PageUp) => {
                for _ in 0..self.window_size.rows {
                    self.move_cursor(CursorMovement::Up);
                }
            }
            Key::Custom(EditorKey::PageDown) => {
                for _ in 0..self.window_size.rows {
                    self.move_cursor(CursorMovement::Down);
                }
            }

            Key::Custom(EditorKey::Home) => self.cursor_position.x = 0,
            Key::Custom(EditorKey::End) => self.cursor_position.x = self.window_size.columns - 1,

            Key::U8(b'p') => panic!("Manual panic!"),
            Key::U8(key) if key == ctrl_key('q') => {
                clear_display();
                move_cursor_to_top_left();
                disable_raw_mode(self.original_termios);

                std::process::exit(0);
            }
            _ => {}
        }
    }

    pub fn move_cursor_to_position(&self) -> () {
        // H: Cursor Position, e.g. <esc>[1;1H]
        write!(
            io::stdout(),
            "\x1b[{};{}H",
            self.cursor_position.y + 1,
            self.cursor_position.x + 1
        )
        .expect("Error positioning cursor");

        flush_stdout();
    }

    /// Uses a String as a buffer to store all lines, before calling `write` once
    /// Prints a welcome message in the middle of the screen using its row/column count
    pub fn draw_rows(&self, window_size: WindowSize) -> () {
        let mut buffer = String::new();

        for row in 0..window_size.rows {
            if row == window_size.rows / 3 {
                let message = format!("Brendan's text editor --- version {VERSION}");
                let message =
                    message[..min(window_size.columns as usize, message.len())].to_string();

                let mut padding = (window_size.columns - message.len() as u16) / 2;

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

            if row < window_size.rows - 1 {
                buffer += "\r\n";
            }
        }

        write!(io::stdout(), "{}", buffer).expect("Error writing to stdout during screen refresh");
        flush_stdout();
    }
}
