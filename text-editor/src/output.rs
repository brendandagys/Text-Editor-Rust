use crate::{
    editor_instance::EditorInstance,
    input::{read_key_input, EditorKey, Key},
    utils::flush_stdout,
};
use std::io::{self, Write};

pub fn move_cursor_to_top_left() -> () {
    // H: Cursor Position, e.g. <esc>[1;1H]
    write!(io::stdout(), "\x1b[H").expect("Error positioning cursor at top-left before draw");
    flush_stdout();
}

pub fn clear_display() -> () {
    // J: Erase in Display, 2: clear entire screen
    write!(io::stdout(), "\x1b[2J").expect("Error clearing screen");
    flush_stdout();
}

fn hide_cursor() -> () {
    // l: Reset mode
    write!(io::stdout(), "\x1b[?25l").expect("Error hiding cursor");
    flush_stdout();
}

fn show_cursor() -> () {
    // h: Set mode
    write!(io::stdout(), "\x1b[?25h").expect("Error hiding cursor");
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

pub fn prompt_user(editor_instance: &mut EditorInstance, prompt: &str) -> Option<String> {
    let mut buffer = String::new();

    loop {
        editor_instance.set_status_message(&format!("{}{}", prompt, buffer));
        refresh_screen(editor_instance);

        if let Some(char) = read_key_input() {
            match char {
                Key::U8(b'\x1b') => {
                    editor_instance.set_status_message("");
                    return None;
                }
                Key::Custom(EditorKey::Backspace) => {
                    buffer.pop();
                }
                Key::U8(b'\r') => {
                    if !buffer.is_empty() {
                        editor_instance.set_status_message("");
                        return Some(buffer);
                    }
                }
                Key::U8(byte) if !(byte as char).is_ascii_control() => {
                    buffer += &(byte as char).to_string()
                }
                _ => {}
            }
        }
    }
}
