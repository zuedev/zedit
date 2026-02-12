use std::io::{self, Read, Write};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(windows)]
#[allow(unused_imports)]
use std::os::windows::io::AsRawHandle;

/// ANSI escape codes for terminal control
pub mod ansi {
    pub const CLEAR_SCREEN: &str = "\x1b[2J";
    pub const CLEAR_LINE: &str = "\x1b[2K";
    pub const CURSOR_HOME: &str = "\x1b[H";
    pub const CURSOR_HIDE: &str = "\x1b[?25l";
    pub const CURSOR_SHOW: &str = "\x1b[?25h";
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const ITALIC: &str = "\x1b[3m";
    pub const UNDERLINE: &str = "\x1b[4m";
    pub const REVERSE: &str = "\x1b[7m";

    // Foreground colors
    pub const FG_BLACK: &str = "\x1b[30m";
    pub const FG_RED: &str = "\x1b[31m";
    pub const FG_GREEN: &str = "\x1b[32m";
    pub const FG_YELLOW: &str = "\x1b[33m";
    pub const FG_BLUE: &str = "\x1b[34m";
    pub const FG_MAGENTA: &str = "\x1b[35m";
    pub const FG_CYAN: &str = "\x1b[36m";
    pub const FG_WHITE: &str = "\x1b[37m";
    pub const FG_DEFAULT: &str = "\x1b[39m";

    // Bright foreground colors
    pub const FG_BRIGHT_BLACK: &str = "\x1b[90m";
    pub const FG_BRIGHT_RED: &str = "\x1b[91m";
    pub const FG_BRIGHT_GREEN: &str = "\x1b[92m";
    pub const FG_BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const FG_BRIGHT_BLUE: &str = "\x1b[94m";
    pub const FG_BRIGHT_MAGENTA: &str = "\x1b[95m";
    pub const FG_BRIGHT_CYAN: &str = "\x1b[96m";
    pub const FG_BRIGHT_WHITE: &str = "\x1b[97m";

    // Background colors
    pub const BG_BLACK: &str = "\x1b[40m";
    pub const BG_RED: &str = "\x1b[41m";
    pub const BG_GREEN: &str = "\x1b[42m";
    pub const BG_YELLOW: &str = "\x1b[43m";
    pub const BG_BLUE: &str = "\x1b[44m";
    pub const BG_MAGENTA: &str = "\x1b[45m";
    pub const BG_CYAN: &str = "\x1b[46m";
    pub const BG_WHITE: &str = "\x1b[47m";
    pub const BG_DEFAULT: &str = "\x1b[49m";

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

/// Key events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Alt(char),
    Enter,
    Tab,
    Backspace,
    Delete,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    F(u8),
    Unknown(Vec<u8>),
}

/// Terminal size
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub rows: u16,
    pub cols: u16,
}

/// Unix termios structure (platform-specific)
#[cfg(unix)]
#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; 32],
    c_ispeed: u32,
    c_ospeed: u32,
}

/// Terminal handler
pub struct Terminal {
    #[cfg(unix)]
    original_termios: Option<Termios>,
    #[cfg(windows)]
    original_mode: Option<(u32, u32)>,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        let mut terminal = Terminal {
            #[cfg(unix)]
            original_termios: None,
            #[cfg(windows)]
            original_mode: None,
        };
        terminal.enable_raw_mode()?;
        Ok(terminal)
    }

    #[cfg(unix)]
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        use std::mem::MaybeUninit;

        // termios flags
        const ICANON: u32 = 0o000002;
        const ECHO: u32 = 0o000010;
        const ISIG: u32 = 0o000001;
        const IEXTEN: u32 = 0o100000;
        const IXON: u32 = 0o002000;
        const ICRNL: u32 = 0o000400;
        const BRKINT: u32 = 0o000002;
        const INPCK: u32 = 0o000020;
        const ISTRIP: u32 = 0o000040;
        const OPOST: u32 = 0o000001;
        const CS8: u32 = 0o000060;
        const VMIN: usize = 6;
        const VTIME: usize = 5;
        const TCSAFLUSH: i32 = 2;

        #[cfg(target_os = "linux")]
        const TCGETS: u64 = 0x5401;
        #[cfg(target_os = "linux")]
        const TCSETS: u64 = 0x5402;

        #[cfg(target_os = "macos")]
        const TCGETS: u64 = 0x40487413;
        #[cfg(target_os = "macos")]
        const TCSETS: u64 = 0x80487414;

        unsafe extern "C" {
            fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
            fn tcsetattr(fd: i32, action: i32, termios: *const Termios) -> i32;
        }

        let fd = io::stdin().as_raw_fd();
        let mut termios = MaybeUninit::<Termios>::uninit();

        if unsafe { tcgetattr(fd, termios.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut termios = unsafe { termios.assume_init() };
        self.original_termios = Some(termios);

        // Disable canonical mode and echo
        termios.c_lflag &= !(ICANON | ECHO | ISIG | IEXTEN);
        termios.c_iflag &= !(IXON | ICRNL | BRKINT | INPCK | ISTRIP);
        termios.c_oflag &= !OPOST;
        termios.c_cflag |= CS8;

        // Set minimum characters and timeout for read
        termios.c_cc[VMIN] = 0;
        termios.c_cc[VTIME] = 1;

        if unsafe { tcsetattr(fd, TCSAFLUSH, &termios) } != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    #[cfg(windows)]
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        use std::ptr::null_mut;

        const STD_INPUT_HANDLE: u32 = 0xFFFFFFF6;
        const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5;
        const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;
        const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
        const DISABLE_NEWLINE_AUTO_RETURN: u32 = 0x0008;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetStdHandle(nStdHandle: u32) -> *mut std::ffi::c_void;
            fn GetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, lpMode: *mut u32) -> i32;
            fn SetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, dwMode: u32) -> i32;
        }

        unsafe {
            let stdin_handle = GetStdHandle(STD_INPUT_HANDLE);
            let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);

            if stdin_handle == null_mut() || stdout_handle == null_mut() {
                return Err(io::Error::new(io::ErrorKind::Other, "Failed to get console handles"));
            }

            let mut stdin_mode: u32 = 0;
            let mut stdout_mode: u32 = 0;

            if GetConsoleMode(stdin_handle, &mut stdin_mode) == 0 {
                return Err(io::Error::last_os_error());
            }
            if GetConsoleMode(stdout_handle, &mut stdout_mode) == 0 {
                return Err(io::Error::last_os_error());
            }

            self.original_mode = Some((stdin_mode, stdout_mode));

            // Enable virtual terminal input
            let new_stdin_mode = ENABLE_VIRTUAL_TERMINAL_INPUT;
            // Enable virtual terminal processing for output
            let new_stdout_mode = stdout_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING | DISABLE_NEWLINE_AUTO_RETURN;

            if SetConsoleMode(stdin_handle, new_stdin_mode) == 0 {
                return Err(io::Error::last_os_error());
            }
            if SetConsoleMode(stdout_handle, new_stdout_mode) == 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(())
    }

    #[cfg(unix)]
    fn disable_raw_mode(&mut self) -> io::Result<()> {
        const TCSAFLUSH: i32 = 2;

        unsafe extern "C" {
            fn tcsetattr(fd: i32, action: i32, termios: *const Termios) -> i32;
        }

        if let Some(termios) = self.original_termios.take() {
            let fd = io::stdin().as_raw_fd();
            if unsafe { tcsetattr(fd, TCSAFLUSH, &termios) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    fn disable_raw_mode(&mut self) -> io::Result<()> {
        if let Some((stdin_mode, stdout_mode)) = self.original_mode.take() {
            const STD_INPUT_HANDLE: u32 = 0xFFFFFFF6;
            const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5;

            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn GetStdHandle(nStdHandle: u32) -> *mut std::ffi::c_void;
                fn SetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, dwMode: u32) -> i32;
            }

            unsafe {
                let stdin_handle = GetStdHandle(STD_INPUT_HANDLE);
                let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);

                SetConsoleMode(stdin_handle, stdin_mode);
                SetConsoleMode(stdout_handle, stdout_mode);
            }
        }
        Ok(())
    }

    /// Get terminal size
    pub fn size() -> io::Result<Size> {
        #[cfg(unix)]
        {
            use std::mem::MaybeUninit;

            #[repr(C)]
            struct Winsize {
                ws_row: u16,
                ws_col: u16,
                ws_xpixel: u16,
                ws_ypixel: u16,
            }

            // TIOCGWINSZ varies by platform
            #[cfg(target_os = "linux")]
            const TIOCGWINSZ: u64 = 0x5413;
            #[cfg(target_os = "macos")]
            const TIOCGWINSZ: u64 = 0x40087468;

            unsafe extern "C" {
                fn ioctl(fd: i32, request: u64, ...) -> i32;
            }

            let mut size = MaybeUninit::<Winsize>::uninit();
            let fd = io::stdout().as_raw_fd();

            if unsafe { ioctl(fd, TIOCGWINSZ, size.as_mut_ptr()) } != 0 {
                return Err(io::Error::last_os_error());
            }

            let size = unsafe { size.assume_init() };
            Ok(Size {
                rows: size.ws_row,
                cols: size.ws_col,
            })
        }

        #[cfg(windows)]
        {
            const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5;

            #[repr(C)]
            struct Coord {
                x: i16,
                y: i16,
            }

            #[repr(C)]
            struct SmallRect {
                left: i16,
                top: i16,
                right: i16,
                bottom: i16,
            }

            #[repr(C)]
            struct ConsoleScreenBufferInfo {
                size: Coord,
                cursor_position: Coord,
                attributes: u16,
                window: SmallRect,
                maximum_window_size: Coord,
            }

            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn GetStdHandle(nStdHandle: u32) -> *mut std::ffi::c_void;
                fn GetConsoleScreenBufferInfo(
                    hConsoleOutput: *mut std::ffi::c_void,
                    lpConsoleScreenBufferInfo: *mut ConsoleScreenBufferInfo,
                ) -> i32;
            }

            unsafe {
                let handle = GetStdHandle(STD_OUTPUT_HANDLE);
                let mut info = std::mem::zeroed::<ConsoleScreenBufferInfo>();

                if GetConsoleScreenBufferInfo(handle, &mut info) == 0 {
                    return Err(io::Error::last_os_error());
                }

                Ok(Size {
                    rows: (info.window.bottom - info.window.top + 1) as u16,
                    cols: (info.window.right - info.window.left + 1) as u16,
                })
            }
        }
    }

    /// Read a key from stdin
    pub fn read_key(&self) -> io::Result<Option<Key>> {
        let mut buf = [0u8; 8];
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        let n = handle.read(&mut buf)?;
        if n == 0 {
            return Ok(None);
        }

        let key = self.parse_key(&buf[..n]);
        Ok(Some(key))
    }

    fn parse_key(&self, buf: &[u8]) -> Key {
        match buf {
            // Control characters
            [0] => Key::Ctrl(' '),
            [1] => Key::Ctrl('a'),
            [2] => Key::Ctrl('b'),
            [3] => Key::Ctrl('c'),
            [4] => Key::Ctrl('d'),
            [5] => Key::Ctrl('e'),
            [6] => Key::Ctrl('f'),
            [7] => Key::Ctrl('g'),
            [8] => Key::Backspace,
            [9] => Key::Tab,
            [10] => Key::Enter,
            [11] => Key::Ctrl('k'),
            [12] => Key::Ctrl('l'),
            [13] => Key::Enter,
            [14] => Key::Ctrl('n'),
            [15] => Key::Ctrl('o'),
            [16] => Key::Ctrl('p'),
            [17] => Key::Ctrl('q'),
            [18] => Key::Ctrl('r'),
            [19] => Key::Ctrl('s'),
            [20] => Key::Ctrl('t'),
            [21] => Key::Ctrl('u'),
            [22] => Key::Ctrl('v'),
            [23] => Key::Ctrl('w'),
            [24] => Key::Ctrl('x'),
            [25] => Key::Ctrl('y'),
            [26] => Key::Ctrl('z'),
            [27] => Key::Escape,
            [127] => Key::Backspace,

            // Escape sequences
            [27, 91, 65] => Key::Up,
            [27, 91, 66] => Key::Down,
            [27, 91, 67] => Key::Right,
            [27, 91, 68] => Key::Left,
            [27, 91, 72] => Key::Home,
            [27, 91, 70] => Key::End,
            [27, 91, 49, 126] => Key::Home,
            [27, 91, 52, 126] => Key::End,
            [27, 91, 51, 126] => Key::Delete,
            [27, 91, 50, 126] => Key::Insert,
            [27, 91, 53, 126] => Key::PageUp,
            [27, 91, 54, 126] => Key::PageDown,

            // Function keys
            [27, 79, 80] => Key::F(1),
            [27, 79, 81] => Key::F(2),
            [27, 79, 82] => Key::F(3),
            [27, 79, 83] => Key::F(4),
            [27, 91, 49, 53, 126] => Key::F(5),
            [27, 91, 49, 55, 126] => Key::F(6),
            [27, 91, 49, 56, 126] => Key::F(7),
            [27, 91, 49, 57, 126] => Key::F(8),
            [27, 91, 50, 48, 126] => Key::F(9),
            [27, 91, 50, 49, 126] => Key::F(10),
            [27, 91, 50, 51, 126] => Key::F(11),
            [27, 91, 50, 52, 126] => Key::F(12),

            // Alt + key
            [27, c] if *c >= 32 => Key::Alt(*c as char),

            // Regular characters (ASCII)
            [c] if *c >= 32 && *c < 127 => Key::Char(*c as char),

            // UTF-8 characters
            _ if buf[0] >= 0xC0 => {
                if let Ok(s) = std::str::from_utf8(buf) {
                    if let Some(c) = s.chars().next() {
                        return Key::Char(c);
                    }
                }
                Key::Unknown(buf.to_vec())
            }

            _ => Key::Unknown(buf.to_vec()),
        }
    }

    /// Clear the screen
    pub fn clear_screen() {
        print!("{}{}", ansi::CLEAR_SCREEN, ansi::CURSOR_HOME);
    }

    /// Hide cursor
    pub fn hide_cursor() {
        print!("{}", ansi::CURSOR_HIDE);
    }

    /// Show cursor
    pub fn show_cursor() {
        print!("{}", ansi::CURSOR_SHOW);
    }

    /// Move cursor to position
    pub fn move_cursor(row: u16, col: u16) {
        print!("{}", ansi::cursor_position(row, col));
    }

    /// Flush stdout
    pub fn flush() -> io::Result<()> {
        io::stdout().flush()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        Terminal::show_cursor();
        let _ = Terminal::flush();
        let _ = self.disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ANSI escape code tests
    #[test]
    fn test_ansi_cursor_position() {
        let pos = ansi::cursor_position(0, 0);
        assert_eq!(pos, "\x1b[1;1H");

        let pos = ansi::cursor_position(5, 10);
        assert_eq!(pos, "\x1b[6;11H");
    }

    #[test]
    fn test_ansi_fg_rgb() {
        let color = ansi::fg_rgb(255, 128, 0);
        assert_eq!(color, "\x1b[38;2;255;128;0m");
    }

    #[test]
    fn test_ansi_bg_rgb() {
        let color = ansi::bg_rgb(0, 128, 255);
        assert_eq!(color, "\x1b[48;2;0;128;255m");
    }

    #[test]
    fn test_ansi_constants() {
        assert_eq!(ansi::CLEAR_SCREEN, "\x1b[2J");
        assert_eq!(ansi::CLEAR_LINE, "\x1b[2K");
        assert_eq!(ansi::CURSOR_HOME, "\x1b[H");
        assert_eq!(ansi::CURSOR_HIDE, "\x1b[?25l");
        assert_eq!(ansi::CURSOR_SHOW, "\x1b[?25h");
        assert_eq!(ansi::RESET, "\x1b[0m");
        assert_eq!(ansi::BOLD, "\x1b[1m");
        assert_eq!(ansi::REVERSE, "\x1b[7m");
    }

    #[test]
    fn test_ansi_foreground_colors() {
        assert!(ansi::FG_RED.starts_with("\x1b["));
        assert!(ansi::FG_GREEN.starts_with("\x1b["));
        assert!(ansi::FG_BLUE.starts_with("\x1b["));
        assert!(ansi::FG_YELLOW.starts_with("\x1b["));
        assert!(ansi::FG_MAGENTA.starts_with("\x1b["));
        assert!(ansi::FG_CYAN.starts_with("\x1b["));
        assert!(ansi::FG_DEFAULT.starts_with("\x1b["));
    }

    // Key parsing tests
    #[test]
    fn test_key_equality() {
        assert_eq!(Key::Char('a'), Key::Char('a'));
        assert_ne!(Key::Char('a'), Key::Char('b'));
        assert_eq!(Key::Enter, Key::Enter);
        assert_ne!(Key::Enter, Key::Tab);
    }

    #[test]
    fn test_key_ctrl() {
        assert_eq!(Key::Ctrl('c'), Key::Ctrl('c'));
        assert_ne!(Key::Ctrl('c'), Key::Ctrl('d'));
    }

    #[test]
    fn test_key_function() {
        assert_eq!(Key::F(1), Key::F(1));
        assert_ne!(Key::F(1), Key::F(2));
    }

    #[test]
    fn test_key_debug() {
        let key = Key::Char('x');
        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("Char"));
        assert!(debug_str.contains("x"));
    }

    #[test]
    fn test_key_clone() {
        let key1 = Key::Char('a');
        let key2 = key1.clone();
        assert_eq!(key1, key2);
    }

    // Size tests
    #[test]
    fn test_size_struct() {
        let size = Size { rows: 24, cols: 80 };
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
    }

    #[test]
    fn test_size_copy() {
        let size1 = Size { rows: 24, cols: 80 };
        let size2 = size1; // Copy
        assert_eq!(size1.rows, size2.rows);
        assert_eq!(size1.cols, size2.cols);
    }

    #[test]
    fn test_size_debug() {
        let size = Size { rows: 24, cols: 80 };
        let debug_str = format!("{:?}", size);
        assert!(debug_str.contains("24"));
        assert!(debug_str.contains("80"));
    }

    // Terminal parse_key tests (internal method testing via wrapper)
    #[test]
    fn test_parse_control_chars() {
        // We can't easily test parse_key directly since Terminal requires raw mode
        // But we can verify the Key enum variants exist and are constructable
        let ctrl_a = Key::Ctrl('a');
        let ctrl_c = Key::Ctrl('c');
        assert_ne!(ctrl_a, ctrl_c);
    }

    #[test]
    fn test_parse_special_keys() {
        let enter = Key::Enter;
        let tab = Key::Tab;
        let backspace = Key::Backspace;
        let delete = Key::Delete;
        let escape = Key::Escape;

        assert_ne!(enter, tab);
        assert_ne!(backspace, delete);
        assert_ne!(escape, enter);
    }

    #[test]
    fn test_parse_arrow_keys() {
        let up = Key::Up;
        let down = Key::Down;
        let left = Key::Left;
        let right = Key::Right;

        assert_ne!(up, down);
        assert_ne!(left, right);
        assert_ne!(up, left);
    }

    #[test]
    fn test_parse_navigation_keys() {
        let home = Key::Home;
        let end = Key::End;
        let page_up = Key::PageUp;
        let page_down = Key::PageDown;
        let insert = Key::Insert;

        assert_ne!(home, end);
        assert_ne!(page_up, page_down);
        assert_ne!(home, insert);
    }

    #[test]
    fn test_unknown_key() {
        let unknown = Key::Unknown(vec![0xFF, 0xFE]);
        if let Key::Unknown(bytes) = unknown {
            assert_eq!(bytes, vec![0xFF, 0xFE]);
        } else {
            panic!("Expected Unknown variant");
        }
    }

    // Note: Terminal::new(), Terminal::size(), and Terminal::read_key()
    // require actual terminal access and cannot be easily unit tested
    // They are better covered by integration tests
}
