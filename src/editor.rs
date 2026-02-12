use crate::browser::Browser;
use crate::buffer::Buffer;
use crate::syntax::{HighlightState, Highlighter};
use crate::terminal::{ansi, Key, Size, Terminal};
use std::io::{self, Write};
use std::path::PathBuf;

/// Editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Search,
    Browser,
}

/// Editor state
pub struct Editor {
    terminal: Terminal,
    buffer: Buffer,
    highlighter: Highlighter,
    cursor_row: usize,
    cursor_col: usize,
    scroll_row: usize,
    scroll_col: usize,
    mode: Mode,
    command_buffer: String,
    search_buffer: String,
    search_direction: i8,
    message: Option<String>,
    browser: Option<Browser>,
    quit: bool,
    size: Size,
}

impl Editor {
    pub fn new() -> io::Result<Self> {
        let terminal = Terminal::new()?;
        let size = Terminal::size()?;

        Ok(Editor {
            terminal,
            buffer: Buffer::new(),
            highlighter: Highlighter::new(None),
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            mode: Mode::Normal,
            command_buffer: String::new(),
            search_buffer: String::new(),
            search_direction: 1,
            message: None,
            browser: None,
            quit: false,
            size,
        })
    }

    /// Open a file or directory
    pub fn open(&mut self, path: &PathBuf) -> io::Result<()> {
        if path.is_dir() {
            self.browser = Some(Browser::new(path)?);
            self.mode = Mode::Browser;
        } else {
            self.buffer = Buffer::from_file(path)?;
            self.highlighter = Highlighter::new(self.buffer.extension().as_deref());
            self.cursor_row = 0;
            self.cursor_col = 0;
            self.scroll_row = 0;
            self.scroll_col = 0;
            self.mode = Mode::Normal;
        }
        Ok(())
    }

    /// Main event loop
    pub fn run(&mut self) -> io::Result<()> {
        Terminal::hide_cursor();

        loop {
            self.size = Terminal::size()?;
            self.draw()?;

            if let Some(key) = self.terminal.read_key()? {
                self.handle_key(key)?;
            }

            if self.quit {
                break;
            }
        }

        Terminal::show_cursor();
        Terminal::clear_screen();
        Terminal::flush()?;

        Ok(())
    }

    /// Handle key input
    fn handle_key(&mut self, key: Key) -> io::Result<()> {
        self.message = None;

        match self.mode {
            Mode::Normal => self.handle_normal_key(key)?,
            Mode::Insert => self.handle_insert_key(key)?,
            Mode::Command => self.handle_command_key(key)?,
            Mode::Search => self.handle_search_key(key)?,
            Mode::Browser => self.handle_browser_key(key)?,
        }

        Ok(())
    }

    /// Handle keys in normal mode
    fn handle_normal_key(&mut self, key: Key) -> io::Result<()> {
        match key {
            // Movement
            Key::Char('h') | Key::Left => self.move_cursor_left(),
            Key::Char('j') | Key::Down => self.move_cursor_down(),
            Key::Char('k') | Key::Up => self.move_cursor_up(),
            Key::Char('l') | Key::Right => self.move_cursor_right(),
            Key::Char('0') | Key::Home => self.cursor_col = 0,
            Key::Char('$') | Key::End => self.move_cursor_end_of_line(),
            Key::Char('g') => self.cursor_row = 0,
            Key::Char('G') => self.cursor_row = self.buffer.line_count().saturating_sub(1),
            Key::Char('w') => self.move_word_forward(),
            Key::Char('b') => self.move_word_backward(),
            Key::PageUp | Key::Ctrl('u') => self.page_up(),
            Key::PageDown | Key::Ctrl('d') => self.page_down(),

            // Mode switching
            Key::Char('i') => self.mode = Mode::Insert,
            Key::Char('I') => {
                self.cursor_col = 0;
                self.mode = Mode::Insert;
            }
            Key::Char('a') => {
                self.move_cursor_right();
                self.mode = Mode::Insert;
            }
            Key::Char('A') => {
                self.move_cursor_end_of_line();
                self.move_cursor_right();
                self.mode = Mode::Insert;
            }
            Key::Char('o') => {
                self.cursor_row = self.cursor_row.saturating_add(1).min(self.buffer.line_count());
                self.buffer.insert_empty_line(self.cursor_row);
                self.cursor_col = 0;
                self.mode = Mode::Insert;
            }
            Key::Char('O') => {
                self.buffer.insert_empty_line(self.cursor_row);
                self.cursor_col = 0;
                self.mode = Mode::Insert;
            }
            Key::Char(':') => {
                self.command_buffer.clear();
                self.mode = Mode::Command;
            }
            Key::Char('/') => {
                self.search_buffer.clear();
                self.search_direction = 1;
                self.mode = Mode::Search;
            }
            Key::Char('?') => {
                self.search_buffer.clear();
                self.search_direction = -1;
                self.mode = Mode::Search;
            }
            Key::Char('n') => self.search_next(),
            Key::Char('N') => self.search_prev(),

            // Editing
            Key::Char('x') => {
                if let Some(line) = self.buffer.line(self.cursor_row) {
                    if self.cursor_col < line.len() {
                        self.buffer.delete_char(self.cursor_row, self.cursor_col);
                    }
                }
            }
            Key::Char('d') => {
                // Simple dd implementation - delete line
                if self.buffer.line_count() > 1 {
                    self.buffer.lines.remove(self.cursor_row);
                    if self.cursor_row >= self.buffer.line_count() {
                        self.cursor_row = self.buffer.line_count() - 1;
                    }
                    self.buffer.modified = true;
                }
            }

            // File browser
            Key::Char('e') => {
                let dir = self
                    .buffer
                    .path
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                self.browser = Some(Browser::new(&dir)?);
                self.mode = Mode::Browser;
            }

            Key::Ctrl('q') => {
                if self.buffer.modified {
                    self.message = Some("Unsaved changes! Use :q! to force quit".to_string());
                } else {
                    self.quit = true;
                }
            }
            Key::Ctrl('s') => {
                self.save_file()?;
            }

            _ => {}
        }

        self.clamp_cursor();
        Ok(())
    }

    /// Handle keys in insert mode
    fn handle_insert_key(&mut self, key: Key) -> io::Result<()> {
        match key {
            Key::Escape => {
                self.mode = Mode::Normal;
                self.move_cursor_left();
            }
            Key::Char(c) => {
                self.buffer.insert_char(self.cursor_row, self.cursor_col, c);
                self.cursor_col += 1;
            }
            Key::Enter => {
                self.buffer.insert_newline(self.cursor_row, self.cursor_col);
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
            Key::Backspace => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    self.buffer.delete_char(self.cursor_row, self.cursor_col);
                } else if self.cursor_row > 0 {
                    let prev_line_len = self
                        .buffer
                        .line(self.cursor_row - 1)
                        .map(|l| l.len())
                        .unwrap_or(0);
                    self.buffer.delete_line(self.cursor_row);
                    self.cursor_row -= 1;
                    self.cursor_col = prev_line_len;
                }
            }
            Key::Delete => {
                if let Some(line) = self.buffer.line(self.cursor_row) {
                    if self.cursor_col < line.len() {
                        self.buffer.delete_char(self.cursor_row, self.cursor_col);
                    } else if self.cursor_row + 1 < self.buffer.line_count() {
                        // Merge with next line
                        let next_line = self.buffer.lines.remove(self.cursor_row + 1);
                        if let Some(current) = self.buffer.line_mut(self.cursor_row) {
                            current.append(&next_line);
                        }
                        self.buffer.modified = true;
                    }
                }
            }
            Key::Tab => {
                // Insert 4 spaces for tab
                for _ in 0..4 {
                    self.buffer.insert_char(self.cursor_row, self.cursor_col, ' ');
                    self.cursor_col += 1;
                }
            }
            Key::Left => self.move_cursor_left(),
            Key::Right => self.move_cursor_right(),
            Key::Up => self.move_cursor_up(),
            Key::Down => self.move_cursor_down(),
            Key::Home => self.cursor_col = 0,
            Key::End => self.move_cursor_end_of_line(),
            Key::Ctrl('s') => self.save_file()?,
            _ => {}
        }

        self.clamp_cursor();
        Ok(())
    }

    /// Handle keys in command mode
    fn handle_command_key(&mut self, key: Key) -> io::Result<()> {
        match key {
            Key::Escape => {
                self.mode = Mode::Normal;
            }
            Key::Enter => {
                let cmd = self.command_buffer.clone();
                self.mode = Mode::Normal;
                self.execute_command(&cmd)?;
            }
            Key::Char(c) => {
                self.command_buffer.push(c);
            }
            Key::Backspace => {
                self.command_buffer.pop();
                if self.command_buffer.is_empty() {
                    self.mode = Mode::Normal;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys in search mode
    fn handle_search_key(&mut self, key: Key) -> io::Result<()> {
        match key {
            Key::Escape => {
                self.mode = Mode::Normal;
            }
            Key::Enter => {
                self.mode = Mode::Normal;
                self.perform_search();
            }
            Key::Char(c) => {
                self.search_buffer.push(c);
            }
            Key::Backspace => {
                self.search_buffer.pop();
                if self.search_buffer.is_empty() {
                    self.mode = Mode::Normal;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys in browser mode
    fn handle_browser_key(&mut self, key: Key) -> io::Result<()> {
        if let Some(browser) = &mut self.browser {
            match key {
                Key::Escape | Key::Char('q') => {
                    self.browser = None;
                    self.mode = Mode::Normal;
                }
                Key::Up | Key::Char('k') => browser.move_up(),
                Key::Down | Key::Char('j') => browser.move_down(),
                Key::PageUp => browser.page_up(self.size.rows as usize - 4),
                Key::PageDown => browser.page_down(self.size.rows as usize - 4),
                Key::Home | Key::Char('g') => browser.go_to_first(),
                Key::End | Key::Char('G') => browser.go_to_last(),
                Key::Left | Key::Char('h') | Key::Backspace => browser.go_up()?,
                Key::Enter | Key::Right | Key::Char('l') => {
                    if let Some(path) = browser.enter()? {
                        self.buffer = Buffer::from_file(&path)?;
                        self.highlighter = Highlighter::new(self.buffer.extension().as_deref());
                        self.cursor_row = 0;
                        self.cursor_col = 0;
                        self.scroll_row = 0;
                        self.scroll_col = 0;
                        self.browser = None;
                        self.mode = Mode::Normal;
                    }
                }
                Key::Char('.') => browser.toggle_hidden()?,
                Key::Char('r') => browser.refresh()?,
                _ => {}
            }
        }
        Ok(())
    }

    /// Execute a command
    fn execute_command(&mut self, cmd: &str) -> io::Result<()> {
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();

        match parts.as_slice() {
            ["q"] | ["quit"] => {
                if self.buffer.modified {
                    self.message = Some("Unsaved changes! Use :q! to force quit".to_string());
                } else {
                    self.quit = true;
                }
            }
            ["q!"] | ["quit!"] => {
                self.quit = true;
            }
            ["w"] | ["write"] => {
                self.save_file()?;
            }
            ["w", path] | ["write", path] => {
                self.buffer.save_as(PathBuf::from(path))?;
                self.message = Some(format!("Saved to {}", path));
            }
            ["wq"] => {
                self.save_file()?;
                self.quit = true;
            }
            ["e", path] | ["edit", path] => {
                let path = PathBuf::from(path);
                if path.is_dir() {
                    self.browser = Some(Browser::new(&path)?);
                    self.mode = Mode::Browser;
                } else {
                    self.open(&path)?;
                }
            }
            ["e"] | ["edit"] => {
                let dir = self
                    .buffer
                    .path
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                self.browser = Some(Browser::new(&dir)?);
                self.mode = Mode::Browser;
            }
            [num] if num.parse::<usize>().is_ok() => {
                let line = num.parse::<usize>().unwrap();
                self.cursor_row = line.saturating_sub(1).min(self.buffer.line_count() - 1);
                self.cursor_col = 0;
            }
            ["set", "number"] | ["set", "nu"] => {
                self.message = Some("Line numbers enabled".to_string());
            }
            ["help"] | ["h"] => {
                self.message = Some("Commands: :w :q :wq :e <file> :<num>".to_string());
            }
            _ => {
                self.message = Some(format!("Unknown command: {}", cmd));
            }
        }

        Ok(())
    }

    /// Save the current file
    fn save_file(&mut self) -> io::Result<()> {
        if self.buffer.path.is_some() {
            self.buffer.save()?;
            self.message = Some("File saved".to_string());
        } else {
            self.message = Some("No filename. Use :w <filename>".to_string());
        }
        Ok(())
    }

    /// Perform search
    fn perform_search(&mut self) {
        if self.search_buffer.is_empty() {
            return;
        }

        let start_row = self.cursor_row;
        let start_col = self.cursor_col + 1;

        // Search forward from cursor
        for row in start_row..self.buffer.line_count() {
            if let Some(line) = self.buffer.line(row) {
                let line_str = line.to_string();
                let search_start = if row == start_row { start_col } else { 0 };
                if search_start < line_str.len() {
                    if let Some(pos) = line_str[search_start..].find(&self.search_buffer) {
                        self.cursor_row = row;
                        self.cursor_col = search_start + pos;
                        return;
                    }
                }
            }
        }

        // Wrap around
        for row in 0..=start_row {
            if let Some(line) = self.buffer.line(row) {
                let line_str = line.to_string();
                if let Some(pos) = line_str.find(&self.search_buffer) {
                    self.cursor_row = row;
                    self.cursor_col = pos;
                    self.message = Some("Search wrapped".to_string());
                    return;
                }
            }
        }

        self.message = Some(format!("Pattern not found: {}", self.search_buffer));
    }

    fn search_next(&mut self) {
        self.perform_search();
    }

    fn search_prev(&mut self) {
        if self.search_buffer.is_empty() {
            return;
        }

        let start_row = self.cursor_row;
        let start_col = self.cursor_col;

        // Search backward from cursor
        for row in (0..=start_row).rev() {
            if let Some(line) = self.buffer.line(row) {
                let line_str = line.to_string();
                let search_end = if row == start_row { start_col } else { line_str.len() };
                if let Some(pos) = line_str[..search_end].rfind(&self.search_buffer) {
                    self.cursor_row = row;
                    self.cursor_col = pos;
                    return;
                }
            }
        }

        // Wrap around
        for row in (start_row..self.buffer.line_count()).rev() {
            if let Some(line) = self.buffer.line(row) {
                let line_str = line.to_string();
                if let Some(pos) = line_str.rfind(&self.search_buffer) {
                    self.cursor_row = row;
                    self.cursor_col = pos;
                    self.message = Some("Search wrapped".to_string());
                    return;
                }
            }
        }

        self.message = Some(format!("Pattern not found: {}", self.search_buffer));
    }

    /// Movement helpers
    fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if let Some(line) = self.buffer.line(self.cursor_row) {
            let max_col = if self.mode == Mode::Insert {
                line.len()
            } else {
                line.len().saturating_sub(1)
            };
            if self.cursor_col < max_col {
                self.cursor_col += 1;
            }
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_row + 1 < self.buffer.line_count() {
            self.cursor_row += 1;
        }
    }

    fn move_cursor_end_of_line(&mut self) {
        if let Some(line) = self.buffer.line(self.cursor_row) {
            self.cursor_col = if self.mode == Mode::Insert {
                line.len()
            } else {
                line.len().saturating_sub(1)
            };
        }
    }

    fn move_word_forward(&mut self) {
        if let Some(line) = self.buffer.line(self.cursor_row) {
            let chars: Vec<char> = line.chars.clone();
            let mut col = self.cursor_col;

            // Skip current word
            while col < chars.len() && chars[col].is_alphanumeric() {
                col += 1;
            }
            // Skip whitespace
            while col < chars.len() && !chars[col].is_alphanumeric() {
                col += 1;
            }

            if col >= chars.len() && self.cursor_row + 1 < self.buffer.line_count() {
                self.cursor_row += 1;
                self.cursor_col = 0;
            } else {
                self.cursor_col = col.min(chars.len().saturating_sub(1));
            }
        }
    }

    fn move_word_backward(&mut self) {
        if let Some(line) = self.buffer.line(self.cursor_row) {
            let chars: Vec<char> = line.chars.clone();
            let mut col = self.cursor_col;

            if col == 0 && self.cursor_row > 0 {
                self.cursor_row -= 1;
                if let Some(prev_line) = self.buffer.line(self.cursor_row) {
                    self.cursor_col = prev_line.len().saturating_sub(1);
                }
                return;
            }

            // Skip whitespace
            while col > 0 && !chars[col - 1].is_alphanumeric() {
                col -= 1;
            }
            // Skip word
            while col > 0 && chars[col - 1].is_alphanumeric() {
                col -= 1;
            }

            self.cursor_col = col;
        }
    }

    fn page_up(&mut self) {
        let page_size = (self.size.rows as usize).saturating_sub(4);
        if self.cursor_row > page_size {
            self.cursor_row -= page_size;
        } else {
            self.cursor_row = 0;
        }
    }

    fn page_down(&mut self) {
        let page_size = (self.size.rows as usize).saturating_sub(4);
        self.cursor_row = (self.cursor_row + page_size).min(self.buffer.line_count() - 1);
    }

    fn clamp_cursor(&mut self) {
        // Clamp row
        if self.cursor_row >= self.buffer.line_count() {
            self.cursor_row = self.buffer.line_count().saturating_sub(1);
        }

        // Clamp column
        if let Some(line) = self.buffer.line(self.cursor_row) {
            let max_col = if self.mode == Mode::Insert {
                line.len()
            } else {
                line.len().saturating_sub(1).max(0)
            };
            if self.cursor_col > max_col {
                self.cursor_col = max_col;
            }
        }
    }

    /// Drawing
    fn draw(&mut self) -> io::Result<()> {
        let mut output = String::new();
        output.push_str(ansi::CURSOR_HIDE);
        output.push_str(ansi::CURSOR_HOME);

        if self.mode == Mode::Browser {
            self.draw_browser(&mut output)?;
        } else {
            self.draw_editor(&mut output)?;
        }

        print!("{}", output);
        io::stdout().flush()?;
        Ok(())
    }

    fn draw_editor(&mut self, output: &mut String) -> io::Result<()> {
        let content_height = self.size.rows.saturating_sub(2) as usize;
        let content_width = self.size.cols as usize;
        let gutter_width = 5; // Line numbers

        // Update scroll
        self.update_scroll(content_height);

        let mut highlight_state = HighlightState::default();

        // Skip lines before scroll_row for highlighting state
        for row in 0..self.scroll_row {
            if let Some(line) = self.buffer.line(row) {
                let line_str = line.to_string();
                self.highlighter.highlight_line(&line_str, &mut highlight_state);
            }
        }

        // Draw content lines
        for screen_row in 0..content_height {
            let file_row = self.scroll_row + screen_row;

            // Move to line start
            output.push_str(&ansi::cursor_position(screen_row as u16, 0));
            output.push_str(ansi::CLEAR_LINE);

            if file_row < self.buffer.line_count() {
                // Line number
                output.push_str(ansi::FG_BRIGHT_BLACK);
                output.push_str(&format!("{:>4} ", file_row + 1));
                output.push_str(ansi::RESET);

                // Line content
                if let Some(line) = self.buffer.line(file_row) {
                    let line_str = line.to_string();
                    let tokens = self.highlighter.highlight_line(&line_str, &mut highlight_state);

                    let mut col = 0;
                    let visible_start = self.scroll_col;
                    let visible_end = visible_start + content_width - gutter_width;

                    for token in tokens {
                        let token_start = col;
                        let token_end = col + token.text.len();

                        if token_end > visible_start && token_start < visible_end {
                            output.push_str(token.token_type.color());

                            let start = token_start.max(visible_start) - token_start;
                            let end = token_end.min(visible_end) - token_start;

                            let visible_text: String = token.text.chars().skip(start).take(end - start).collect();
                            output.push_str(&visible_text);
                            output.push_str(ansi::RESET);
                        }

                        col = token_end;
                    }
                }
            } else {
                // Empty line indicator
                output.push_str(ansi::FG_BLUE);
                output.push_str("~");
                output.push_str(ansi::RESET);
            }
        }

        // Draw status line
        self.draw_status_line(output, content_height as u16)?;

        // Draw command/message line
        self.draw_command_line(output, content_height as u16 + 1)?;

        // Position cursor
        let cursor_screen_row = (self.cursor_row - self.scroll_row) as u16;
        let cursor_screen_col = (self.cursor_col - self.scroll_col + gutter_width) as u16;
        output.push_str(&ansi::cursor_position(cursor_screen_row, cursor_screen_col));
        output.push_str(ansi::CURSOR_SHOW);

        Ok(())
    }

    fn draw_browser(&mut self, output: &mut String) -> io::Result<()> {
        let content_height = self.size.rows.saturating_sub(2) as usize;

        if let Some(browser) = &mut self.browser {
            browser.update_scroll(content_height);

            // Draw header
            output.push_str(&ansi::cursor_position(0, 0));
            output.push_str(ansi::CLEAR_LINE);
            output.push_str(ansi::REVERSE);
            output.push_str(&format!(
                " {} ",
                browser.current_dir.to_string_lossy()
            ));
            output.push_str(ansi::RESET);

            // Draw entries
            for (screen_row, (idx, entry)) in browser.visible_entries(content_height - 1).enumerate() {
                output.push_str(&ansi::cursor_position((screen_row + 1) as u16, 0));
                output.push_str(ansi::CLEAR_LINE);

                if idx == browser.selected {
                    output.push_str(ansi::REVERSE);
                }

                // Icon and name
                if entry.is_directory() {
                    output.push_str(ansi::FG_BLUE);
                    output.push_str(ansi::BOLD);
                } else {
                    output.push_str(ansi::FG_DEFAULT);
                }

                let name = entry.display_name();
                let max_name_len = (self.size.cols as usize).saturating_sub(15);
                let display_name = if name.len() > max_name_len {
                    format!("{}...", &name[..max_name_len - 3])
                } else {
                    name
                };

                output.push_str(&format!(" {:<width$}", display_name, width = max_name_len));

                // Size
                if entry.is_file() {
                    output.push_str(ansi::FG_BRIGHT_BLACK);
                    output.push_str(&format!(" {:>10}", entry.size_string()));
                }

                output.push_str(ansi::RESET);
            }

            // Fill remaining lines
            for screen_row in (browser.entries.len().min(content_height))..content_height {
                output.push_str(&ansi::cursor_position((screen_row + 1) as u16, 0));
                output.push_str(ansi::CLEAR_LINE);
            }

            // Status line
            output.push_str(&ansi::cursor_position(content_height as u16, 0));
            output.push_str(ansi::CLEAR_LINE);
            output.push_str(ansi::REVERSE);
            output.push_str(&format!(
                " BROWSER | {} items | . toggle hidden | Enter/l open | h/Backspace up | q close ",
                browser.entries.len()
            ));
            output.push_str(ansi::RESET);

            // Message line
            output.push_str(&ansi::cursor_position(content_height as u16 + 1, 0));
            output.push_str(ansi::CLEAR_LINE);
        }

        Ok(())
    }

    fn draw_status_line(&self, output: &mut String, row: u16) -> io::Result<()> {
        output.push_str(&ansi::cursor_position(row, 0));
        output.push_str(ansi::CLEAR_LINE);
        output.push_str(ansi::REVERSE);

        let mode_str = match self.mode {
            Mode::Normal => " NORMAL ",
            Mode::Insert => " INSERT ",
            Mode::Command => " COMMAND ",
            Mode::Search => " SEARCH ",
            Mode::Browser => " BROWSER ",
        };

        let filename = self
            .buffer
            .filename()
            .unwrap_or_else(|| "[No Name]".to_string());

        let modified = if self.buffer.modified { " [+]" } else { "" };
        let readonly = if self.buffer.readonly { " [RO]" } else { "" };

        let left = format!("{} {}{}{}", mode_str, filename, modified, readonly);
        let right = format!(
            " {}:{} ",
            self.cursor_row + 1,
            self.cursor_col + 1
        );

        let padding = (self.size.cols as usize)
            .saturating_sub(left.len())
            .saturating_sub(right.len());

        output.push_str(&left);
        output.push_str(&" ".repeat(padding));
        output.push_str(&right);
        output.push_str(ansi::RESET);

        Ok(())
    }

    fn draw_command_line(&self, output: &mut String, row: u16) -> io::Result<()> {
        output.push_str(&ansi::cursor_position(row, 0));
        output.push_str(ansi::CLEAR_LINE);

        match self.mode {
            Mode::Command => {
                output.push_str(&format!(":{}", self.command_buffer));
            }
            Mode::Search => {
                let prefix = if self.search_direction > 0 { "/" } else { "?" };
                output.push_str(&format!("{}{}", prefix, self.search_buffer));
            }
            _ => {
                if let Some(msg) = &self.message {
                    output.push_str(msg);
                }
            }
        }

        Ok(())
    }

    fn update_scroll(&mut self, content_height: usize) {
        // Vertical scroll
        if self.cursor_row < self.scroll_row {
            self.scroll_row = self.cursor_row;
        } else if self.cursor_row >= self.scroll_row + content_height {
            self.scroll_row = self.cursor_row - content_height + 1;
        }

        // Horizontal scroll
        let gutter_width = 5;
        let visible_width = (self.size.cols as usize).saturating_sub(gutter_width);

        if self.cursor_col < self.scroll_col {
            self.scroll_col = self.cursor_col;
        } else if self.cursor_col >= self.scroll_col + visible_width {
            self.scroll_col = self.cursor_col - visible_width + 1;
        }
    }
}
