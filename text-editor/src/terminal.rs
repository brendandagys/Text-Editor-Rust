use termios::{
    tcgetattr, tcsetattr, Termios, BRKINT, CS8, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG, ISTRIP,
    IXON, OPOST, TCSAFLUSH, VMIN, VTIME,
};

use crate::utils::panic_with_error;

pub fn get_populated_termios(stdin_fd: i32) -> Termios {
    let mut termios = match Termios::from_fd(stdin_fd) {
        Ok(termios) => termios,
        Err(e) => panic_with_error(e, "Unable to get current terminal configuration"),
    };

    match tcgetattr(stdin_fd, &mut termios) {
        Ok(_) => termios,
        Err(e) => panic_with_error(
            e,
            "Unable to populate new `termios` object with current terminal configuration",
        ),
    }
}

pub fn disable_raw_mode(stdin_fd: i32, original_termios: Termios) -> () {
    match tcsetattr(stdin_fd, TCSAFLUSH, &original_termios) {
        Ok(_) => (),
        Err(e) => panic_with_error(e, "Unable to reset terminal settings (disable raw mode)"),
    }
}

pub fn enable_raw_mode(stdin_fd: i32) -> () {
    let mut raw_termios = get_populated_termios(stdin_fd);

    // `c_lflag`: LOCAL/MISCELLANEOUS FLAGS
    raw_termios.c_lflag &= !(
        ECHO    // Disable echoing keys to terminal
      | ICANON  // Disable Canonical mode (press Enter to submit)
      | IEXTEN  // Fix for possible Ctrl-V wait-for-next-character issue
      | ISIG
        // `SIGINT` (Ctrl-C) AND `SIGTSTP` (Ctrl-Z) which terminate and suspend, respectively
    );

    // `c_iflag`: INPUT FLAGS

    raw_termios.c_iflag &= !(
        IXON // No software flow control (pause/resume transmission) [Ctrl-S and Ctrl-Q]
      | BRKINT // Disable break condition causing a `SIGINT` (e.g. Ctrl-C)
      | INPCK // Disable input parity checking
      | ISTRIP // Strip 8th bit of each input byte (set to 0)
      | ICRNL
        // Carriage return (13) -> new line (affects Ctrl-M [13])
    );

    // `c_oflag`: OUTPUT FLAGS
    raw_termios.c_oflag &= !(
        // Disable output post-processing: "\n" -> "\r\n"
        OPOST
    );

    // `c_cflag`: CONTROL FLAGS
    raw_termios.c_cflag |= CS8; // Set character size to 8 bits/byte

    // Control characters (array of bytes controlling various terminal settings)
    raw_termios.c_cc[VMIN] = 0; // Minimum bytes needed before `read()` returns
    raw_termios.c_cc[VTIME] = 1; // Time-out (1/10 second)

    match tcsetattr(stdin_fd, TCSAFLUSH, &raw_termios) {
        Ok(_) => (),
        Err(e) => panic_with_error(e, "Unable to set terminal settings (enable raw mode)"),
    }
}
