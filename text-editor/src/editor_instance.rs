use crate::{
    globals::{TAB_SIZE, VERSION},
    input::{EditorKey, Key},
    output::{clear_display, move_cursor_to_top_left},
    terminal::disable_raw_mode,
    utils::{flush_stdout, get_window_size},
    WindowSize,
};
use std::{
    cmp::min,
    fs::File,
    io::{self, BufRead, BufReader, Write},
};
use termios::Termios;

#[derive(Clone, Copy)]
pub struct CursorPosition {
    pub x: u16,
    pub y: u32,

    render_x: u16, // Includes extra space from tabs
}

pub enum CursorMovement {
    Up,
    Down,
    Left,
    Right,
}

struct Line {
    text: String,
    render: String,
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
            cursor_position: CursorPosition {
                x: 0,
                y: 0,
                render_x: 0,
            },
            lines: vec![],
            line_scrolled_to: 0,
            column_scrolled_to: 0,
        }
    }

    pub fn open(&mut self, file_path: &str) {
        let reader =
            BufReader::new(File::open(file_path).expect("Failed to open file at specified path"));

        for line in reader.lines() {
            let text = line.expect(&format!("Failed to read line from file: {}", file_path));

            let mut render = String::new();
            let mut render_index = 0;

            for char in text.chars() {
                if char == '\t' {
                    render.push(char);
                    render_index += 1;

                    while render_index % TAB_SIZE != 0 {
                        render.push(' ');
                        render_index += 1;
                    }
                } else {
                    render.push(char);
                    render_index += 1;
                }
            }

            self.lines.push(Line { text, render });
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
                    self.cursor_position.x = self.lines[self.cursor_position.y as usize]
                        .text
                        .chars()
                        .count()
                        .try_into()
                        .expect("Unable to convert line length usize into a u16");
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
                    if (self.cursor_position.x as usize) < current_line.text.chars().count() {
                        self.cursor_position.x += 1;
                    } else if self.cursor_position.x as usize == current_line.text.chars().count() {
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
            Some(line) => line.text.chars().count(),
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
                self.cursor_position.y = self.line_scrolled_to;

                for _ in 0..self.window_size.rows {
                    self.move_cursor(CursorMovement::Up);
                }
            }
            Key::Custom(EditorKey::PageDown) => {
                self.cursor_position.y = self.line_scrolled_to + self.window_size.rows - 1;

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
            self.cursor_position.y - self.line_scrolled_to + 1,
            self.cursor_position.render_x - self.column_scrolled_to + 1
        )
        .expect("Error positioning cursor");

        flush_stdout();
    }

    fn cx_to_render_x(&self, cursor_x_position: u16) -> u16 {
        (0..cursor_x_position).fold(0, |acc, x| {
            let char = self.lines[self.line_scrolled_to as usize]
                .text
                .chars()
                .nth(x as usize);

            match char {
                Some(char) if char == '\t' => acc + TAB_SIZE as u16 - (acc % TAB_SIZE as u16),
                _ => acc + 1,
            }
        })
    }

    pub fn scroll(&mut self) -> () {
        self.cursor_position.render_x = if (self.cursor_position.y as usize) < self.lines.len() {
            self.cx_to_render_x(self.cursor_position.x)
        } else {
            0
        };

        if self.cursor_position.y < self.line_scrolled_to {
            self.line_scrolled_to = self.cursor_position.y;
        }

        if self.cursor_position.y >= self.line_scrolled_to + self.window_size.rows {
            self.line_scrolled_to = self.cursor_position.y - self.window_size.rows + 1;
        }

        if self.cursor_position.render_x < self.column_scrolled_to {
            self.column_scrolled_to = self.cursor_position.render_x;
        }

        if self.cursor_position.render_x >= self.column_scrolled_to + self.window_size.columns {
            self.column_scrolled_to = self.cursor_position.render_x - self.window_size.columns + 1;
        }
    }

    /// Uses a String as a buffer to store all lines, before calling `write` once
    /// Prints a welcome message in the middle of the screen using its row/column count
    pub fn draw_rows(&self) -> () {
        let mut buffer = String::new();

        for row in 0..self.window_size.rows {
            let scrolled_to_row = row + self.line_scrolled_to;

            if scrolled_to_row as usize >= self.lines.len() {
                if self.lines.len() == 0 && row == self.window_size.rows / 3 {
                    let mut message = format!("Brendan's text editor --- version {VERSION}");
                    message.truncate(self.window_size.columns as usize);

                    let message_length: u16 = message.chars().count().try_into().expect(
                        "Could not convert welcome message length into a u16 during screen refresh",
                    );

                    let mut padding = (self.window_size.columns - message_length) / 2;

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
                let line_content = &self.lines[scrolled_to_row as usize].render;

                let start = self.column_scrolled_to as usize;
                let end = start + self.window_size.columns as usize;

                let num_characters = line_content.chars().count();

                if num_characters > end {
                    buffer += &line_content[start..end];
                } else if num_characters > start {
                    buffer += &line_content[start..];
                }
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
