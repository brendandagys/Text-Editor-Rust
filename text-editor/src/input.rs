use crate::{editor_instance::EditorInstance, globals::get_buffer_lock};
use std::io::{self, Read};

#[derive(PartialEq)]
pub enum EditorKey {
    ArrowUp = 1000,
    ArrowDown,
    ArrowRight,
    ArrowLeft,
    PageUp,
    PageDown,
}

#[derive(PartialEq)]
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
        Some(key) => match key {
            b'\x1b' => {
                let first = match read_single_key() {
                    Some(key) => key,
                    None => return Some(esc),
                };

                let second = match read_single_key() {
                    Some(key) => key,
                    None => return Some(esc),
                };

                match first {
                    b'[' => match second {
                        b'0'..=b'9' => {
                            let third = match read_single_key() {
                                Some(key) => key,
                                None => return Some(esc),
                            };

                            match third {
                                b'~' => match second {
                                    b'5' => Some(Key::Custom(EditorKey::PageUp)),
                                    b'6' => Some(Key::Custom(EditorKey::PageDown)),
                                    _ => Some(esc),
                                },
                                _ => Some(esc),
                            }
                        }
                        _ => match second {
                            b'A' => Some(Key::Custom(EditorKey::ArrowUp)),
                            b'B' => Some(Key::Custom(EditorKey::ArrowDown)),
                            b'C' => Some(Key::Custom(EditorKey::ArrowRight)),
                            b'D' => Some(Key::Custom(EditorKey::ArrowLeft)),
                            _ => Some(esc),
                        },
                    },
                    _ => return Some(esc),
                }
            }
            _ => Some(Key::U8(key)),
        },
        None => None,
    }
}

pub fn process_keypress(editor: &mut EditorInstance) -> () {
    if let Some(key) = read_key_input() {
        editor.process_key(key);
    }
}
