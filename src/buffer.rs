use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

/// A single line in the buffer
#[derive(Clone, Debug)]
pub struct Line {
    pub chars: Vec<char>,
}

impl Line {
    pub fn new() -> Self {
        Line { chars: Vec::new() }
    }

    pub fn from_str(s: &str) -> Self {
        Line {
            chars: s.chars().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn to_string(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn insert(&mut self, idx: usize, c: char) {
        if idx <= self.chars.len() {
            self.chars.insert(idx, c);
        }
    }

    pub fn delete(&mut self, idx: usize) -> Option<char> {
        if idx < self.chars.len() {
            Some(self.chars.remove(idx))
        } else {
            None
        }
    }

    pub fn split_off(&mut self, idx: usize) -> Line {
        if idx >= self.chars.len() {
            Line::new()
        } else {
            let rest = self.chars.split_off(idx);
            Line { chars: rest }
        }
    }

    pub fn append(&mut self, other: &Line) {
        self.chars.extend(other.chars.iter().cloned());
    }
}

/// Text buffer containing all lines
pub struct Buffer {
    pub lines: Vec<Line>,
    pub path: Option<PathBuf>,
    pub modified: bool,
    pub readonly: bool,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            lines: vec![Line::new()],
            path: None,
            modified: false,
            readonly: false,
        }
    }

    pub fn from_file(path: &PathBuf) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = Vec::new();

        for line_result in reader.lines() {
            let line = line_result?;
            lines.push(Line::from_str(&line));
        }

        if lines.is_empty() {
            lines.push(Line::new());
        }

        let readonly = fs::metadata(path)
            .map(|m| m.permissions().readonly())
            .unwrap_or(false);

        Ok(Buffer {
            lines,
            path: Some(path.clone()),
            modified: false,
            readonly,
        })
    }

    pub fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.path {
            let mut file = fs::File::create(path)?;
            for (i, line) in self.lines.iter().enumerate() {
                if i > 0 {
                    writeln!(file)?;
                }
                write!(file, "{}", line.to_string())?;
            }
            // Add final newline
            writeln!(file)?;
            self.modified = false;
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "No file path set"))
        }
    }

    pub fn save_as(&mut self, path: PathBuf) -> io::Result<()> {
        self.path = Some(path);
        self.save()
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, idx: usize) -> Option<&Line> {
        self.lines.get(idx)
    }

    pub fn line_mut(&mut self, idx: usize) -> Option<&mut Line> {
        self.lines.get_mut(idx)
    }

    /// Insert a character at the given position
    pub fn insert_char(&mut self, row: usize, col: usize, c: char) {
        if row < self.lines.len() {
            self.lines[row].insert(col, c);
            self.modified = true;
        }
    }

    /// Delete a character at the given position
    pub fn delete_char(&mut self, row: usize, col: usize) -> Option<char> {
        if row < self.lines.len() {
            let result = self.lines[row].delete(col);
            if result.is_some() {
                self.modified = true;
            }
            result
        } else {
            None
        }
    }

    /// Insert a new line (split current line at position)
    pub fn insert_newline(&mut self, row: usize, col: usize) {
        if row < self.lines.len() {
            let new_line = self.lines[row].split_off(col);
            self.lines.insert(row + 1, new_line);
            self.modified = true;
        }
    }

    /// Delete a line and merge with previous
    pub fn delete_line(&mut self, row: usize) {
        if row > 0 && row < self.lines.len() {
            let line = self.lines.remove(row);
            self.lines[row - 1].append(&line);
            self.modified = true;
        }
    }

    /// Insert an empty line at the given position
    pub fn insert_empty_line(&mut self, row: usize) {
        if row <= self.lines.len() {
            self.lines.insert(row, Line::new());
            self.modified = true;
        }
    }

    /// Get the filename (if any)
    pub fn filename(&self) -> Option<String> {
        self.path.as_ref().and_then(|p| {
            p.file_name()
                .map(|s| s.to_string_lossy().to_string())
        })
    }

    /// Get the file extension (if any)
    pub fn extension(&self) -> Option<String> {
        self.path.as_ref().and_then(|p| {
            p.extension()
                .map(|s| s.to_string_lossy().to_string().to_lowercase())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Line tests
    #[test]
    fn test_line_new() {
        let line = Line::new();
        assert!(line.is_empty());
        assert_eq!(line.len(), 0);
    }

    #[test]
    fn test_line_from_str() {
        let line = Line::from_str("Hello, World!");
        assert_eq!(line.len(), 13);
        assert_eq!(line.to_string(), "Hello, World!");
    }

    #[test]
    fn test_line_from_str_unicode() {
        let line = Line::from_str("Hello, 世界! 🌍");
        // "Hello, " = 7 chars, "世界" = 2 chars, "! " = 2 chars, "🌍" = 1 char = 12 total
        assert_eq!(line.len(), 12);
        assert_eq!(line.to_string(), "Hello, 世界! 🌍");
    }

    #[test]
    fn test_line_insert() {
        let mut line = Line::from_str("Hllo");
        line.insert(1, 'e');
        assert_eq!(line.to_string(), "Hello");
    }

    #[test]
    fn test_line_insert_at_start() {
        let mut line = Line::from_str("ello");
        line.insert(0, 'H');
        assert_eq!(line.to_string(), "Hello");
    }

    #[test]
    fn test_line_insert_at_end() {
        let mut line = Line::from_str("Hell");
        line.insert(4, 'o');
        assert_eq!(line.to_string(), "Hello");
    }

    #[test]
    fn test_line_insert_beyond_bounds() {
        let mut line = Line::from_str("Hi");
        line.insert(100, 'x'); // Should not panic, just ignore
        assert_eq!(line.to_string(), "Hi");
    }

    #[test]
    fn test_line_delete() {
        let mut line = Line::from_str("Hello");
        let deleted = line.delete(1);
        assert_eq!(deleted, Some('e'));
        assert_eq!(line.to_string(), "Hllo");
    }

    #[test]
    fn test_line_delete_at_start() {
        let mut line = Line::from_str("Hello");
        let deleted = line.delete(0);
        assert_eq!(deleted, Some('H'));
        assert_eq!(line.to_string(), "ello");
    }

    #[test]
    fn test_line_delete_beyond_bounds() {
        let mut line = Line::from_str("Hi");
        let deleted = line.delete(10);
        assert_eq!(deleted, None);
        assert_eq!(line.to_string(), "Hi");
    }

    #[test]
    fn test_line_split_off() {
        let mut line = Line::from_str("Hello World");
        let rest = line.split_off(6);
        assert_eq!(line.to_string(), "Hello ");
        assert_eq!(rest.to_string(), "World");
    }

    #[test]
    fn test_line_split_off_at_start() {
        let mut line = Line::from_str("Hello");
        let rest = line.split_off(0);
        assert_eq!(line.to_string(), "");
        assert_eq!(rest.to_string(), "Hello");
    }

    #[test]
    fn test_line_split_off_at_end() {
        let mut line = Line::from_str("Hello");
        let rest = line.split_off(5);
        assert_eq!(line.to_string(), "Hello");
        assert_eq!(rest.to_string(), "");
    }

    #[test]
    fn test_line_split_off_beyond_bounds() {
        let mut line = Line::from_str("Hello");
        let rest = line.split_off(100);
        assert_eq!(line.to_string(), "Hello");
        assert!(rest.is_empty());
    }

    #[test]
    fn test_line_append() {
        let mut line1 = Line::from_str("Hello ");
        let line2 = Line::from_str("World");
        line1.append(&line2);
        assert_eq!(line1.to_string(), "Hello World");
    }

    // Buffer tests
    #[test]
    fn test_buffer_new() {
        let buffer = Buffer::new();
        assert_eq!(buffer.line_count(), 1);
        assert!(!buffer.modified);
        assert!(buffer.path.is_none());
    }

    #[test]
    fn test_buffer_insert_char() {
        let mut buffer = Buffer::new();
        buffer.insert_char(0, 0, 'H');
        buffer.insert_char(0, 1, 'i');
        assert_eq!(buffer.line(0).unwrap().to_string(), "Hi");
        assert!(buffer.modified);
    }

    #[test]
    fn test_buffer_delete_char() {
        let mut buffer = Buffer::new();
        buffer.lines[0] = Line::from_str("Hello");
        let deleted = buffer.delete_char(0, 1);
        assert_eq!(deleted, Some('e'));
        assert_eq!(buffer.line(0).unwrap().to_string(), "Hllo");
        assert!(buffer.modified);
    }

    #[test]
    fn test_buffer_insert_newline() {
        let mut buffer = Buffer::new();
        buffer.lines[0] = Line::from_str("HelloWorld");
        buffer.insert_newline(0, 5);
        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.line(0).unwrap().to_string(), "Hello");
        assert_eq!(buffer.line(1).unwrap().to_string(), "World");
        assert!(buffer.modified);
    }

    #[test]
    fn test_buffer_delete_line() {
        let mut buffer = Buffer::new();
        buffer.lines = vec![
            Line::from_str("Hello"),
            Line::from_str("World"),
        ];
        buffer.delete_line(1);
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.line(0).unwrap().to_string(), "HelloWorld");
        assert!(buffer.modified);
    }

    #[test]
    fn test_buffer_delete_line_first_row() {
        let mut buffer = Buffer::new();
        buffer.lines = vec![
            Line::from_str("Hello"),
            Line::from_str("World"),
        ];
        buffer.delete_line(0); // Should not delete first line
        assert_eq!(buffer.line_count(), 2);
    }

    #[test]
    fn test_buffer_insert_empty_line() {
        let mut buffer = Buffer::new();
        buffer.lines[0] = Line::from_str("Hello");
        buffer.insert_empty_line(1);
        assert_eq!(buffer.line_count(), 2);
        assert!(buffer.line(1).unwrap().is_empty());
        assert!(buffer.modified);
    }

    #[test]
    fn test_buffer_filename() {
        let mut buffer = Buffer::new();
        buffer.path = Some(PathBuf::from("/path/to/file.txt"));
        assert_eq!(buffer.filename(), Some("file.txt".to_string()));
    }

    #[test]
    fn test_buffer_extension() {
        let mut buffer = Buffer::new();
        buffer.path = Some(PathBuf::from("/path/to/file.RS"));
        assert_eq!(buffer.extension(), Some("rs".to_string())); // Lowercase
    }

    #[test]
    fn test_buffer_no_extension() {
        let mut buffer = Buffer::new();
        buffer.path = Some(PathBuf::from("/path/to/Makefile"));
        assert_eq!(buffer.extension(), None);
    }

    #[test]
    fn test_buffer_save_and_load() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("zedit_test_file.txt");

        // Create and save
        let mut buffer = Buffer::new();
        buffer.lines = vec![
            Line::from_str("Line 1"),
            Line::from_str("Line 2"),
            Line::from_str("Line 3"),
        ];
        buffer.path = Some(test_file.clone());
        buffer.save().unwrap();

        // Load and verify
        let loaded = Buffer::from_file(&test_file).unwrap();
        assert_eq!(loaded.line_count(), 3);
        assert_eq!(loaded.line(0).unwrap().to_string(), "Line 1");
        assert_eq!(loaded.line(1).unwrap().to_string(), "Line 2");
        assert_eq!(loaded.line(2).unwrap().to_string(), "Line 3");

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_buffer_load_empty_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("zedit_test_empty.txt");

        // Create empty file
        std::fs::File::create(&test_file).unwrap();

        // Load and verify
        let loaded = Buffer::from_file(&test_file).unwrap();
        assert_eq!(loaded.line_count(), 1); // Should have at least one empty line

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_buffer_line_access() {
        let mut buffer = Buffer::new();
        buffer.lines = vec![
            Line::from_str("Line 0"),
            Line::from_str("Line 1"),
        ];

        assert!(buffer.line(0).is_some());
        assert!(buffer.line(1).is_some());
        assert!(buffer.line(2).is_none());

        assert!(buffer.line_mut(0).is_some());
        assert!(buffer.line_mut(2).is_none());
    }
}
