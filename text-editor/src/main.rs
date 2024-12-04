use editor_instance::EditorInstance;
use input::process_keypress;
use output::refresh_screen;
use std::error::Error;
use std::io;
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
    let stdin = io::stdin();

    let termios = get_populated_termios();

    set_panic_hook(termios);
    enable_raw_mode(termios);

    let active_editor = Arc::new(RwLock::new(EditorInstance::new(termios, &mut stdin.lock())));

    watch_for_window_size_change(Arc::clone(&active_editor));

    loop {
        let screen_rows_columns = {
            active_editor
                .read()
                .expect("Could not get reader for editor")
                .screen_rows_columns
        };

        refresh_screen(screen_rows_columns.0);

        process_keypress(
            &mut stdin.lock(),
            &active_editor
                .read()
                .expect("Could not get reader for editor"),
        );
    }
}
