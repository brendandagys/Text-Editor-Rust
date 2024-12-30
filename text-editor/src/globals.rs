use once_cell::sync::Lazy;
use std::sync::{Mutex, MutexGuard};

pub const VERSION: &str = "1.0.0";

pub static WELCOME_MESSAGE: Lazy<String> =
    Lazy::new(|| format!("Brendan's text editor --- version {VERSION}"));

pub const TAB_SIZE: u8 = 4;
pub const LINE_NUMBER_GAP: u8 = 3;
pub const QUIT_CONFIRMATION_COUNT: u8 = 1;

pub const DEFAULT_STATUS_BAR_MESSAGE: &str =
    "Ctrl-F: find | Ctrl-G: go to line | Ctrl-S: save | Ctrl-Q: quit";

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
    pub multi_line_comment_start: &'static str,
    pub multi_line_comment_end: &'static str,
    pub flags: i32,
}

pub const HIGHLIGHT_NUMBERS: i32 = 1 << 0;
pub const HIGHLIGHT_STRINGS: i32 = 1 << 1;

pub static SYNTAX_CONFIGURATIONS: &[Syntax] = &[
    Syntax {
        file_type: "C",
        file_match: &[".c", ".h", ".cpp"],
        keywords: &[
            "switch", "if", "while", "for", "break", "continue", "return", "else", "struct",
            "union", "typedef", "static", "enum", "class", "case",
        ],
        types: &[
            "int", "long", "double", "float", "char", "unsigned", "signed", "void",
        ],
        single_line_comment_start: "//",
        multi_line_comment_start: "/*",
        multi_line_comment_end: "*/",
        flags: HIGHLIGHT_NUMBERS | HIGHLIGHT_STRINGS,
    },
    Syntax {
        file_type: "Rust",
        file_match: &[".rs"],
        keywords: &[
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
            "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
            "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe",
            "use", "where", "while",
        ],
        types: &[
            "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "str", "u8",
            "u16", "u32", "u64", "u128", "usize",
        ],
        single_line_comment_start: "//",
        multi_line_comment_start: "/*",
        multi_line_comment_end: "*/",
        flags: HIGHLIGHT_NUMBERS | HIGHLIGHT_STRINGS,
    },
    Syntax {
        file_type: "JavaScript",
        file_match: &[".js", ".ts"],
        keywords: &[
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
            "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
            "return", "self", "static", "struct", "super", "this", "trait", "true", "type",
            "unsafe", "use", "where", "while",
        ],
        types: &[
            "Number",
            "String",
            "Boolean",
            "Object",
            "Array",
            "Function",
            "null",
            "undefined",
        ],
        single_line_comment_start: "//",
        multi_line_comment_start: "/*",
        multi_line_comment_end: "*/",
        flags: HIGHLIGHT_NUMBERS | HIGHLIGHT_STRINGS,
    },
    Syntax {
        file_type: "Python",
        file_match: &[".py"],
        keywords: &[
            "and", "as", "assert", "break", "class", "continue", "def", "del", "elif", "else",
            "except", "False", "finally", "for", "from", "global", "if", "import", "in", "is",
            "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "True", "try",
            "while", "with", "yield",
        ],
        types: &[
            "bool",
            "bytearray",
            "bytes",
            "complex",
            "dict",
            "float",
            "frozenset",
            "int",
            "list",
            "object",
            "range",
            "set",
            "slice",
            "str",
            "tuple",
            "type",
        ],
        single_line_comment_start: "#",
        multi_line_comment_start: "\"\"\"",
        multi_line_comment_end: "\"\"\"",
        flags: HIGHLIGHT_NUMBERS | HIGHLIGHT_STRINGS,
    },
];
