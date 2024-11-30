use termios::Termios;

use crate::terminal::{disable_raw_mode, enable_raw_mode, get_populated_termios};

pub struct EditorInstance {
    stdin_fd: i32,
    original_termios: Termios,
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
            b'q' => {
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
