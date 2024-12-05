use std::cmp::min;

use crate::{
    input::{EditorKey, Key},
    output::{clear_display, move_cursor_to_top_left},
    terminal::disable_raw_mode,
    utils::get_window_size,
};
use termios::Termios;

#[derive(Clone, Copy)]
pub struct WindowSize {
    pub rows: u16,
    pub columns: u16,
}

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
}
