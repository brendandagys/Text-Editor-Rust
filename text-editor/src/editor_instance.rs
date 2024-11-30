use termios::Termios;

use crate::{
    output::clear_display,
    terminal::{disable_raw_mode, enable_raw_mode, get_populated_termios},
};

pub struct EditorInstance {
    stdin_fd: i32,
    original_termios: Termios,
}

fn ctrl_key(k: char) -> u8 {
    (k as u8) & 0x1f // Ctrl key strips bits 5 and 6 from 7-bit ASCII
}

impl EditorInstance {
    pub fn new(stdin_fd: i32) -> Self {
        let original_termios = get_populated_termios(stdin_fd);
        enable_raw_mode(stdin_fd);

        EditorInstance {
            stdin_fd,
            original_termios,
        }
    }

    pub fn process_key(&self, key: u8) -> () {
        match key {
            key if key == ctrl_key('q') => {
                clear_display();
                disable_raw_mode(self.stdin_fd, self.original_termios);

                std::process::exit(0);
            }

            _ => {}
        }
    }
}

impl Drop for EditorInstance {
    fn drop(&mut self) {
        disable_raw_mode(self.stdin_fd, self.original_termios);
    }
}
