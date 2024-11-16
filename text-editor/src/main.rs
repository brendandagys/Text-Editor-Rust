use std::io::{self, Read};
use std::os::unix::io::AsRawFd;

use cleanup::CleanupTask;
use terminal::{disable_raw_mode, enable_raw_mode, get_populated_termios};

mod cleanup;
mod terminal;

fn main() -> Result<(), io::Error> {
    let stdin_fd = io::stdin().as_raw_fd();

    let original_termios = get_populated_termios(stdin_fd)?;

    let _cleanup_restore_termios =
        CleanupTask::new(move || disable_raw_mode(stdin_fd, original_termios).unwrap());

    enable_raw_mode(stdin_fd)?;

    io::stdin()
        .lock()
        .bytes()
        .for_each(|read_result| match read_result {
            Ok(key) => {
                if key.is_ascii_control() {
                    println!("{}", key as u32);
                } else {
                    println!("{} ('{}')", key as char, key)
                }

                match key {
                    b'q' => std::process::exit(0),
                    _ => {}
                }
            }
            Err(e) => {
                println!("Error reading byte: {:?}", e)
            }
        });

    Ok(())
}
