use std::error::Error;
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;

use editor_instance::EditorInstance;
use utils::{debug_input, panic_with_error, set_panic_hook};

mod editor_instance;
mod terminal;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let stdin_fd = stdin.as_raw_fd();
    let mut stdin_lock = stdin.lock();

    set_panic_hook(stdin_fd);

    let active_editor = EditorInstance::new(stdin_fd);
    let mut byte_buffer = [0u8; 1];

    loop {
        match stdin_lock.read_exact(&mut byte_buffer) {
            Ok(_) => {
                let key = byte_buffer[0];
                debug_input(key);

                active_editor.process_key(key);
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {}
            Err(e) => panic_with_error(e, "Error reading byte into buffer"),
        };
    }
}
