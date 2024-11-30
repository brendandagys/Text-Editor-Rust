use std::io::{self, Write};

use crate::utils::panic_with_error;

pub fn refresh_screen() {
    // Escape sequences begin with escape characters `\x1b` (27) and '['
    // Escape sequence commands take arguments that come before the command itself
    // Arguments are separated by a ';'
    // https://vt100.net/docs/vt100-ug/chapter3.html

    // J: Erase in Display, 2: clear entire screen
    write!(io::stdout(), "\x1b[2J")
        .unwrap_or_else(|e| panic_with_error(e, "Error clearing screen"));

    // H: Cursor Position, e.g. <esc>[1;1H]
    write!(io::stdout(), "\x1b[H").unwrap_or_else(|e| {
        panic_with_error(e, "Error positioning cursor at top-left before draw")
    });

    io::stdout()
        .flush()
        .unwrap_or_else(|e| panic_with_error(e, "Error flushing stdout"));
}
