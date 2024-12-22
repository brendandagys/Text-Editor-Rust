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

pub struct Syntax {
    pub file_type: &'static str,
    pub file_match: &'static [&'static str],
    pub keywords: &'static [&'static str],
    pub types: &'static [&'static str],
    pub single_line_comment_start: &'static str,
    pub flags: i32,
}

pub const HIGHLIGHT_NUMBERS: i32 = 1 << 0;
pub const HIGHLIGHT_STRINGS: i32 = 1 << 1;

pub static SYNTAX_CONFIGURATIONS: &'static [Syntax] = &[Syntax {
    file_type: "C",
    file_match: &[".c", ".h", ".cpp"],
    keywords: &[
        "switch", "if", "while", "for", "break", "continue", "return", "else", "struct", "union",
        "typedef", "static", "enum", "class", "case",
    ],
    types: &[
        "int", "long", "double", "float", "char", "unsigned", "signed", "void",
    ],
    single_line_comment_start: "//",
    flags: HIGHLIGHT_NUMBERS | HIGHLIGHT_STRINGS,
}];
