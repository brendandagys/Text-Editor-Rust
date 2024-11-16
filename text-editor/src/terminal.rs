use std::io;
use termios::{tcgetattr, tcsetattr, Termios, ECHO, ICANON, TCSAFLUSH};

pub fn get_populated_termios(stdin_fd: i32) -> io::Result<Termios> {
    let mut termios = Termios::from_fd(stdin_fd)?;
    tcgetattr(stdin_fd, &mut termios)?;
    Ok(termios)
}

pub fn disable_raw_mode(stdin_fd: i32, original_termios: Termios) -> io::Result<()> {
    tcsetattr(stdin_fd, TCSAFLUSH, &original_termios)
}

pub fn enable_raw_mode(stdin_fd: i32) -> io::Result<()> {
    let mut raw_termios = get_populated_termios(stdin_fd)?;

    // `c_lflag`: local/miscellaneous flags
    raw_termios.c_lflag &= !(ECHO | ICANON);

    // `c_iflag`: Input flags

    // `c_oflag`: Output flags

    // `c_cflag`: Control flags

    tcsetattr(stdin_fd, TCSAFLUSH, &raw_termios)?;

    Ok(())
}
