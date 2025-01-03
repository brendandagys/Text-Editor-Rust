# Rust Text Editor

A lightweight and fast text editor built with Rust. Designed for ease-of-use, efficiency, safety, and future extensibility (more features and keybindings), this project aims to deliver a modern text editing tool.

## Features

- **Create new files or edit existing ones**: Use command-line arguments to open an existing file, or start from scratch.
- **Syntax Highlighting**: Supports highlighting for popular programming languages (Rust, C, JavaScript, Python).
- **Search**: Efficient text searching and navigation, with visual cues.
- **Line numbers**: Always know your location in the file.
- **Go to line**: Navigate to a specific line number with a few key-presses.
- **Status bar**: Always have access to the current file name, line count, current line, and a help menu. Also accepts user prompts for relevant features.
- **Cross-Platform**: Runs on Unix-based systems.
- **Vim Keybindings**: Supports basic Vim keybindings with a Normal and Insert mode.

## Installation

### Prerequisites
- [Rust](https://www.rust-lang.org/) (ensure you have the latest stable version installed).

### Steps
1. Clone the repository:
   ```bash
   git clone https://github.com/brendandagys/Text-Editor-Rust.git
   cd text-editor
   ```
2. Build the project:
   ```bash
   cargo build --release
   ```
3. Run the text editor:
   ```bash
   ./target/release/text-editor
   ```

Or simply use the provided binary at the repository root.

## Usage

### Basic Commands
- Open a file: `text-editor <filename>`
- Save file: `Ctrl+S`
- Quit: `Ctrl+Q`
- Search: `Ctrl+F`
- Go to line: `Ctrl+G`

## Contributing

Feel free to submit a pull request or suggest feature additions. I will likely extend this project in the future!


## Acknowledgments

- Inspired by the Vim text editor.
- Thank you to those who inspire me to try new things and step outside of my comfort zone.

