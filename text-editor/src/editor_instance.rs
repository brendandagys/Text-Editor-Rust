use crate::{
    globals::{QUIT_CONFIRMATION_COUNT, TAB_SIZE, VERSION},
    input::{EditorKey, Key},
    output::{clear_display, move_cursor_to_top_left, prompt_user},
    terminal::disable_raw_mode,
    utils::{ctrl_key, flush_stdout, get_file_name_from_path, get_window_size, lines_to_string},
    WindowSize,
};
use std::{
    cmp::min,
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    os::unix::fs::OpenOptionsExt,
    time::Instant,
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

pub struct Line {
    pub text: String,
    render: String,
}

struct StatusMessage {
    message: String,
    time_set: Instant,
}

enum SearchDirection {
    Forward,
    Backward,
}

pub struct EditorInstance {
    original_termios: Termios,
    pub window_size: WindowSize,
    pub cursor_position: CursorPosition,
    lines: Vec<Line>,
    line_scrolled_to: u32,
    column_scrolled_to: u16,
    file_path: Option<String>,
    file_name: Option<String>,
    status_message: Option<StatusMessage>,
    edited: bool,
    quit_confirmations: u8,
    previous_search_match_line_index: Option<usize>,
    search_direction: SearchDirection,
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
            file_path: None,
            file_name: None,
            status_message: None,
            edited: false,
            quit_confirmations: 0,
            previous_search_match_line_index: None,
            search_direction: SearchDirection::Forward,
        }
    }

    fn get_render_text_from_text(text: &str) -> String {
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

        render
    }

    pub fn open(&mut self, file_path: &str) {
        let reader =
            BufReader::new(File::open(file_path).expect("Failed to open file at specified path"));

        for line in reader.lines() {
            let text = line.expect(&format!("Failed to read line from file: {}", file_path));
            let render = EditorInstance::get_render_text_from_text(&text);

            self.lines.push(Line { text, render });
        }

        self.file_path = Some(file_path.to_string());
        self.file_name = Some(get_file_name_from_path(file_path));
    }

    fn save(&mut self) -> () {
        if self.file_path.is_none() {
            match prompt_user::<fn(&mut EditorInstance, &str, Key)>(self, "Save as: ", None) {
                Some(file_path) => {
                    self.file_name = Some(get_file_name_from_path(&file_path));
                    self.file_path = Some(file_path);
                }
                None => {
                    self.set_status_message("Save aborted");
                    return;
                }
            }
        }

        if let Some(file_path) = &self.file_path {
            let mut file = match OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .mode(0o644) // Owner R/W; others R
                .open(file_path)
            {
                Ok(file) => file,
                Err(e) => {
                    self.set_status_message(&format!(
                        "Failed to open {file_path} for saving: {:?}",
                        e
                    ));
                    return;
                }
            };

            let content = lines_to_string(&self.lines);

            let content_length: u64 = match content.len().try_into() {
                Ok(length) => length,
                Err(e) => {
                    self.set_status_message(&format!(
                        "Failed to convert content length from usize to u64: {:?}",
                        e
                    ));
                    return;
                }
            };

            if let Err(e) = file.set_len(content_length) {
                self.set_status_message(&format!(
                    "Failed to truncate {file_path} to new length: {:?}",
                    e
                ));
                return;
            }

            match file.write_all(content.as_bytes()) {
                Ok(_) => {
                    self.set_status_message(&format!("{} bytes written to disk", content.len()));
                    self.edited = false;
                }
                Err(e) => {
                    self.set_status_message(&format!("Failed to write to {file_path}: {:?}", e))
                }
            }
        }
    }

    pub fn process_key(&mut self, key: Key) -> () {
        match key {
            Key::U8(b'\r') => self.insert_line(), // Enter

            Key::Custom(EditorKey::ArrowLeft) => self.move_cursor(CursorMovement::Left),
            Key::Custom(EditorKey::ArrowDown) => self.move_cursor(CursorMovement::Down),
            Key::Custom(EditorKey::ArrowUp) => self.move_cursor(CursorMovement::Up),
            Key::Custom(EditorKey::ArrowRight) => self.move_cursor(CursorMovement::Right),

            Key::Custom(EditorKey::Home) => self.cursor_position.x = 0,
            Key::Custom(EditorKey::End) => {
                if (self.cursor_position.y as usize) < self.lines.len() {
                    self.cursor_position.x = self.lines[self.cursor_position.y as usize]
                        .text
                        .chars()
                        .count()
                        .try_into()
                        .expect("Failed to convert current line length into x cursor position");
                }
            }

            // Backspace: historically sent `8`; now sends `127`
            // Delete: historically sent `127`; now sends `<esc>[3~`
            Key::Custom(EditorKey::Backspace) | Key::Custom(EditorKey::Delete) => {
                if key == Key::Custom(EditorKey::Delete) {
                    self.move_cursor(CursorMovement::Right);
                }

                self.delete_character();
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

            // Ctrl-L typically refreshes terminal screen; we do so after each key-press
            // We ignore escapes because there are many key escape sequences we don't handle (e.g. F1-F12)
            Key::U8(key) if key == ctrl_key('l') || key == b'\x1b' => {}

            Key::U8(key) if key == ctrl_key('s') => {
                self.save();
            }

            Key::U8(key) if key == ctrl_key('f') => self.prompt_and_find_text(),

            Key::U8(key) if key == ctrl_key('q') => {
                if self.edited && self.quit_confirmations < QUIT_CONFIRMATION_COUNT {
                    let confirmations_remaining = QUIT_CONFIRMATION_COUNT - self.quit_confirmations;

                    self.set_status_message(&format!(
                        "WARNING: File has unsaved changes! Press Ctrl-Q {} more time{} to quit.",
                        confirmations_remaining,
                        if confirmations_remaining == 1 {
                            ""
                        } else {
                            "s"
                        }
                    ));

                    self.quit_confirmations += 1;

                    return;
                }

                clear_display();
                move_cursor_to_top_left();
                disable_raw_mode(self.original_termios);

                std::process::exit(0);
            }
            _ => {
                if let Key::U8(key) = key {
                    self.insert_character(key as char);
                }
            }
        }

        self.quit_confirmations = 0;
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

    fn cursor_x_to_render_x(&self, cursor_x_position: u16) -> u16 {
        (0..cursor_x_position).fold(0, |acc, x| {
            let char = self.lines[self.cursor_position.y as usize]
                .text
                .chars()
                .nth(x as usize);

            match char {
                Some(char) if char == '\t' => acc + TAB_SIZE as u16 - (acc % TAB_SIZE as u16),
                _ => acc + 1,
            }
        })
    }

    fn render_x_to_cursor_x(&self, cursor_render_x_position: u16) -> u16 {
        let mut calculated_render_x_position = 0;
        let mut calculated_x_position = 0;

        while (calculated_x_position as usize)
            < self.lines[self.cursor_position.y as usize]
                .text
                .chars()
                .count()
        {
            let char = self.lines[self.cursor_position.y as usize]
                .text
                .chars()
                .nth(calculated_x_position as usize);

            match char {
                Some(char) if char == '\t' => {
                    calculated_render_x_position +=
                        TAB_SIZE as u16 - (calculated_render_x_position % TAB_SIZE as u16)
                }
                _ => calculated_render_x_position += 1,
            }

            if calculated_render_x_position > cursor_render_x_position {
                return calculated_x_position as u16;
            }

            calculated_x_position += 1;
        }

        return calculated_x_position;
    }

    pub fn scroll(&mut self) -> () {
        self.cursor_position.render_x = if (self.cursor_position.y as usize) < self.lines.len() {
            self.cursor_x_to_render_x(self.cursor_position.x)
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

    fn insert_character_into_line(line: &mut Line, index: usize, character: char) -> () {
        line.text
            .insert(min(index, line.text.chars().count()), character);

        line.render = EditorInstance::get_render_text_from_text(&line.text);
    }

    fn insert_character(&mut self, character: char) -> () {
        if self.cursor_position.y as usize == self.lines.len() {
            self.lines.push(Line {
                text: String::new(),
                render: String::new(),
            });
        }

        EditorInstance::insert_character_into_line(
            &mut self.lines[self.cursor_position.y as usize],
            self.cursor_position.x as usize,
            character,
        );

        self.cursor_position.x += 1;
        self.edited = true;
    }

    fn append_string_to_line(line: &mut Line, string: &str) -> () {
        line.text += string;
        line.render = EditorInstance::get_render_text_from_text(&line.text);
    }

    fn delete_character_from_line(line: &mut Line, index: usize) -> () {
        line.text.remove(index);
        line.render = EditorInstance::get_render_text_from_text(&line.text);
    }

    fn delete_character(&mut self) -> () {
        if self.cursor_position.y as usize == self.lines.len()
            || (self.cursor_position.x == 0 && self.cursor_position.y == 0)
        {
            return;
        }

        if self.cursor_position.x > 0 {
            EditorInstance::delete_character_from_line(
                &mut self.lines[self.cursor_position.y as usize],
                (self.cursor_position.x - 1) as usize,
            );

            self.cursor_position.x -= 1;
        } else {
            self.cursor_position.x = self.lines[(self.cursor_position.y - 1) as usize]
                .text
                .chars()
                .count()
                .try_into()
                .expect("Could not convert line index usize into cursor x-position u16");

            let string_to_append = self.lines[self.cursor_position.y as usize].text.clone();

            EditorInstance::append_string_to_line(
                &mut self.lines[(self.cursor_position.y - 1) as usize],
                &string_to_append,
            );

            self.lines.remove(self.cursor_position.y as usize);
            self.cursor_position.y -= 1;
        }

        self.edited = true;
    }

    fn insert_line(&mut self) -> () {
        if self.cursor_position.x == 0 {
            self.lines.insert(
                self.cursor_position.y as usize,
                Line {
                    text: String::new(),
                    render: String::new(),
                },
            );
        } else {
            let current_line = &mut self.lines[self.cursor_position.y as usize];

            let new_next_line_text =
                current_line.text[self.cursor_position.x as usize..].to_string();

            let new_next_line_render_text =
                EditorInstance::get_render_text_from_text(&new_next_line_text);

            current_line.text.truncate(self.cursor_position.x as usize);

            current_line.render = EditorInstance::get_render_text_from_text(&current_line.text);

            self.lines.insert(
                self.cursor_position.y as usize + 1,
                Line {
                    text: new_next_line_text,
                    render: new_next_line_render_text,
                },
            );
        }

        self.cursor_position.y += 1;
        self.cursor_position.x = 0;

        self.edited = true;
    }

    fn find_text_callback(&mut self, query: &str, key: Key) -> () {
        match key {
            Key::U8(key) if key == b'\x1b' || key == b'\r' => {
                self.previous_search_match_line_index = None;
                self.search_direction = SearchDirection::Forward;
                return;
            }
            Key::Custom(EditorKey::ArrowRight) | Key::Custom(EditorKey::ArrowDown) => {
                self.search_direction = SearchDirection::Forward;
            }
            Key::Custom(EditorKey::ArrowLeft) | Key::Custom(EditorKey::ArrowUp) => {
                self.search_direction = SearchDirection::Backward;
            }
            _ => {
                self.previous_search_match_line_index = None;
                self.search_direction = SearchDirection::Forward;
            }
        }

        if self.previous_search_match_line_index.is_none() {
            self.search_direction = SearchDirection::Forward;
        }

        let mut current_line_index: isize = match self.previous_search_match_line_index {
            Some(i) => i
                .try_into()
                .expect("Failed to convert lines index usize to isize for search"),
            None => -1,
        };

        for _ in 0..self.lines.len() {
            current_line_index += match self.search_direction {
                SearchDirection::Forward => 1,
                SearchDirection::Backward => -1,
            };

            match current_line_index {
                -1 => {
                    let num_lines: isize = self
                        .lines
                        .len()
                        .try_into()
                        .expect("Failed to convert lines index usize to isize for search");

                    current_line_index = num_lines - 1;
                }
                x if x
                    == self
                        .lines
                        .len()
                        .try_into()
                        .expect("Failed to convert number of lines from usize to isize") =>
                {
                    current_line_index = 0
                }
                _ => {}
            }

            let current_line = &self.lines[current_line_index as usize];

            if current_line.render.contains(&query) {
                self.previous_search_match_line_index = Some(current_line_index as usize);

                self.cursor_position.y = current_line_index.try_into().expect(
                    "Could not convert matched line index usize into cursor y-position u32",
                );

                self.cursor_position.x = self.render_x_to_cursor_x(
                    current_line.render.find(&query).unwrap().try_into().expect(
                        "Could not convert matched line index usize into cursor x-position u16",
                    ),
                );

                self.line_scrolled_to = self
                    .lines
                    .len()
                    .try_into()
                    .expect("Could not convert line length usize into u32");

                return;
            }
        }
    }

    fn prompt_and_find_text(&mut self) -> () {
        let saved_cursor_position = self.cursor_position.clone();
        let saved_column_scrolled_to = self.column_scrolled_to;
        let saved_line_scrolled_to = self.line_scrolled_to;

        if let None = prompt_user(
            self,
            "Search (ESC to abort, arrows to jump): ",
            Some(EditorInstance::find_text_callback),
        ) {
            self.cursor_position = saved_cursor_position;
            self.column_scrolled_to = saved_column_scrolled_to;
            self.line_scrolled_to = saved_line_scrolled_to;
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
            buffer += "\r\n";
        }

        write!(io::stdout(), "{}", buffer).expect("Error writing to stdout while drawing rows");
        flush_stdout();
    }

    pub fn draw_status_bar(&self) -> () {
        let mut buffer = String::new();

        // Select Graphic Rendition (e.g. `<esc>[1;4;5;7m`)
        // 1: Bold, 4: Underscore, 5: Blink, 7: Inverted colors, 0: Clear all (default)
        buffer += "\x1b[7m";

        let mut status_bar_content = format!(
            " {:.20} - {} lines{} ",
            self.file_name.as_ref().unwrap_or(&"[New File]".to_string()),
            self.lines.len(),
            if self.edited { " (modified)" } else { "" }
        );

        status_bar_content.truncate(self.window_size.columns as usize);

        buffer += &status_bar_content;

        let space_left = self.window_size.columns as usize - status_bar_content.chars().count();

        let mut cursor_position_information =
            format!("{}/{} ", self.cursor_position.y + 1, self.lines.len());

        cursor_position_information.truncate(space_left);

        let gap = space_left - cursor_position_information.chars().count();

        buffer += &" ".repeat(gap);
        buffer += &cursor_position_information;

        buffer += "\x1b[m";
        buffer += "\r\n"; // New line for status message

        write!(io::stdout(), "{}", buffer)
            .expect("Error writing to stdout while drawing status bar");
        flush_stdout();
    }

    pub fn set_status_message(&mut self, message: &str) -> () {
        self.status_message = Some(StatusMessage {
            message: message.to_string(),
            time_set: Instant::now(),
        });
    }

    pub fn draw_status_message_bar(&mut self) -> () {
        let mut buffer = "\x1b[K".to_string(); // Erase In Line (2: whole, 1: to left, 0: to right [default])

        if let Some(status_message) = &self.status_message {
            if status_message.time_set.elapsed().as_secs() < 5 {
                let mut message = format!(" {} ", status_message.message.clone());
                message.truncate(self.window_size.columns as usize);

                buffer += &message;
            }
        }

        write!(io::stdout(), "{buffer}")
            .expect("Error writing to stdout while drawing status message bar");
        flush_stdout();
    }
}
