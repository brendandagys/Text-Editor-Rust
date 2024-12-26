use crate::{
    editor_instance::EditorInstance,
    input::{read_key_input, EditorKey, Key},
    utils::flush_stdout,
};
use std::io::{self, Write};

#[rustfmt::skip]
pub enum AnsiEscapeCode {
    BackgroundGreen,  // \x1b[42m
    BackgroundRed,    // \x1b[41m
    ClearScreen,      // \x1b[2J   (J: Erase in Display, 2: clear entire screen)
    CursorHide,       // \x1b[?25l (?: private mode setting, 25: cursor visibility, l: reset/disable)
    CursorReport,     // \x1b[6n   (Cursor Position Report [reply e.g. `\x1b[24;80R`])
    CursorShow,       // \x1b[?25h (?: private mode setting, 25: cursor visibility, h: set/enable)
    CursorToTopLeft,  // \x1b[H    (H: Cursor Position, e.g. `<esc>[1;1H]`)
    DefaultColor,     // \x1b[39m  (m: Select Graphic Rendition [39: default color])
    EraseLineToRight, // \x1b[K    (K: Erase In Line (2: whole, 1: to left, 0: to right [default])
    ForegroundBlack,  // \x1b[30m
    Reset,            // \x1b[m
    ReverseMode,      // \x1b[7m
}

impl AnsiEscapeCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnsiEscapeCode::BackgroundGreen => "\x1b[42m",
            AnsiEscapeCode::BackgroundRed => "\x1b[41m",
            AnsiEscapeCode::ClearScreen => "\x1b[2J",
            AnsiEscapeCode::CursorHide => "\x1b[?25l",
            AnsiEscapeCode::CursorReport => "\x1b[6n",
            AnsiEscapeCode::CursorShow => "\x1b[?25h",
            AnsiEscapeCode::CursorToTopLeft => "\x1b[H",
            AnsiEscapeCode::DefaultColor => "\x1b[39m",
            AnsiEscapeCode::EraseLineToRight => "\x1b[K",
            AnsiEscapeCode::ForegroundBlack => "\x1b[30m",
            AnsiEscapeCode::Reset => "\x1b[m",
            AnsiEscapeCode::ReverseMode => "\x1b[7m",
        }
    }

    pub fn as_string(&self) -> String {
        self.as_str().to_string()
    }
}

pub fn move_cursor_to_top_left() -> () {
    write!(io::stdout(), "{}", AnsiEscapeCode::CursorToTopLeft.as_str())
        .expect("Failed to position cursor at top-left before draw");
    flush_stdout();
}

pub fn clear_display() -> () {
    write!(io::stdout(), "{}", AnsiEscapeCode::ClearScreen.as_str())
        .expect("Failed to clear screen");
    flush_stdout();
}

fn hide_cursor() -> () {
    write!(io::stdout(), "{}", AnsiEscapeCode::CursorHide.as_str()).expect("Failed to hide cursor");
    flush_stdout();
}

fn show_cursor() -> () {
    write!(io::stdout(), "{}", AnsiEscapeCode::CursorShow.as_str()).expect("Failed to show cursor");
    flush_stdout();
}

pub fn refresh_screen(editor_instance: &mut EditorInstance) -> () {
    // Escape sequences begin with escape characters `\x1b` (27) and '['
    // Escape sequence commands take arguments that come before the command itself
    // Arguments are separated by a ';'
    // https://vt100.net/docs/vt100-ug/chapter3.html

    hide_cursor();
    editor_instance.scroll();
    move_cursor_to_top_left();
    editor_instance.draw_rows();
    editor_instance.draw_status_bar();
    editor_instance.draw_status_message_bar();
    editor_instance.move_cursor_to_position();
    show_cursor();
}

pub fn prompt_user<F: Fn(&mut EditorInstance, &str, Key)>(
    editor_instance: &mut EditorInstance,
    prompt: &str,
    callback: Option<F>,
) -> Option<String> {
    let mut buffer = String::new();

    loop {
        editor_instance.set_status_message(&format!("{}{}", prompt, buffer));
        refresh_screen(editor_instance);

        if let Some(key) = read_key_input() {
            match key {
                Key::U8(b'\x1b') => {
                    editor_instance.set_status_message("");

                    if let Some(callback) = &callback {
                        callback(editor_instance, &buffer, key);
                    }

                    return None;
                }
                Key::Custom(EditorKey::Backspace) => {
                    buffer.pop();
                }
                Key::U8(b'\r') => {
                    if !buffer.is_empty() {
                        editor_instance.set_status_message("");

                        if let Some(callback) = &callback {
                            callback(editor_instance, &buffer, key);
                        }

                        return Some(buffer);
                    }
                }
                Key::U8(byte) if !(byte as char).is_ascii_control() => buffer.push(byte as char),
                _ => {}
            }

            if let Some(callback) = &callback {
                callback(editor_instance, &buffer, key);
            }
        }
    }
}
