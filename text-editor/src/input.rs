use crate::{editor_instance::EditorInstance, globals::get_buffer_lock};
use std::io::{self, Read};

pub enum EditorKey {
    ArrowUp = 1000,
    ArrowDown,
    ArrowRight,
    ArrowLeft,
}

pub enum Key {
    U8(u8),
    Custom(EditorKey),
}

fn read_single_key() -> Option<u8> {
    let mut buffer = *get_buffer_lock();

    match &mut io::stdin().lock().read_exact(&mut buffer) {
        Ok(_) => Some(buffer[0]),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
        Err(e) => panic!("Error reading byte into buffer: {:?}", e),
    }
}

fn read_key_input() -> Option<Key> {
    let esc = Key::U8(b'\x1b');

    match read_single_key() {
        Some(key1) => match key1 {
            b'\x1b' => match read_single_key() {
                Some(key2) => match key2 {
                    b'[' => match read_single_key() {
                        Some(key3) => match key3 {
                            b'A' => Some(Key::Custom(EditorKey::ArrowUp)),
                            b'B' => Some(Key::Custom(EditorKey::ArrowDown)),
                            b'C' => Some(Key::Custom(EditorKey::ArrowRight)),
                            b'D' => Some(Key::Custom(EditorKey::ArrowLeft)),
                            _ => Some(esc),
                        },
                        None => Some(esc),
                    },
                    _ => Some(esc),
                },
                None => Some(esc),
            },
            _ => Some(Key::U8(key1)),
        },
        None => None,
    }
}

pub fn process_keypress(editor: &mut EditorInstance) -> () {
    if let Some(key) = read_key_input() {
        editor.process_key(key);
    }
}
