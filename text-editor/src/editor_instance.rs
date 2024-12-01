use termios::Termios;

use crate::{output::clear_display, terminal::disable_raw_mode};

pub struct EditorInstance {
    original_termios: Termios,
}

fn ctrl_key(k: char) -> u8 {
    (k as u8) & 0x1f // Ctrl key strips bits 5 and 6 from 7-bit ASCII
}

impl EditorInstance {
    pub fn new(original_termios: Termios) -> Self {
        EditorInstance { original_termios }
    }

    pub fn process_key(&self, key: u8) -> () {
        match key {
            b'p' => panic!("Manual panic!"),
            key if key == ctrl_key('q') => {
                clear_display();
                disable_raw_mode(self.original_termios);

                std::process::exit(0);
            }

            _ => {}
        }
    }
}
