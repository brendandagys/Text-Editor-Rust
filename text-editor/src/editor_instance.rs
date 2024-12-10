use crate::{
    globals::VERSION,
    input::{EditorKey, Key},
    output::{clear_display, move_cursor_to_top_left},
    terminal::disable_raw_mode,
    utils::{flush_stdout, get_window_size},
    WindowSize,
};
use std::{
    cmp::{max, min},
    fs::File,
    io::{self, BufRead, BufReader, Write},
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

struct Line {
    text: String,
}

pub struct EditorInstance {
    original_termios: Termios,
    pub window_size: WindowSize,
    pub cursor_position: CursorPosition,
    lines: Vec<Line>,
    line_scrolled_to: u32,
    column_scrolled_to: u16,
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
            lines: vec![],
            line_scrolled_to: 0,
            column_scrolled_to: 0,
        }
    }

    pub fn open(&mut self, file_path: &str) {
        let reader =
            BufReader::new(File::open(file_path).expect("Failed to open file at specified path"));

        for line in reader.lines() {
            self.lines.push(Line {
                text: line.expect(&format!("Failed to read line from file: {}", file_path)),
            });
        }
    }

    pub fn move_cursor(&mut self, direction: CursorMovement) -> () {
        let current_line = if (self.cursor_position.y as usize) < self.lines.len() {
            Some(&self.lines[self.cursor_position.y as usize])
        } else {
            None
        };

        match direction {
            CursorMovement::Left => {
                if self.cursor_position.x > 0 {
                    self.cursor_position.x -= 1;
                } else if self.cursor_position.y > 0 {
                    self.cursor_position.y -= 1;
                    self.cursor_position.x =
                        self.lines[self.cursor_position.y as usize].text.len() as u16;
                }
            }
            CursorMovement::Down => {
                if (self.cursor_position.y as usize) < self.lines.len() {
                    self.cursor_position.y += 1;
                }
            }
            CursorMovement::Up => {
                if self.cursor_position.y > 0 {
                    self.cursor_position.y -= 1;
                }
            }
            CursorMovement::Right => {
                if let Some(current_line) = current_line {
                    if (self.cursor_position.x as usize) < current_line.text.len() {
                        self.cursor_position.x += 1;
                    } else if self.cursor_position.x as usize == current_line.text.len() {
                        self.cursor_position.y += 1;
                        self.cursor_position.x = 0;
                    }
                }
            }
        }

        let current_line_after_cursor_move = if (self.cursor_position.y as usize) < self.lines.len()
        {
            Some(&self.lines[self.cursor_position.y as usize])
        } else {
            None
        };

        let line_length = match current_line_after_cursor_move {
            Some(line) => line.text.len(),
            None => 0,
        };

        self.cursor_position.x = min(
            self.cursor_position.x,
            line_length
                .try_into()
                .expect("Unable to convert line length usize into a u16"),
        );
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
            self.cursor_position.y as u32 - self.line_scrolled_to + 1,
            self.cursor_position.x - self.column_scrolled_to + 1
        )
        .expect("Error positioning cursor");

        flush_stdout();
    }

    pub fn scroll(&mut self) -> () {
        if (self.cursor_position.y as u32) < self.line_scrolled_to {
            self.line_scrolled_to = self.cursor_position.y as u32;
        }

        if self.cursor_position.y as u32 >= self.line_scrolled_to + self.window_size.rows as u32 {
            self.line_scrolled_to =
                self.cursor_position.y as u32 - self.window_size.rows as u32 + 1;
        }

        if self.cursor_position.x < self.column_scrolled_to {
            self.column_scrolled_to = self.cursor_position.x;
        }

        if self.cursor_position.x >= self.column_scrolled_to + self.window_size.columns {
            self.column_scrolled_to = self.cursor_position.x - self.window_size.columns + 1;
        }
    }

    /// Uses a String as a buffer to store all lines, before calling `write` once
    /// Prints a welcome message in the middle of the screen using its row/column count
    pub fn draw_rows(&self) -> () {
        let mut buffer = String::new();

        for row in 0..self.window_size.rows {
            let scrolled_to_row = row as u32 + self.line_scrolled_to;

            if scrolled_to_row as usize >= self.lines.len() {
                if self.lines.len() == 0 && row == self.window_size.rows / 3 {
                    let message = format!("Brendan's text editor --- version {VERSION}");
                    let message = &message[..min(self.window_size.columns as usize, message.len())];

                    let mut padding = (self.window_size.columns - message.len() as u16) / 2;

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
            } else {
                let text = &self.lines[scrolled_to_row as usize].text;

                let line_length = min(
                    max(text.len() - self.column_scrolled_to as usize, 0),
                    self.window_size.columns as usize,
                );

                buffer += &text[(self.column_scrolled_to as usize)
                    ..((self.column_scrolled_to as usize) + line_length)];
            }

            buffer += "\x1b[K"; // Erase In Line (2: whole, 1: to left, 0: to right [default])

            if row < self.window_size.rows - 1 {
                buffer += "\r\n";
            }
        }

        write!(io::stdout(), "{}", buffer).expect("Error writing to stdout during screen refresh");
        flush_stdout();
    }
}
