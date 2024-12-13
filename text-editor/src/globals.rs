use std::sync::{Mutex, MutexGuard};

pub const VERSION: &str = "0.0.1";
pub const TAB_SIZE: u8 = 8;
pub const QUIT_CONFIRMATION_COUNT: u8 = 1;

static BUFFER: Mutex<[u8; 1]> = Mutex::new([0u8; 1]);

pub fn get_buffer_lock() -> MutexGuard<'static, [u8; 1]> {
    match BUFFER.lock() {
        Ok(buffer) => buffer,
        Err(poisoned) => poisoned.into_inner(),
    }
}
