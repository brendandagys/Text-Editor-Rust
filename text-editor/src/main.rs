use std::error::Error;
use std::io;
use std::os::unix::io::AsRawFd;

use editor_instance::EditorInstance;
use input::process_keypress;
use output::refresh_screen;
use utils::set_panic_hook;

mod editor_instance;
mod input;
mod output;
mod terminal;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let stdin_fd = stdin.as_raw_fd();
    let mut stdin_lock = stdin.lock();

    set_panic_hook(stdin_fd);

    let active_editor = EditorInstance::new(stdin_fd);

    loop {
        refresh_screen();
        process_keypress(&mut stdin_lock, &active_editor);
    }
}
