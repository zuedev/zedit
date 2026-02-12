mod browser;
mod buffer;
mod editor;
mod syntax;
mod terminal;

use editor::Editor;
use std::env;
use std::path::PathBuf;
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");

fn print_help() {
    println!("{} v{}", NAME, VERSION);
    println!("Fast, dependency-free editing with syntax highlighting and directory browsing.");
    println!();
    println!("Usage: {} [options] [file/directory]", NAME);
    println!();
    println!("Options:");
    println!("  -h, --help     Show this help message and exit");
    println!("  -v, --version  Show version information and exit");
    println!();
    println!("Keybindings (Normal mode):");
    println!("  h/j/k/l        Move left/down/up/right");
    println!("  w/b            Move word forward/backward");
    println!("  0/$            Move to start/end of line");
    println!("  g/G            Move to first/last line");
    println!("  i/I            Enter insert mode (at cursor/line start)");
    println!("  a/A            Enter insert mode (after cursor/line end)");
    println!("  o/O            Insert new line below/above");
    println!("  x              Delete character");
    println!("  d              Delete line");
    println!("  e              Open file browser");
    println!("  /              Search forward");
    println!("  ?              Search backward");
    println!("  n/N            Next/previous search result");
    println!("  :              Enter command mode");
    println!("  Ctrl+s         Save file");
    println!("  Ctrl+q         Quit");
    println!();
    println!("Commands:");
    println!("  :w             Save file");
    println!("  :w <file>      Save as file");
    println!("  :q             Quit (fails if unsaved changes)");
    println!("  :q!            Force quit");
    println!("  :wq            Save and quit");
    println!("  :e <file>      Edit file");
    println!("  :e             Open file browser");
    println!("  :<number>      Go to line number");
    println!();
    println!("File Browser:");
    println!("  j/k or arrows  Navigate");
    println!("  Enter/l        Open file/directory");
    println!("  h/Backspace    Go to parent directory");
    println!("  .              Toggle hidden files");
    println!("  q/Esc          Close browser");
}

fn print_version() {
    println!("{} v{}", NAME, VERSION);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut path: Option<PathBuf> = None;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-v" | "--version" => {
                print_version();
                return;
            }
            arg if arg.starts_with('-') => {
                eprintln!("Unknown option: {}", arg);
                eprintln!("Use --help for usage information.");
                process::exit(1);
            }
            _ => {
                path = Some(PathBuf::from(arg));
            }
        }
    }

    // Run the editor
    let result = run_editor(path);

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run_editor(path: Option<PathBuf>) -> std::io::Result<()> {
    let mut editor = Editor::new()?;

    if let Some(p) = path {
        if p.exists() {
            editor.open(&p)?;
        } else if p.to_string_lossy().contains('.') {
            // Assume it's a new file
            editor.open(&p).or_else(|_: std::io::Error| -> std::io::Result<()> {
                // Create new file by just setting the path
                Ok(())
            })?;
        } else {
            // Could be a new directory or file
            eprintln!("Path does not exist: {}", p.display());
            std::process::exit(1);
        }
    }

    editor.run()
}
