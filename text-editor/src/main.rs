use editor_instance::EditorInstance;
use input::process_keypress;
use output::{clear_display, refresh_screen};
use std::error::Error;
use std::sync::{Arc, RwLock};
use terminal::{enable_raw_mode, get_populated_termios};
use utils::{set_panic_hook, watch_for_window_size_change};

mod editor_instance;
mod globals;
mod input;
mod output;
mod terminal;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    clear_display(); // Does not rely on raw mode

    let termios = get_populated_termios();
    set_panic_hook(termios);
    enable_raw_mode(termios);

    let active_editor = Arc::new(RwLock::new(EditorInstance::new(termios)));
    watch_for_window_size_change(Arc::clone(&active_editor));

    loop {
        let EditorInstance {
            window_size,
            cursor_position,
            ..
        } = *{
            active_editor
                .read()
                .expect("Could not get reader for editor")
        };

        refresh_screen(window_size, cursor_position);

        process_keypress(
            &mut active_editor
                .write()
                .expect("Could not get reader for editor"),
        );
    }
}
