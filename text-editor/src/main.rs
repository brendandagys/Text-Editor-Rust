use editor_instance::EditorInstance;
use globals::DEFAULT_STATUS_BAR_MESSAGE;
use input::process_keypress;
use output::{clear_display, refresh_screen};
use std::error::Error;
use std::sync::{Arc, RwLock};
use terminal::{enable_raw_mode, get_populated_termios};
use utils::{get_window_size, set_panic_hook, watch_for_window_size_change};

mod editor_instance;
mod globals;
mod input;
mod output;
mod terminal;
mod utils;

#[derive(Clone, Copy, PartialEq)]
pub struct WindowSize {
    pub rows: u32,
    pub columns: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    clear_display(); // Does not rely on raw mode

    let termios = get_populated_termios();
    set_panic_hook(termios);
    enable_raw_mode(termios);

    let window_size = Arc::new(RwLock::new(get_window_size()));
    watch_for_window_size_change(Arc::clone(&window_size));

    let mut active_editor = EditorInstance::new(termios);

    let args = std::env::args();
    if args.len() > 1 {
        active_editor.open(&args.collect::<Vec<String>>()[1]);
    }

    active_editor.set_status_message(DEFAULT_STATUS_BAR_MESSAGE, false);

    loop {
        let window_size = *window_size
            .read()
            .expect("Failed to obtain window size read lock");

        if window_size != active_editor.window_size {
            active_editor.window_size = window_size;
        }

        refresh_screen(&mut active_editor);

        process_keypress(&mut active_editor);
    }
}
