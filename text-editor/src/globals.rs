use std::sync::{Mutex, MutexGuard};

static BUFFER: Mutex<[u8; 1]> = Mutex::new([0u8; 1]);

pub fn get_buffer_lock() -> MutexGuard<'static, [u8; 1]> {
    match BUFFER.lock() {
        Ok(buffer) => buffer,
        Err(poisoned) => poisoned.into_inner(),
    }
}