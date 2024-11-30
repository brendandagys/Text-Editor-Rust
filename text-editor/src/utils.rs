use std::error::Error;
use std::panic;

use crate::{
    output::clear_display,
    terminal::{disable_raw_mode, get_populated_termios},
};

#[allow(dead_code)]
pub fn debug_input(key: u8) {
    if key.is_ascii_control() {
        println!("{}\r", key);
    } else {
        println!("{} ('{}')\r", key as char, key)
    }
}

pub fn panic_with_error(e: impl Error, message: &str) -> ! {
    panic!("{message} | {:?}", e)
}

pub fn set_panic_hook(stdin_fd: i32) -> () {
    let original_termios = get_populated_termios(stdin_fd);

    let default_panic_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        clear_display();
        disable_raw_mode(stdin_fd, original_termios);

        default_panic_hook(info);
    }));
}
