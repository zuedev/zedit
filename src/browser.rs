use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Entry type in the file browser
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryType {
    Directory,
    File,
    Symlink,
    Unknown,
}

/// A single entry in the directory listing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub entry_type: EntryType,
    pub size: Option<u64>,
    pub extension: Option<String>,
}

impl Entry {
    pub fn is_directory(&self) -> bool {
        self.entry_type == EntryType::Directory
    }

    pub fn is_file(&self) -> bool {
        self.entry_type == EntryType::File
    }

    pub fn display_name(&self) -> String {
        if self.is_directory() {
            format!("{}/", self.name)
        } else {
            self.name.clone()
        }
    }

    pub fn size_string(&self) -> String {
        match self.size {
            Some(size) => {
                if size < 1024 {
                    format!("{} B", size)
                } else if size < 1024 * 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else if size < 1024 * 1024 * 1024 {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                } else {
                    format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
                }
            }
            None => String::new(),
        }
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Directories first, then files
        match (&self.entry_type, &other.entry_type) {
            (EntryType::Directory, EntryType::Directory) => {
                self.name.to_lowercase().cmp(&other.name.to_lowercase())
            }
            (EntryType::Directory, _) => std::cmp::Ordering::Less,
            (_, EntryType::Directory) => std::cmp::Ordering::Greater,
            _ => self.name.to_lowercase().cmp(&other.name.to_lowercase()),
        }
    }
}

/// Directory browser
pub struct Browser {
    pub current_dir: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub show_hidden: bool,
}

impl Browser {
    pub fn new(path: &Path) -> io::Result<Self> {
        let current_dir = if path.is_dir() {
            path.canonicalize()?
        } else {
            path.parent()
                .unwrap_or(Path::new("."))
                .canonicalize()?
        };

        let mut browser = Browser {
            current_dir,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            show_hidden: false,
        };

        browser.refresh()?;
        Ok(browser)
    }

    /// Refresh the directory listing
    pub fn refresh(&mut self) -> io::Result<()> {
        self.entries.clear();

        // Add parent directory entry if not at root
        if let Some(parent) = self.current_dir.parent() {
            self.entries.push(Entry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                entry_type: EntryType::Directory,
                size: None,
                extension: None,
            });
        }

        // Read directory entries
        for entry_result in fs::read_dir(&self.current_dir)? {
            let entry = entry_result?;
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files unless show_hidden is true
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata();
            let path = entry.path();

            let entry_type = if let Ok(ref meta) = metadata {
                if meta.is_dir() {
                    EntryType::Directory
                } else if meta.is_file() {
                    EntryType::File
                } else if meta.is_symlink() {
                    EntryType::Symlink
                } else {
                    EntryType::Unknown
                }
            } else {
                EntryType::Unknown
            };

            let size = metadata.ok().map(|m| m.len());
            let extension = path
                .extension()
                .map(|e| e.to_string_lossy().to_string().to_lowercase());

            self.entries.push(Entry {
                name,
                path,
                entry_type,
                size,
                extension,
            });
        }

        // Sort entries
        self.entries.sort();

        // Reset selection if needed
        if self.selected >= self.entries.len() && !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }

        Ok(())
    }

    /// Navigate into a directory or return the selected file path
    pub fn enter(&mut self) -> io::Result<Option<PathBuf>> {
        if self.entries.is_empty() {
            return Ok(None);
        }

        let entry = &self.entries[self.selected];

        if entry.is_directory() {
            self.current_dir = entry.path.clone();
            self.selected = 0;
            self.scroll_offset = 0;
            self.refresh()?;
            Ok(None)
        } else {
            Ok(Some(entry.path.clone()))
        }
    }

    /// Go to parent directory
    pub fn go_up(&mut self) -> io::Result<()> {
        if let Some(parent) = self.current_dir.parent() {
            let old_dir = self.current_dir.clone();
            self.current_dir = parent.to_path_buf();
            self.refresh()?;

            // Try to select the directory we came from
            if let Some(pos) = self.entries.iter().position(|e| e.path == old_dir) {
                self.selected = pos;
            }
        }
        Ok(())
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    /// Move selection by page
    pub fn page_up(&mut self, page_size: usize) {
        if self.selected > page_size {
            self.selected -= page_size;
        } else {
            self.selected = 0;
        }
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.selected = (self.selected + page_size).min(self.entries.len().saturating_sub(1));
    }

    /// Move to first entry
    pub fn go_to_first(&mut self) {
        self.selected = 0;
    }

    /// Move to last entry
    pub fn go_to_last(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
    }

    /// Toggle showing hidden files
    pub fn toggle_hidden(&mut self) -> io::Result<()> {
        self.show_hidden = !self.show_hidden;
        self.refresh()
    }

    /// Get the currently selected entry
    pub fn selected_entry(&self) -> Option<&Entry> {
        self.entries.get(self.selected)
    }

    /// Update scroll offset for display
    pub fn update_scroll(&mut self, visible_height: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected - visible_height + 1;
        }
    }

    /// Get visible entries
    pub fn visible_entries(&self, visible_height: usize) -> impl Iterator<Item = (usize, &Entry)> {
        self.entries
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Entry tests
    #[test]
    fn test_entry_is_directory() {
        let entry = Entry {
            name: "test".to_string(),
            path: PathBuf::from("/test"),
            entry_type: EntryType::Directory,
            size: None,
            extension: None,
        };
        assert!(entry.is_directory());
        assert!(!entry.is_file());
    }

    #[test]
    fn test_entry_is_file() {
        let entry = Entry {
            name: "test.txt".to_string(),
            path: PathBuf::from("/test.txt"),
            entry_type: EntryType::File,
            size: Some(1024),
            extension: Some("txt".to_string()),
        };
        assert!(entry.is_file());
        assert!(!entry.is_directory());
    }

    #[test]
    fn test_entry_display_name_directory() {
        let entry = Entry {
            name: "folder".to_string(),
            path: PathBuf::from("/folder"),
            entry_type: EntryType::Directory,
            size: None,
            extension: None,
        };
        assert_eq!(entry.display_name(), "folder/");
    }

    #[test]
    fn test_entry_display_name_file() {
        let entry = Entry {
            name: "file.txt".to_string(),
            path: PathBuf::from("/file.txt"),
            entry_type: EntryType::File,
            size: Some(100),
            extension: Some("txt".to_string()),
        };
        assert_eq!(entry.display_name(), "file.txt");
    }

    #[test]
    fn test_entry_size_string_bytes() {
        let entry = Entry {
            name: "small.txt".to_string(),
            path: PathBuf::from("/small.txt"),
            entry_type: EntryType::File,
            size: Some(500),
            extension: None,
        };
        assert_eq!(entry.size_string(), "500 B");
    }

    #[test]
    fn test_entry_size_string_kilobytes() {
        let entry = Entry {
            name: "medium.txt".to_string(),
            path: PathBuf::from("/medium.txt"),
            entry_type: EntryType::File,
            size: Some(2048),
            extension: None,
        };
        assert_eq!(entry.size_string(), "2.0 KB");
    }

    #[test]
    fn test_entry_size_string_megabytes() {
        let entry = Entry {
            name: "large.txt".to_string(),
            path: PathBuf::from("/large.txt"),
            entry_type: EntryType::File,
            size: Some(5 * 1024 * 1024),
            extension: None,
        };
        assert_eq!(entry.size_string(), "5.0 MB");
    }

    #[test]
    fn test_entry_size_string_gigabytes() {
        let entry = Entry {
            name: "huge.txt".to_string(),
            path: PathBuf::from("/huge.txt"),
            entry_type: EntryType::File,
            size: Some(2 * 1024 * 1024 * 1024),
            extension: None,
        };
        assert_eq!(entry.size_string(), "2.0 GB");
    }

    #[test]
    fn test_entry_size_string_none() {
        let entry = Entry {
            name: "dir".to_string(),
            path: PathBuf::from("/dir"),
            entry_type: EntryType::Directory,
            size: None,
            extension: None,
        };
        assert_eq!(entry.size_string(), "");
    }

    #[test]
    fn test_entry_ordering_directories_first() {
        let dir = Entry {
            name: "zzz".to_string(),
            path: PathBuf::from("/zzz"),
            entry_type: EntryType::Directory,
            size: None,
            extension: None,
        };
        let file = Entry {
            name: "aaa.txt".to_string(),
            path: PathBuf::from("/aaa.txt"),
            entry_type: EntryType::File,
            size: Some(100),
            extension: Some("txt".to_string()),
        };

        assert!(dir < file); // Directories come first regardless of name
    }

    #[test]
    fn test_entry_ordering_alphabetical() {
        let entry_a = Entry {
            name: "apple".to_string(),
            path: PathBuf::from("/apple"),
            entry_type: EntryType::File,
            size: Some(100),
            extension: None,
        };
        let entry_b = Entry {
            name: "banana".to_string(),
            path: PathBuf::from("/banana"),
            entry_type: EntryType::File,
            size: Some(100),
            extension: None,
        };

        assert!(entry_a < entry_b);
    }

    #[test]
    fn test_entry_ordering_case_insensitive() {
        let entry_upper = Entry {
            name: "Apple".to_string(),
            path: PathBuf::from("/Apple"),
            entry_type: EntryType::File,
            size: Some(100),
            extension: None,
        };
        let entry_lower = Entry {
            name: "banana".to_string(),
            path: PathBuf::from("/banana"),
            entry_type: EntryType::File,
            size: Some(100),
            extension: None,
        };

        assert!(entry_upper < entry_lower);
    }

    // Browser tests
    #[test]
    fn test_browser_new() {
        let temp_dir = std::env::temp_dir();
        let browser = Browser::new(&temp_dir).unwrap();

        assert!(!browser.entries.is_empty());
        assert_eq!(browser.selected, 0);
        assert_eq!(browser.scroll_offset, 0);
        assert!(!browser.show_hidden);
    }

    #[test]
    fn test_browser_move_up_down() {
        let temp_dir = std::env::temp_dir();
        let mut browser = Browser::new(&temp_dir).unwrap();

        // Ensure we have entries
        if browser.entries.len() > 1 {
            browser.move_down();
            assert_eq!(browser.selected, 1);

            browser.move_up();
            assert_eq!(browser.selected, 0);

            // Can't go above 0
            browser.move_up();
            assert_eq!(browser.selected, 0);
        }
    }

    #[test]
    fn test_browser_move_down_at_end() {
        let temp_dir = std::env::temp_dir();
        let mut browser = Browser::new(&temp_dir).unwrap();

        let last_idx = browser.entries.len().saturating_sub(1);
        browser.selected = last_idx;
        browser.move_down();
        assert_eq!(browser.selected, last_idx); // Should not change
    }

    #[test]
    fn test_browser_go_to_first_last() {
        let temp_dir = std::env::temp_dir();
        let mut browser = Browser::new(&temp_dir).unwrap();

        if browser.entries.len() > 1 {
            browser.go_to_last();
            assert_eq!(browser.selected, browser.entries.len() - 1);

            browser.go_to_first();
            assert_eq!(browser.selected, 0);
        }
    }

    #[test]
    fn test_browser_page_up_down() {
        let temp_dir = std::env::temp_dir();
        let mut browser = Browser::new(&temp_dir).unwrap();

        let page_size = 10;
        if browser.entries.len() > page_size {
            browser.page_down(page_size);
            assert!(browser.selected >= page_size);

            browser.page_up(page_size);
            assert_eq!(browser.selected, 0);
        }
    }

    #[test]
    fn test_browser_selected_entry() {
        let temp_dir = std::env::temp_dir();
        let browser = Browser::new(&temp_dir).unwrap();

        let selected = browser.selected_entry();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap(), &browser.entries[0]);
    }

    #[test]
    fn test_browser_update_scroll() {
        let temp_dir = std::env::temp_dir();
        let mut browser = Browser::new(&temp_dir).unwrap();

        let visible_height = 5;

        // If we have enough entries, test scroll behavior
        if browser.entries.len() > visible_height {
            browser.selected = visible_height + 2;
            browser.update_scroll(visible_height);
            assert!(browser.scroll_offset > 0);

            browser.selected = 0;
            browser.update_scroll(visible_height);
            assert_eq!(browser.scroll_offset, 0);
        }
    }

    #[test]
    fn test_browser_visible_entries() {
        let temp_dir = std::env::temp_dir();
        let browser = Browser::new(&temp_dir).unwrap();

        let visible: Vec<_> = browser.visible_entries(3).collect();
        assert!(visible.len() <= 3);
        assert!(visible.len() <= browser.entries.len());
    }

    #[test]
    fn test_browser_toggle_hidden() {
        let temp_dir = std::env::temp_dir();
        let mut browser = Browser::new(&temp_dir).unwrap();

        assert!(!browser.show_hidden);
        browser.toggle_hidden().unwrap();
        assert!(browser.show_hidden);
        browser.toggle_hidden().unwrap();
        assert!(!browser.show_hidden);
    }

    #[test]
    fn test_browser_with_test_directory() {
        // Create a temporary test directory structure
        let temp_dir = std::env::temp_dir().join("zedit_browser_test");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up any previous test
        fs::create_dir_all(&temp_dir).unwrap();

        // Create some test files and directories
        fs::create_dir(temp_dir.join("subdir")).unwrap();
        fs::write(temp_dir.join("file1.txt"), "content").unwrap();
        fs::write(temp_dir.join("file2.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.join(".hidden"), "hidden content").unwrap();

        let mut browser = Browser::new(&temp_dir).unwrap();

        // Should have parent (..), subdir, file1.txt, file2.rs (not .hidden)
        // Count may vary if parent doesn't exist
        let visible_count = browser.entries.len();
        assert!(visible_count >= 3);

        // Hidden files should not be visible
        assert!(!browser.entries.iter().any(|e| e.name == ".hidden"));

        // Toggle hidden
        browser.toggle_hidden().unwrap();
        assert!(browser.entries.iter().any(|e| e.name == ".hidden"));

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_entry_types() {
        assert_eq!(EntryType::Directory, EntryType::Directory);
        assert_eq!(EntryType::File, EntryType::File);
        assert_eq!(EntryType::Symlink, EntryType::Symlink);
        assert_eq!(EntryType::Unknown, EntryType::Unknown);
        assert_ne!(EntryType::Directory, EntryType::File);
    }
}
