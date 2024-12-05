use crate::{editor_instance::EditorInstance, globals::get_buffer_lock};
use std::io::{self, Read};

fn read_key() -> Option<u8> {
    let mut buffer = *get_buffer_lock();

    match &mut io::stdin().lock().read_exact(&mut buffer) {
        Ok(_) => Some(buffer[0]),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
        Err(e) => panic!("Error reading byte into buffer: {:?}", e),
    }
}

pub fn process_keypress(editor: &mut EditorInstance) -> () {
    if let Some(key) = read_key() {
        editor.process_key(key);
    }
}
