//! A crate for safely writing files using an atomic write pattern.
//!
//! This crate implements a safe file writing strategy that helps prevent file corruption
//! in case of system crashes or power failures. It follows these steps:
//!
//! 1. Creates parent directories if they don't exist
//! 2. Writes content to a temporary file
//! 3. Ensures the content is fully written to disk
//! 4. Atomically renames the temporary file to the target path
//!
//! # Examples
//!
//! ```
//! use safe_write::safe_write;
//!
//! let content = b"Hello, World!";
//! safe_write("example.txt", content).expect("Failed to write file");
//! ```
//!
//! # Platform-specific behavior
//!
//! On Windows, if the target file exists, it will be explicitly removed before
//! the rename operation since Windows doesn't support atomic file replacement.

use fs_err as fs;
use std::io::{self, Write};
use std::path::Path;

use fs::OpenOptions;

/// Safely writes content to a file using an atomic write pattern.
///
/// # Arguments
///
/// * `path` - The path where the file should be written
/// * `content` - The bytes to write to the file
///
/// # Returns
///
/// Returns `io::Result<()>` which is:
/// * `Ok(())` if the write was successful
/// * `Err(e)` if any IO operation failed
pub fn safe_write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> io::Result<()> {
    let path = path.as_ref();
    let content = content.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    // Create parent directory if it doesn't exist
    fs::create_dir_all(parent)?;

    // Create a temporary file by appending .tmp to the original path
    let temp_path = path.with_extension("tmp");

    if temp_path.exists() {
        fs::remove_file(&temp_path)?;
    }

    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)?;

    // Write content
    temp_file.write_all(content)?;
    // Flush to OS buffers
    temp_file.flush()?;
    temp_file.sync_all()?;
    // Close the file
    drop(temp_file);

    #[cfg(windows)]
    {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    fs::rename(&temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_basic_write() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let test_path = temp_dir.path().join("test.txt");

        let content = b"Hello, World!";
        safe_write(&test_path, content)?;

        // Verify the content was written correctly
        let read_content = fs::read(&test_path)?;
        assert_eq!(content, read_content.as_slice());

        Ok(())
    }

    #[test]
    fn test_nested_directory_creation() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let test_path = temp_dir.path().join("nested/dirs/test.txt");

        let content = b"Nested content";
        safe_write(&test_path, content)?;

        assert!(test_path.exists());
        let read_content = fs::read(&test_path)?;
        assert_eq!(content, read_content.as_slice());

        Ok(())
    }

    #[test]
    fn test_overwrite_existing() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let test_path = temp_dir.path().join("overwrite.txt");

        // Write initial content
        safe_write(&test_path, b"Initial content")?;

        // Overwrite with new content
        let new_content = b"New content";
        safe_write(&test_path, new_content)?;

        let read_content = fs::read(&test_path)?;
        assert_eq!(new_content, read_content.as_slice());

        Ok(())
    }
}
