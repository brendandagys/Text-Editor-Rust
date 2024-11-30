use crate::editor_instance::EditorInstance;
use crate::utils::panic_with_error;
use std::io::{Read, StdinLock};
use std::sync::Mutex;

static BUFFER: Mutex<[u8; 1]> = Mutex::new([0u8; 1]);

fn read_key(stdin_lock: &mut StdinLock) -> Option<u8> {
    let mut buffer = match BUFFER.lock() {
        Ok(buffer) => buffer,
        Err(e) => panic_with_error(e, "Error locking input buffer"),
    };

    match stdin_lock.read_exact(&mut *buffer) {
        Ok(_) => Some(buffer[0]),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
        Err(e) => panic_with_error(e, "Error reading byte into buffer"),
    }
}

pub fn process_keypress(stdin_lock: &mut StdinLock, editor: &EditorInstance) -> () {
    if let Some(key) = read_key(stdin_lock) {
        editor.process_key(key);
    }
}
