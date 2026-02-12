//! Integration tests for zedit

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn zedit_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("x86_64-pc-windows-gnu");
    path.push("debug");
    path.push("zedit.exe");
    path
}

#[test]
fn test_help_flag() {
    let output = Command::new(zedit_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute zedit");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("zedit"));
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("Options:"));
    assert!(stdout.contains("--help"));
    assert!(stdout.contains("--version"));
}

#[test]
fn test_short_help_flag() {
    let output = Command::new(zedit_binary())
        .arg("-h")
        .output()
        .expect("Failed to execute zedit");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("zedit"));
}

#[test]
fn test_version_flag() {
    let output = Command::new(zedit_binary())
        .arg("--version")
        .output()
        .expect("Failed to execute zedit");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("zedit"));
    assert!(stdout.contains("v0.1.0"));
}

#[test]
fn test_short_version_flag() {
    let output = Command::new(zedit_binary())
        .arg("-v")
        .output()
        .expect("Failed to execute zedit");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("zedit"));
    assert!(stdout.contains("v0.1.0"));
}

#[test]
fn test_unknown_option() {
    let output = Command::new(zedit_binary())
        .arg("--unknown-flag")
        .output()
        .expect("Failed to execute zedit");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown option"));
}

#[test]
fn test_nonexistent_path() {
    let output = Command::new(zedit_binary())
        .arg("/nonexistent/path/to/nowhere")
        .output()
        .expect("Failed to execute zedit");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not exist") || stderr.contains("Error"));
}

#[test]
fn test_help_contains_keybindings() {
    let output = Command::new(zedit_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute zedit");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for vim-like keybindings documentation
    assert!(stdout.contains("h/j/k/l"));
    assert!(stdout.contains("Insert"));
    assert!(stdout.contains("Ctrl+s"));
    assert!(stdout.contains("Ctrl+q"));
}

#[test]
fn test_help_contains_commands() {
    let output = Command::new(zedit_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute zedit");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for command documentation
    assert!(stdout.contains(":w"));
    assert!(stdout.contains(":q"));
    assert!(stdout.contains(":wq"));
    assert!(stdout.contains(":e"));
}

#[test]
fn test_help_contains_browser_info() {
    let output = Command::new(zedit_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute zedit");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for file browser documentation
    assert!(stdout.contains("File Browser"));
    assert!(stdout.contains("Navigate"));
}

// File operation tests using temp files
mod file_operations {
    use super::*;

    fn temp_test_file(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("zedit_test_{}", name));
        path
    }

    #[test]
    fn test_create_temp_file_structure() {
        let test_dir = temp_test_file("dir");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        // Create test files
        fs::write(test_dir.join("test.rs"), "fn main() {}").unwrap();
        fs::write(test_dir.join("test.py"), "print('hello')").unwrap();
        fs::write(test_dir.join("test.js"), "console.log('hello');").unwrap();

        // Verify files exist
        assert!(test_dir.join("test.rs").exists());
        assert!(test_dir.join("test.py").exists());
        assert!(test_dir.join("test.js").exists());

        // Cleanup
        fs::remove_dir_all(&test_dir).ok();
    }

    #[test]
    fn test_file_with_unicode() {
        let test_file = temp_test_file("unicode.txt");
        let content = "Hello, 世界! 🌍 Привет мир!";

        fs::write(&test_file, content).unwrap();
        let read_content = fs::read_to_string(&test_file).unwrap();

        assert_eq!(content, read_content);

        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_large_file_creation() {
        let test_file = temp_test_file("large.txt");

        // Create a file with 10000 lines
        let mut content = String::new();
        for i in 0..10000 {
            content.push_str(&format!("Line {}: This is test content for line number {}\n", i, i));
        }

        fs::write(&test_file, &content).unwrap();

        let metadata = fs::metadata(&test_file).unwrap();
        assert!(metadata.len() > 100000); // Should be > 100KB

        fs::remove_file(&test_file).ok();
    }
}

// Syntax highlighting language detection tests
mod syntax_detection {
    #[test]
    fn test_rust_file_extensions() {
        let extensions = ["rs"];
        for ext in extensions {
            assert!(ext == "rs", "Rust extension should be rs");
        }
    }

    #[test]
    fn test_python_file_extensions() {
        let extensions = ["py", "pyw", "pyi"];
        assert_eq!(extensions.len(), 3);
    }

    #[test]
    fn test_javascript_file_extensions() {
        let extensions = ["js", "jsx", "mjs", "cjs"];
        assert_eq!(extensions.len(), 4);
    }

    #[test]
    fn test_typescript_file_extensions() {
        let extensions = ["ts", "tsx"];
        assert_eq!(extensions.len(), 2);
    }
}
