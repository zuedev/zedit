//! Simple benchmarks for zedit (no external dependencies)
//!
//! Run with: cargo run --release --bin benchmarks

use std::time::{Duration, Instant};
use std::path::PathBuf;

/// Simple benchmark result
struct BenchResult {
    name: String,
    iterations: u64,
    total_time: Duration,
}

impl BenchResult {
    fn avg_ns(&self) -> u64 {
        self.total_time.as_nanos() as u64 / self.iterations
    }

    fn ops_per_sec(&self) -> f64 {
        self.iterations as f64 / self.total_time.as_secs_f64()
    }
}

/// Run a benchmark function
fn bench<F>(name: &str, iterations: u64, mut f: F) -> BenchResult
where
    F: FnMut(),
{
    // Warmup
    for _ in 0..100 {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let total_time = start.elapsed();

    BenchResult {
        name: name.to_string(),
        iterations,
        total_time,
    }
}

fn print_result(result: &BenchResult) {
    println!(
        "{:40} {:>10} iterations {:>12} ns/iter {:>15.2} ops/sec",
        result.name,
        result.iterations,
        result.avg_ns(),
        result.ops_per_sec()
    );
}

// ============================================================================
// Minimal reimplementations for benchmarking (to avoid module dependency issues)
// ============================================================================

/// Minimal Line implementation for benchmarking
#[derive(Clone)]
struct Line {
    chars: Vec<char>,
}

impl Line {
    fn new() -> Self {
        Line { chars: Vec::new() }
    }

    fn from_str(s: &str) -> Self {
        Line {
            chars: s.chars().collect(),
        }
    }

    fn len(&self) -> usize {
        self.chars.len()
    }

    fn to_string(&self) -> String {
        self.chars.iter().collect()
    }

    fn insert(&mut self, idx: usize, c: char) {
        if idx <= self.chars.len() {
            self.chars.insert(idx, c);
        }
    }

    fn delete(&mut self, idx: usize) -> Option<char> {
        if idx < self.chars.len() {
            Some(self.chars.remove(idx))
        } else {
            None
        }
    }

    fn split_off(&mut self, idx: usize) -> Line {
        if idx >= self.chars.len() {
            Line::new()
        } else {
            let rest = self.chars.split_off(idx);
            Line { chars: rest }
        }
    }

    fn append(&mut self, other: &Line) {
        self.chars.extend(other.chars.iter().cloned());
    }
}

/// Minimal Buffer implementation for benchmarking
struct Buffer {
    lines: Vec<Line>,
    modified: bool,
}

impl Buffer {
    fn new() -> Self {
        Buffer {
            lines: vec![Line::new()],
            modified: false,
        }
    }

    fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn insert_char(&mut self, row: usize, col: usize, c: char) {
        if row < self.lines.len() {
            self.lines[row].insert(col, c);
            self.modified = true;
        }
    }

    fn insert_newline(&mut self, row: usize, col: usize) {
        if row < self.lines.len() {
            let new_line = self.lines[row].split_off(col);
            self.lines.insert(row + 1, new_line);
            self.modified = true;
        }
    }
}

/// Minimal syntax highlighting types for benchmarking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    Normal,
    Keyword,
    String,
    Number,
    Comment,
}

#[derive(Clone)]
struct Token {
    text: String,
    token_type: TokenType,
}

#[derive(Clone, Default)]
struct HighlightState {
    in_comment: bool,
}

struct Highlighter {
    keywords: Vec<&'static str>,
}

impl Highlighter {
    fn new_rust() -> Self {
        Highlighter {
            keywords: vec![
                "fn", "let", "mut", "if", "else", "while", "for", "loop",
                "match", "return", "struct", "enum", "impl", "trait", "pub",
                "use", "mod", "const", "static", "type", "where", "async",
                "await", "move", "ref", "self", "Self", "super", "crate",
            ],
        }
    }

    fn highlight_line(&self, line: &str, _state: &mut HighlightState) -> Vec<Token> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Comment
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                let text: String = chars[i..].iter().collect();
                tokens.push(Token {
                    text,
                    token_type: TokenType::Comment,
                });
                break;
            }

            // String
            if chars[i] == '"' {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < chars.len() {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                tokens.push(Token {
                    text,
                    token_type: TokenType::String,
                });
                continue;
            }

            // Number
            if chars[i].is_ascii_digit() {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                tokens.push(Token {
                    text,
                    token_type: TokenType::Number,
                });
                continue;
            }

            // Identifier/keyword
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let token_type = if self.keywords.contains(&text.as_str()) {
                    TokenType::Keyword
                } else {
                    TokenType::Normal
                };
                tokens.push(Token { text, token_type });
                continue;
            }

            // Other
            tokens.push(Token {
                text: chars[i].to_string(),
                token_type: TokenType::Normal,
            });
            i += 1;
        }

        tokens
    }
}

/// Entry type for browser benchmarking
#[derive(Clone, PartialEq, Eq)]
enum EntryType {
    Directory,
    File,
}

#[derive(Clone)]
struct Entry {
    name: String,
    path: PathBuf,
    entry_type: EntryType,
    size: Option<u64>,
}

impl Entry {
    fn display_name(&self) -> String {
        if self.entry_type == EntryType::Directory {
            format!("{}/", self.name)
        } else {
            self.name.clone()
        }
    }

    fn size_string(&self) -> String {
        match self.size {
            Some(size) => {
                if size < 1024 {
                    format!("{} B", size)
                } else if size < 1024 * 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                }
            }
            None => String::new(),
        }
    }
}

/// ANSI helpers for benchmarking
mod ansi {
    pub fn cursor_position(row: u16, col: u16) -> String {
        format!("\x1b[{};{}H", row + 1, col + 1)
    }

    pub fn fg_rgb(r: u8, g: u8, b: u8) -> String {
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    }

    pub fn bg_rgb(r: u8, g: u8, b: u8) -> String {
        format!("\x1b[48;2;{};{};{}m", r, g, b)
    }
}

// ============================================================================
// Main benchmark runner
// ============================================================================

fn main() {
    println!("zedit benchmarks");
    println!("{}", "=".repeat(90));
    println!();

    // Line operations benchmarks
    println!("Line Operations:");
    println!("{}", "-".repeat(90));

    let result = bench("Line::new()", 1_000_000, || {
        let _ = Line::new();
    });
    print_result(&result);

    let result = bench("Line::from_str(short)", 1_000_000, || {
        let _ = Line::from_str("Hello, World!");
    });
    print_result(&result);

    let result = bench("Line::from_str(long)", 100_000, || {
        let _ = Line::from_str(&"x".repeat(1000));
    });
    print_result(&result);

    let result = bench("Line::insert(middle)", 100_000, || {
        let mut line = Line::from_str("Hello World");
        line.insert(5, 'X');
    });
    print_result(&result);

    let result = bench("Line::delete(middle)", 100_000, || {
        let mut line = Line::from_str("Hello World");
        let _ = line.delete(5);
    });
    print_result(&result);

    let result = bench("Line::split_off(middle)", 100_000, || {
        let mut line = Line::from_str("Hello World");
        let _ = line.split_off(5);
    });
    print_result(&result);

    let result = bench("Line::append", 100_000, || {
        let mut line1 = Line::from_str("Hello ");
        let line2 = Line::from_str("World");
        line1.append(&line2);
    });
    print_result(&result);

    let result = bench("Line::to_string", 1_000_000, || {
        let line = Line::from_str("Hello, World!");
        let _ = line.to_string();
    });
    print_result(&result);

    println!();

    // Buffer operations benchmarks
    println!("Buffer Operations:");
    println!("{}", "-".repeat(90));

    let result = bench("Buffer::new()", 1_000_000, || {
        let _ = Buffer::new();
    });
    print_result(&result);

    let result = bench("Buffer::insert_char", 100_000, || {
        let mut buffer = Buffer::new();
        buffer.insert_char(0, 0, 'X');
    });
    print_result(&result);

    let result = bench("Buffer::insert_newline", 100_000, || {
        let mut buffer = Buffer::new();
        buffer.lines[0] = Line::from_str("Hello World");
        buffer.insert_newline(0, 5);
    });
    print_result(&result);

    let result = bench("Buffer::line_count (100 lines)", 1_000_000, || {
        let mut buffer = Buffer::new();
        buffer.lines = (0..100).map(|_| Line::from_str("test")).collect();
        let _ = buffer.line_count();
    });
    print_result(&result);

    println!();

    // Syntax highlighting benchmarks
    println!("Syntax Highlighting:");
    println!("{}", "-".repeat(90));

    let result = bench("Highlighter::new_rust()", 1_000_000, || {
        let _ = Highlighter::new_rust();
    });
    print_result(&result);

    let highlighter = Highlighter::new_rust();
    let result = bench("highlight_line(simple)", 100_000, || {
        let mut state = HighlightState::default();
        let _ = highlighter.highlight_line("let x = 42;", &mut state);
    });
    print_result(&result);

    let highlighter = Highlighter::new_rust();
    let result = bench("highlight_line(complex)", 50_000, || {
        let mut state = HighlightState::default();
        let _ = highlighter.highlight_line(
            "fn main() { let s = \"hello\"; println!(\"{}\", s); }",
            &mut state,
        );
    });
    print_result(&result);

    let highlighter = Highlighter::new_rust();
    let long_line = format!("let x = {};", "1 + ".repeat(100));
    let result = bench("highlight_line(long)", 10_000, || {
        let mut state = HighlightState::default();
        let _ = highlighter.highlight_line(&long_line, &mut state);
    });
    print_result(&result);

    println!();

    // Entry operations benchmarks
    println!("Entry Operations:");
    println!("{}", "-".repeat(90));

    let result = bench("Entry::display_name (dir)", 1_000_000, || {
        let entry = Entry {
            name: "folder".to_string(),
            path: PathBuf::from("/folder"),
            entry_type: EntryType::Directory,
            size: None,
        };
        let _ = entry.display_name();
    });
    print_result(&result);

    let result = bench("Entry::size_string (KB)", 1_000_000, || {
        let entry = Entry {
            name: "file.txt".to_string(),
            path: PathBuf::from("/file.txt"),
            entry_type: EntryType::File,
            size: Some(2048),
        };
        let _ = entry.size_string();
    });
    print_result(&result);

    println!();

    // ANSI escape code benchmarks
    println!("ANSI Escape Codes:");
    println!("{}", "-".repeat(90));

    let result = bench("cursor_position", 1_000_000, || {
        let _ = ansi::cursor_position(10, 20);
    });
    print_result(&result);

    let result = bench("fg_rgb", 1_000_000, || {
        let _ = ansi::fg_rgb(255, 128, 64);
    });
    print_result(&result);

    let result = bench("bg_rgb", 1_000_000, || {
        let _ = ansi::bg_rgb(64, 128, 255);
    });
    print_result(&result);

    println!();

    // String operations benchmarks
    println!("String Operations:");
    println!("{}", "-".repeat(90));

    let result = bench("String allocation (short)", 1_000_000, || {
        let _ = String::from("Hello, World!");
    });
    print_result(&result);

    let result = bench("String allocation (1KB)", 100_000, || {
        let _ = "x".repeat(1024);
    });
    print_result(&result);

    let result = bench("Vec<char> from str (short)", 1_000_000, || {
        let _: Vec<char> = "Hello, World!".chars().collect();
    });
    print_result(&result);

    let result = bench("Vec<char> from str (1KB)", 100_000, || {
        let s = "x".repeat(1024);
        let _: Vec<char> = s.chars().collect();
    });
    print_result(&result);

    println!();
    println!("{}", "=".repeat(90));
    println!("Benchmarks complete.");
}
