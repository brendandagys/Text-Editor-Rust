use std::cmp::min;

use crate::{
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

    pub fn process_key(&mut self, key: u8) -> () {
        match key {
            b'h' => self.move_cursor(CursorMovement::Left),
            b'j' => self.move_cursor(CursorMovement::Down),
            b'k' => self.move_cursor(CursorMovement::Up),
            b'l' => self.move_cursor(CursorMovement::Right),
            b'p' => panic!("Manual panic!"),
            key if key == ctrl_key('q') => {
                clear_display();
                move_cursor_to_top_left();
                disable_raw_mode(self.original_termios);

                std::process::exit(0);
            }
            _ => {}
        }
    }
}
