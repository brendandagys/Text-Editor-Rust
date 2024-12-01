use std::error::Error;
use std::io;

use editor_instance::EditorInstance;
use input::process_keypress;
use output::refresh_screen;
use terminal::{enable_raw_mode, get_populated_termios};
use utils::set_panic_hook;

mod editor_instance;
mod input;
mod output;
mod terminal;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    let mut stdin_lock = io::stdin().lock();

    let termios = get_populated_termios();

    set_panic_hook(termios);
    enable_raw_mode(termios);

    let active_editor = EditorInstance::new(termios);

    loop {
        refresh_screen();
        process_keypress(&mut stdin_lock, &active_editor);
    }
}
