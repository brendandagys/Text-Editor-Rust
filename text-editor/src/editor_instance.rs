use crate::{
    globals::{
        Syntax, HIGHLIGHT_NUMBERS, HIGHLIGHT_STRINGS, LINE_NUMBER_GAP, QUIT_CONFIRMATION_COUNT,
        SYNTAX_CONFIGURATIONS, TAB_SIZE, VERSION,
    },
    input::{EditorKey, Key},
    output::{clear_display, move_cursor_to_top_left, prompt_user},
    terminal::disable_raw_mode,
    utils::{ctrl_key, flush_stdout, get_file_name_from_path, get_window_size, lines_to_string},
    WindowSize,
};
use std::{
    cmp::{max, min},
    fs::{self, OpenOptions},
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

#[derive(Clone, PartialEq)]
enum HighlightType {
    Normal,
    Number,
    String,
    Comment,
    MultiLineComment,
    Keyword,
    Type,
    SearchMatch,
}

pub struct Line {
    pub text: String,
    render: String,
    highlight: Vec<HighlightType>,
    index: usize,
    has_open_multiline_comment: bool,
}

struct StatusMessage {
    message: String,
    time_set: Instant,
}

enum SearchDirection {
    Forward,
    Backward,
}

struct File {
    path: String,
    name: String,
}

struct SavedHighlight {
    line_index: usize,
    highlight: Vec<HighlightType>,
}

pub struct EditorInstance {
    original_termios: Termios,
    pub window_size: WindowSize,
    pub cursor_position: CursorPosition,
    lines: Vec<Line>,
    line_scrolled_to: u32,
    column_scrolled_to: u16,
    file: Option<File>,
    status_message: Option<StatusMessage>,
    edited: bool,
    quit_confirmations: u8,
    previous_search_match_line_index: Option<usize>,
    search_direction: SearchDirection,
    saved_highlight: Option<SavedHighlight>,
    syntax: Option<&'static Syntax>,
    num_columns_for_line_number: usize,
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
            syntax: None,
            lines: vec![],
            line_scrolled_to: 0,
            column_scrolled_to: 0,
            file: None,
            status_message: None,
            edited: false,
            quit_confirmations: 0,
            previous_search_match_line_index: None,
            search_direction: SearchDirection::Forward,
            saved_highlight: None,
            num_columns_for_line_number: 0,
        }
    }

    fn get_current_line(&self) -> &Line {
        &self.lines[self.cursor_position.y as usize] // TODO: is this always safe?
    }

    fn get_render_text_from_text(text: &str) -> String {
        let mut render = String::new();
        let mut render_index = 0;

        for char in text.chars() {
            if char == '\t' {
                render.push(' ');
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

    fn is_separator(char: char) -> bool {
        char.is_ascii_punctuation() || char.is_ascii_whitespace() || char == '\n'
    }

    fn set_line_highlight(&mut self, line_index: usize) -> () {
        let chars = &mut self.lines[line_index].render.chars();
        let num_chars = chars.clone().count();
        let mut highlight = vec![HighlightType::Normal; num_chars];

        if let Some(syntax) = self.syntax {
            let mut is_previous_char_separator = true;
            let mut current_string_quote = None;

            let current_line = &self.lines[line_index];
            let mut is_part_of_multiline_comment =
                current_line.index > 0 && self.lines[line_index - 1].has_open_multiline_comment;

            let mut i = 0;
            'outer: while i < num_chars {
                let char = match chars.next() {
                    Some(char) => char,
                    None => break,
                };

                let previous_highlight = if i > 0 {
                    &highlight[i - 1]
                } else {
                    &HighlightType::Normal
                };

                if current_string_quote.is_none() {
                    let mut single_line_comment_iterator = syntax.single_line_comment_start.chars();

                    if !is_part_of_multiline_comment
                        && chars.clone().count() >= single_line_comment_iterator.clone().count() - 1
                    {
                        if let Some(first_single_line_comment_char) =
                            single_line_comment_iterator.next()
                        {
                            if first_single_line_comment_char == char
                                && chars
                                    .clone()
                                    .zip(single_line_comment_iterator)
                                    .all(|(char, comment_char)| char == comment_char)
                            {
                                highlight[i..].fill(HighlightType::Comment);
                                break;
                            }
                        }
                    }

                    let mut multi_line_comment_start_iterator =
                        syntax.multi_line_comment_start.chars();

                    let multi_line_comment_start_length =
                        multi_line_comment_start_iterator.clone().count();

                    let mut multi_line_comment_end_iterator = syntax.multi_line_comment_end.chars();

                    let multi_line_comment_end_length =
                        multi_line_comment_end_iterator.clone().count();

                    if is_part_of_multiline_comment {
                        highlight[i] = HighlightType::MultiLineComment;

                        if let Some(first_multi_line_comment_end_char) =
                            multi_line_comment_end_iterator.next()
                        {
                            if chars.clone().count() >= multi_line_comment_end_length - 1
                                && first_multi_line_comment_end_char == char
                                && chars
                                    .clone()
                                    .zip(multi_line_comment_end_iterator.clone())
                                    .all(|(char, comment_char)| char == comment_char)
                            {
                                highlight[i + 1..i + multi_line_comment_end_length]
                                    .fill(HighlightType::MultiLineComment);

                                i += multi_line_comment_end_length;
                                is_part_of_multiline_comment = false;
                                is_previous_char_separator = true;

                                for _ in 0..multi_line_comment_end_length - 1 {
                                    chars.next();
                                }

                                continue;
                            }
                        }

                        i += 1;
                        continue;
                    } else if !is_part_of_multiline_comment
                        && chars.clone().count() >= multi_line_comment_start_length - 1
                    {
                        if let Some(first_multi_line_comment_start_char) =
                            multi_line_comment_start_iterator.next()
                        {
                            if first_multi_line_comment_start_char == char
                                && chars
                                    .clone()
                                    .zip(multi_line_comment_start_iterator.clone())
                                    .all(|(char, comment_char)| char == comment_char)
                            {
                                highlight[i..i + multi_line_comment_start_length]
                                    .fill(HighlightType::MultiLineComment);

                                i += multi_line_comment_start_length;
                                is_part_of_multiline_comment = true;

                                for _ in 0..multi_line_comment_start_length - 1 {
                                    chars.next();
                                }

                                continue;
                            }
                        }
                    }
                }

                if (syntax.flags & HIGHLIGHT_STRINGS) != 0 {
                    match current_string_quote {
                        Some(quote) => {
                            highlight[i] = HighlightType::String;

                            if char == '\\' {
                                highlight[i + 1] = HighlightType::String;
                                i += 2;
                                chars.next();
                                continue;
                            }

                            if char == quote {
                                current_string_quote = None;
                            }

                            i += 1;
                            is_previous_char_separator = true;
                            continue;
                        }
                        None => {
                            if char == '"'
                                || (char == '\''
                                    && (i == 0
                                        || self.lines[line_index].render.chars().nth(i - 1)
                                            != Some('&')))
                            {
                                current_string_quote = Some(char);
                                highlight[i] = HighlightType::String;
                                i += 1;
                                continue;
                            }
                        }
                    }
                }

                if (syntax.flags & HIGHLIGHT_NUMBERS) != 0 {
                    if char.is_ascii_digit()
                        && (is_previous_char_separator
                            || previous_highlight == &HighlightType::Number)
                        || (char == '.'
                            && previous_highlight == &HighlightType::Number
                            // Rust: 3..
                            && chars.clone().next() != Some('.'))
                    {
                        highlight[i] = HighlightType::Number;
                        i += 1;
                        is_previous_char_separator = false;
                        continue;
                    }
                }

                if is_previous_char_separator {
                    for (k, keyword) in syntax.keywords.iter().chain(syntax.types).enumerate() {
                        let mut keyword_iterator = keyword.chars();
                        let keyword_length = keyword.chars().count();

                        if (i == 0 || self.lines[line_index].render.chars().nth(i - 1) != Some('_'))
                            && chars.clone().count() >= keyword_length - 1
                        {
                            if let Some(keyword_first_char) = keyword_iterator.next() {
                                if keyword_first_char == char
                                    && chars
                                        .clone()
                                        .zip(keyword_iterator)
                                        .all(|(char, keyword_char)| char == keyword_char)
                                    && match chars.clone().nth(keyword_length - 1) {
                                        Some(char) => EditorInstance::is_separator(char),
                                        None => true,
                                    }
                                {
                                    for j in i..i + keyword_length {
                                        highlight[j] = if k >= syntax.keywords.len() {
                                            HighlightType::Type
                                        } else {
                                            HighlightType::Keyword
                                        };

                                        if j < i + keyword_length - 1 {
                                            chars.next();
                                        }
                                    }

                                    i += keyword_length;
                                    is_previous_char_separator = false;
                                    continue 'outer;
                                }
                            }
                        }
                    }
                }

                is_previous_char_separator = EditorInstance::is_separator(char);
                i += 1;
            }

            let did_is_part_of_multiline_comment_change =
                current_line.has_open_multiline_comment != is_part_of_multiline_comment;

            self.lines[line_index].has_open_multiline_comment = is_part_of_multiline_comment;

            if did_is_part_of_multiline_comment_change && line_index < self.lines.len() - 1 {
                self.set_line_highlight(line_index + 1);
            }
        }

        self.lines[line_index].highlight = highlight;
    }

    fn get_color_from_highlight_type(highlight_type: &HighlightType) -> i8 {
        match highlight_type {
            HighlightType::Normal => 37,
            HighlightType::Number => 93,
            HighlightType::String => 33,
            HighlightType::Comment | HighlightType::MultiLineComment => 36,
            HighlightType::Keyword => 95,
            HighlightType::Type => 92,
            HighlightType::SearchMatch => 34,
        }
    }

    fn update_line_highlights(&mut self) -> () {
        for line_index in 0..self.lines.len() {
            self.set_line_highlight(line_index);
        }
    }

    fn set_syntax_from_file_name(&mut self) -> () {
        let file_name = match &self.file {
            Some(file) => file.name.clone(),
            None => return,
        };

        let file_extension = match file_name.rfind('.') {
            Some(index) => Some(&file_name[index..]),
            None => None,
        };

        for configuration in SYNTAX_CONFIGURATIONS {
            for file_match in configuration.file_match {
                match file_extension {
                    Some(file_extension) => {
                        if file_extension == *file_match {
                            self.syntax = Some(configuration);
                            self.update_line_highlights();
                        }
                    }
                    None => {
                        if file_name.contains(file_match) {
                            self.syntax = Some(configuration);
                            self.update_line_highlights();
                        }
                    }
                }
            }
        }
    }

    pub fn open(&mut self, file_path: &str) {
        let reader = BufReader::new(
            fs::File::open(file_path).expect("Failed to open file at specified path"),
        );

        for line in reader.lines() {
            let index = self.lines.len();
            let text = line.expect(&format!("Failed to read line from file: {}", file_path));
            let render = EditorInstance::get_render_text_from_text(&text);

            self.lines.push(Line {
                text,
                render,
                highlight: vec![],
                index,
                has_open_multiline_comment: false,
            });

            self.set_num_columns_for_line_number();
            self.set_line_highlight(index);
        }

        self.file = Some(File {
            path: file_path.to_string(),
            name: get_file_name_from_path(file_path),
        });

        self.set_syntax_from_file_name();
    }

    fn save(&mut self) -> () {
        if self.file.is_none() {
            match prompt_user::<fn(&mut EditorInstance, &str, Key)>(self, "Save as: ", None) {
                Some(file_path) => {
                    self.file = Some(File {
                        name: get_file_name_from_path(&file_path),
                        path: file_path,
                    });

                    self.set_syntax_from_file_name();
                }
                None => {
                    self.set_status_message("Save aborted");
                    return;
                }
            }
        }

        if let Some(file) = &self.file {
            let mut fs_file = match OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .mode(0o644) // Owner R/W; others R
                .open(&file.path)
            {
                Ok(fs_file) => fs_file,
                Err(e) => {
                    self.set_status_message(&format!(
                        "Failed to open {} for saving: {:?}",
                        file.path, e
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

            if let Err(e) = fs_file.set_len(content_length) {
                self.set_status_message(&format!(
                    "Failed to truncate {} to new length: {:?}",
                    file.path, e
                ));
                return;
            }

            match fs_file.write_all(content.as_bytes()) {
                Ok(_) => {
                    self.set_status_message(&format!("{} bytes written to disk", content.len()));
                    self.edited = false;
                }
                Err(e) => {
                    self.set_status_message(&format!("Failed to write to {}: {:?}", file.path, e))
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

            Key::Custom(EditorKey::Home) => {
                self.cursor_position.x = self
                    .num_columns_for_line_number
                    .try_into()
                    .expect("Failed to convert new cursor x-position usize to u16");
            }
            Key::Custom(EditorKey::End) => {
                if (self.cursor_position.y as usize) < self.lines.len() {
                    let num_characters_in_line: u16 = self
                        .get_current_line()
                        .text
                        .chars()
                        .count()
                        .try_into()
                        .expect("Failed to convert line length usize to u16");

                    let line_number_columns_offset: u16 = self
                        .num_columns_for_line_number
                        .try_into()
                        .expect("Failed to convert usize to u16");

                    self.cursor_position.x = num_characters_in_line + line_number_columns_offset;
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
                self.cursor_position.y = min(
                    self.lines
                        .len()
                        .try_into()
                        .expect("Failed to convert usize to u16"),
                    self.line_scrolled_to + self.window_size.rows - 1,
                );

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
            Some(self.get_current_line())
        } else {
            None
        };

        match direction {
            CursorMovement::Left => {
                if self.cursor_position.x as usize > self.num_columns_for_line_number {
                    self.cursor_position.x -= 1;
                } else if self.cursor_position.y > 0 {
                    self.cursor_position.y -= 1;

                    let num_characters_in_previous_line: u16 = self
                        .get_current_line()
                        .text
                        .chars()
                        .count()
                        .try_into()
                        .expect("Failed to convert line length usize to u16");

                    let line_number_columns_offset: u16 = self
                        .num_columns_for_line_number
                        .try_into()
                        .expect("Failed to convert usize to u16");

                    self.cursor_position.x =
                        num_characters_in_previous_line + line_number_columns_offset
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
                    if (self.cursor_position.x as usize)
                        < current_line.text.chars().count() + self.num_columns_for_line_number
                    {
                        self.cursor_position.x += 1;
                    } else if self.cursor_position.x as usize // TODO: simplify?
                        == current_line.text.chars().count() + self.num_columns_for_line_number
                    {
                        self.cursor_position.y += 1;
                        self.cursor_position.x = self
                            .num_columns_for_line_number
                            .try_into()
                            .expect("Failed to convert usize to u16");
                    }
                }
            }
        }

        let current_line_after_cursor_move = if (self.cursor_position.y as usize) < self.lines.len()
        {
            Some(self.get_current_line())
        } else {
            None
        };

        let line_length = match current_line_after_cursor_move {
            Some(line) => line.text.chars().count() + self.num_columns_for_line_number,
            None => 0,
        };

        self.cursor_position.x = min(
            self.cursor_position.x,
            line_length
                .try_into()
                .expect("Failed to convert line length usize to u16"),
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
            match self.get_current_line().text.chars().nth(x as usize) {
                Some(char) if char == '\t' => acc + TAB_SIZE as u16 - (acc % TAB_SIZE as u16),
                _ => acc + 1,
            }
        })
    }

    fn render_x_to_cursor_x(&self, cursor_render_x_position: u16) -> u16 {
        let mut calculated_render_x_position = 0;
        let mut calculated_x_position = 0;

        while (calculated_x_position as usize) < self.get_current_line().text.chars().count() {
            let char = self
                .get_current_line()
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
        let num_columns_for_line_number: u16 = self
            .num_columns_for_line_number
            .try_into()
            .expect("Failed to convert usize to u16");

        self.cursor_position.render_x = if (self.cursor_position.y as usize) < self.lines.len() {
            self.cursor_x_to_render_x(self.cursor_position.x - num_columns_for_line_number)
        } else {
            0
        } + num_columns_for_line_number;

        if self.cursor_position.y < self.line_scrolled_to {
            self.line_scrolled_to = self.cursor_position.y;
        }

        if self.cursor_position.y >= self.line_scrolled_to + self.window_size.rows {
            self.line_scrolled_to = self.cursor_position.y - self.window_size.rows + 1;
        }

        if self.cursor_position.render_x - num_columns_for_line_number < self.column_scrolled_to {
            self.column_scrolled_to = self.cursor_position.render_x - num_columns_for_line_number;
        }

        if self.cursor_position.render_x >= self.column_scrolled_to + self.window_size.columns {
            self.column_scrolled_to = self.cursor_position.render_x - self.window_size.columns + 1;
        }
    }

    fn insert_character_into_line(&mut self, character: char) -> () {
        let index = self.cursor_position.y as usize;
        let line = &mut self.lines[index];
        line.text.insert(
            if self.num_columns_for_line_number > self.cursor_position.x as usize {
                // TODO: replace this and others with saturating sub
                0
            } else {
                self.cursor_position.x as usize - self.num_columns_for_line_number
            },
            character,
        );
        line.render = EditorInstance::get_render_text_from_text(&line.text);
        self.set_line_highlight(index);
    }

    fn insert_character(&mut self, character: char) -> () {
        if self.cursor_position.y as usize == self.lines.len() {
            self.lines.push(Line {
                text: String::new(),
                render: String::new(),
                highlight: vec![],
                index: self.lines.len(),
                has_open_multiline_comment: false,
            });

            self.set_num_columns_for_line_number();
        }

        self.insert_character_into_line(character);
        self.cursor_position.x += 1;
        self.edited = true;
    }

    fn append_string_to_previous_line(&mut self, string: &str) -> () {
        let previous_line_index = (self.cursor_position.y - 1) as usize;
        let line = &mut self.lines[previous_line_index];
        line.text.push_str(string);
        line.render = EditorInstance::get_render_text_from_text(&line.text);
        self.set_line_highlight(previous_line_index);
    }

    fn delete_character_from_line(&mut self) -> () {
        let line_index = self.cursor_position.y as usize;
        let line = &mut self.lines[line_index];
        line.text
            .remove(self.cursor_position.x as usize - self.num_columns_for_line_number - 1);
        line.render = EditorInstance::get_render_text_from_text(&line.text);
        self.set_line_highlight(line_index);
    }

    fn delete_character(&mut self) -> () {
        let line_index = self.cursor_position.y as usize;

        if line_index == self.lines.len()
            || (self.cursor_position.x as usize == self.num_columns_for_line_number
                && line_index == 0)
        {
            return;
        }

        if self.cursor_position.x as usize > self.num_columns_for_line_number {
            self.delete_character_from_line();
            self.cursor_position.x -= 1;
        } else {
            let line_number_columns_offset: u16 = self
                .num_columns_for_line_number
                .try_into()
                .expect("Failed to convert usize to u16");

            let previous_line_length: u16 = self.lines[line_index - 1]
                .text
                .chars()
                .count()
                .try_into()
                .expect("Failed to convert line index usize to cursor x-position u16");

            self.cursor_position.x = previous_line_length + line_number_columns_offset;

            let string_to_append = self.get_current_line().text.clone();

            self.append_string_to_previous_line(&string_to_append);
            self.lines.remove(line_index);

            for line in self.lines.iter_mut().skip(line_index) {
                line.index -= 1;
            }

            self.set_num_columns_for_line_number();
            self.cursor_position.y -= 1;
        }

        self.edited = true;
    }

    fn insert_line(&mut self) -> () {
        let line_index = self.cursor_position.y as usize;

        if self.cursor_position.x as usize == self.num_columns_for_line_number {
            self.lines.insert(
                line_index,
                Line {
                    text: String::new(),
                    render: String::new(),
                    highlight: vec![],
                    index: line_index,
                    has_open_multiline_comment: false,
                },
            );

            for line in self.lines.iter_mut().skip(line_index + 1) {
                line.index += 1;
            }
        } else {
            let new_next_line_text = self.lines[line_index].text
                [self.cursor_position.x as usize - self.num_columns_for_line_number..]
                .to_string();

            let new_next_line_render_text =
                EditorInstance::get_render_text_from_text(&new_next_line_text);

            self.lines[line_index]
                .text
                .truncate(self.cursor_position.x as usize - self.num_columns_for_line_number);

            self.lines[line_index].render =
                EditorInstance::get_render_text_from_text(&self.lines[line_index].text);

            self.set_line_highlight(line_index);

            self.lines.insert(
                line_index + 1,
                Line {
                    text: new_next_line_text,
                    render: new_next_line_render_text,
                    highlight: vec![],
                    index: line_index + 1,
                    has_open_multiline_comment: false,
                },
            );

            self.set_line_highlight(line_index + 1);

            for line in self.lines.iter_mut().skip(line_index + 2) {
                line.index += 1;
            }
        }

        self.set_num_columns_for_line_number();

        self.cursor_position.y += 1;
        self.cursor_position.x = self
            .num_columns_for_line_number
            .try_into()
            .expect("Failed to convert new cursor x-position usize to u16");

        self.edited = true;
    }

    fn find_text_callback(&mut self, query: &str, key: Key) -> () {
        if let Some(saved_highlight) = self.saved_highlight.take() {
            self.lines[saved_highlight.line_index].highlight = saved_highlight.highlight;
        }

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

            if self.lines[current_line_index as usize]
                .render
                .contains(&query)
            {
                self.previous_search_match_line_index = Some(current_line_index as usize);

                self.cursor_position.y = current_line_index
                    .try_into()
                    .expect("Failed to convert matched line index usize to cursor y-position u32");

                self.cursor_position.x = self.render_x_to_cursor_x(
                    self.lines[current_line_index as usize]
                        .render
                        .find(&query)
                        .unwrap()
                        .try_into()
                        .expect(
                            "Failed to convert matched line index usize to cursor x-position u16",
                        ),
                ) + self.num_columns_for_line_number as u16;

                self.line_scrolled_to = self
                    .lines
                    .len()
                    .try_into()
                    .expect("Failed to convert line length usize to u32");

                self.saved_highlight = Some(SavedHighlight {
                    line_index: current_line_index as usize,
                    highlight: self.lines[current_line_index as usize].highlight.clone(),
                });

                let start = self.cursor_position.x as usize - self.num_columns_for_line_number;
                self.lines[current_line_index as usize].highlight[start..start + query.len()]
                    .fill(HighlightType::SearchMatch);

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

    fn add_welcome_message_to_buffer(&self, buffer: &mut String) -> () {
        let mut message = format!("Brendan's text editor --- version {VERSION}");
        message.truncate(self.window_size.columns as usize);

        let message_length: u16 = message
            .chars()
            .count()
            .try_into()
            .expect("Failed to convert welcome message length to u16 during screen refresh");

        let mut padding = (self.window_size.columns - message_length) / 2;

        if padding > 0 {
            buffer.push('~');
            padding -= 1;
        }

        for _ in 0..padding {
            buffer.push(' ');
        }

        buffer.push_str(&message);
    }

    fn set_num_columns_for_line_number(&mut self) -> () {
        let num_lines = self.lines.len();

        self.num_columns_for_line_number = if num_lines > 0 {
            num_lines.to_string().len() + LINE_NUMBER_GAP as usize
        } else {
            0
        };

        if (self.cursor_position.x as usize) < self.num_columns_for_line_number {
            self.cursor_position.x = self
                .num_columns_for_line_number
                .try_into()
                .expect("Failed to convert line number column index to u16");
        }
    }

    /// Uses a String as a buffer to store all lines, before calling `write` once
    /// Prints a welcome message in the middle of the screen using its row/column count
    pub fn draw_rows(&mut self) -> () {
        let mut buffer = String::new();

        if (self.cursor_position.x as usize) < self.num_columns_for_line_number {
            self.cursor_position.x = self
                .num_columns_for_line_number
                .try_into()
                .expect("Failed to convert line number column index to u16");
        }

        for row in 0..self.window_size.rows {
            let scrolled_to_row = row + self.line_scrolled_to;

            if scrolled_to_row as usize >= self.lines.len() {
                if self.lines.len() == 0 && row == self.window_size.rows / 3 {
                    self.add_welcome_message_to_buffer(&mut buffer);
                } else {
                    buffer.push('~');
                }
            } else {
                let line = &self.lines[scrolled_to_row as usize];
                let line_content = &line.render;
                let line_highlight = &line.highlight;

                let mut line_prefix = (line.index + 1).to_string();

                line_prefix.push_str(
                    &" ".repeat(self.num_columns_for_line_number - line_prefix.chars().count()),
                );

                let start = self.column_scrolled_to as usize;
                let end = max(
                    start + self.window_size.columns as usize - self.num_columns_for_line_number,
                    start,
                );

                let num_characters = line_content.chars().count();

                let to_iter = if num_characters > end {
                    line_prefix.push_str(&line_content[start..end]);
                    Some(&line_prefix)
                } else if num_characters > start {
                    line_prefix.push_str(&line_content[start..]);
                    Some(&line_prefix)
                } else {
                    None
                };

                match to_iter {
                    Some(to_iter) => {
                        let mut current_highlight_type = &HighlightType::Normal;

                        to_iter.chars().enumerate().for_each(|(i, char)| {
                            if i < self.num_columns_for_line_number {
                                buffer.push(line_prefix.chars().nth(i).unwrap()); // TODO: push once, above
                                return;
                            }

                            if char.is_ascii_control() {
                                buffer.push_str("\x1b[7m"); // Inverted colors
                                buffer.push(if char as u8 <= 26 {
                                    ('@' as u8 + char as u8) as char
                                } else {
                                    '?'
                                });
                                buffer.push_str("\x1b[m"); // Reset text formatting

                                if current_highlight_type != &HighlightType::Normal {
                                    buffer.push_str("\x1b[");
                                    buffer.push_str(
                                        &EditorInstance::get_color_from_highlight_type(
                                            current_highlight_type,
                                        )
                                        .to_string(),
                                    );
                                    buffer.push('m');
                                }
                            } else {
                                let highlight_type =
                                    &line_highlight[start + i - self.num_columns_for_line_number];

                                match highlight_type {
                                    HighlightType::Normal => {
                                        if current_highlight_type != &HighlightType::Normal {
                                            buffer.push_str("\x1b[39m"); // m: Select Graphic Rendition (39: default color)
                                            current_highlight_type = &HighlightType::Normal;
                                        }
                                    }
                                    _ => {
                                        if current_highlight_type != highlight_type {
                                            current_highlight_type = highlight_type;
                                            buffer.push_str("\x1b[");
                                            buffer.push_str(
                                                &EditorInstance::get_color_from_highlight_type(
                                                    highlight_type,
                                                )
                                                .to_string(),
                                            );
                                            buffer.push('m');
                                        }
                                    }
                                };

                                buffer.push(char);
                            }
                        });

                        buffer.push_str("\x1b[39m"); // m: Select Graphic Rendition (39: default color)
                    }
                    None => buffer.push_str(&line_prefix),
                }
            }

            buffer.push_str("\x1b[K\r\n"); // K: Erase In Line (2: whole, 1: to left, 0: to right [default])
        }

        write!(io::stdout(), "{}", buffer).expect("Failed to write to stdout while drawing rows");
        flush_stdout();
    }

    pub fn draw_status_bar(&self) -> () {
        let mut buffer = String::new();

        // Select Graphic Rendition (e.g. `<esc>[1;4;5;7m`)
        // 1: Bold, 4: Underscore, 5: Blink, 7: Inverted colors, 0: Clear all (default)
        buffer.push_str("\x1b[7m");

        let mut status_bar_content = format!(
            " {:.20} - {} lines{} ",
            match &self.file {
                Some(file) => &file.name,
                None => "[New File]",
            },
            self.lines.len(),
            if self.edited { " (modified)" } else { "" }
        );

        status_bar_content.truncate(self.window_size.columns as usize);

        buffer.push_str(&status_bar_content);

        let space_left = self.window_size.columns as usize - status_bar_content.chars().count();

        let mut cursor_position_information = format!(
            "{}{}/{} ",
            match &self.syntax {
                Some(syntax) => format!("{} ", syntax.file_type),
                None => "".to_string(),
            },
            self.cursor_position.y + 1,
            self.lines.len()
        );

        cursor_position_information.truncate(space_left);

        let gap = space_left - cursor_position_information.chars().count();

        buffer.push_str(&" ".repeat(gap));
        buffer.push_str(&cursor_position_information);

        buffer.push_str("\x1b[m\r\n"); // Reset text formatting and add newline for status message

        write!(io::stdout(), "{}", buffer)
            .expect("Failed to write to stdout while drawing status bar");
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

                buffer.push_str(&message);
            }
        }

        write!(io::stdout(), "{buffer}")
            .expect("Failed to write to stdout while drawing status message bar");
        flush_stdout();
    }
}
