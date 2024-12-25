use std::{io, os::fd::AsRawFd};

use termios::{
    tcsetattr, Termios, BRKINT, CS8, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG, ISTRIP, IXON, OPOST,
    TCSAFLUSH, VMIN, VTIME,
};

pub fn get_populated_termios() -> Termios {
    Termios::from_fd(io::stdin().as_raw_fd()).expect("Failed to get current terminal configuration")
}

pub fn disable_raw_mode(original_termios: Termios) -> () {
    let stdin_fd = io::stdin().as_raw_fd();

    tcsetattr(stdin_fd, TCSAFLUSH, &original_termios)
        .expect("Failed to reset terminal settings (disable raw mode)")
}

pub fn enable_raw_mode(mut termios: Termios) -> Termios {
    let stdin_fd = io::stdin().as_raw_fd();

    // `c_lflag`: LOCAL/MISCELLANEOUS FLAGS
    termios.c_lflag &= !(
        ECHO    // Disable echoing keys to terminal
      | ICANON  // Disable Canonical mode (press Enter to submit)
      | IEXTEN  // Fix for possible Ctrl-V wait-for-next-character issue
      | ISIG
        // `SIGINT` (Ctrl-C) AND `SIGTSTP` (Ctrl-Z) which terminate and suspend, respectively
    );

    // `c_iflag`: INPUT FLAGS
    termios.c_iflag &= !(
        IXON // No software flow control (pause/resume transmission) [Ctrl-S and Ctrl-Q]
      | BRKINT // Disable break condition causing a `SIGINT` (e.g. Ctrl-C)
      | INPCK // Disable input parity checking
      | ISTRIP // Strip 8th bit of each input byte (set to 0)
      | ICRNL
        // Carriage return (13) -> new line (affects Ctrl-M [13])
    );

    // `c_oflag`: OUTPUT FLAGS
    termios.c_oflag &= !(
        // Disable output post-processing: "\n" -> "\r\n"
        OPOST
    );

    // `c_cflag`: CONTROL FLAGS
    termios.c_cflag |= CS8; // Set character size to 8 bits/byte

    // Control characters (array of bytes controlling various terminal settings)
    termios.c_cc[VMIN] = 0; // Minimum bytes needed before `read()` returns
    termios.c_cc[VTIME] = 1; // Time-out (1/10 second)

    tcsetattr(stdin_fd, TCSAFLUSH, &termios)
        .expect("Failed to set terminal settings (enable raw mode)");

    termios
}
