//! File writer for outputting generated schemas.
//!
//! This module handles writing generated TypeScript files to disk,
//! with support for dry-run mode.

use crate::error::{CliResult, WriteError};
use std::path::{Path, PathBuf};

/// Result of a write operation.
#[derive(Debug)]
pub enum WriteResult {
    /// File was written successfully.
    Written {
        /// Path to the written file.
        path: PathBuf,
        /// Number of bytes written.
        bytes: usize,
    },
    /// Dry run - content was not written.
    DryRun {
        /// Content that would have been written.
        content: String,
        /// Path where content would have been written.
        path: PathBuf,
    },
}

/// File writer with dry-run support.
#[derive(Debug)]
pub struct FileWriter {
    /// Whether to run in dry-run mode.
    dry_run: bool,
}

impl FileWriter {
    /// Create a new file writer.
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Write content to a file.
    ///
    /// In dry-run mode, returns the content without writing.
    pub fn write(&self, path: &Path, content: &str) -> CliResult<WriteResult> {
        if self.dry_run {
            return Ok(WriteResult::DryRun {
                content: content.to_string(),
                path: path.to_path_buf(),
            });
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| WriteError::CreateDir {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
        }

        // Write the file
        std::fs::write(path, content).map_err(|e| WriteError::WriteFile {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(WriteResult::Written {
            path: path.to_path_buf(),
            bytes: content.len(),
        })
    }

    /// Check if running in dry-run mode.
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

impl WriteResult {
    /// Get the path associated with this result.
    pub fn path(&self) -> &Path {
        match self {
            WriteResult::Written { path, .. } => path,
            WriteResult::DryRun { path, .. } => path,
        }
    }

    /// Check if the write was successful (not dry-run).
    pub fn was_written(&self) -> bool {
        matches!(self, WriteResult::Written { .. })
    }

    /// Get the number of bytes written (0 for dry-run).
    pub fn bytes(&self) -> usize {
        match self {
            WriteResult::Written { bytes, .. } => *bytes,
            WriteResult::DryRun { .. } => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("output.ts");
        let content = "export const test = z.string();";

        let writer = FileWriter::new(false);
        let result = writer.write(&path, content).unwrap();

        assert!(matches!(result, WriteResult::Written { .. }));
        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    }

    #[test]
    fn test_write_creates_directories() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested/dir/output.ts");
        let content = "export const test = z.string();";

        let writer = FileWriter::new(false);
        let result = writer.write(&path, content).unwrap();

        assert!(matches!(result, WriteResult::Written { .. }));
        assert!(path.exists());
    }

    #[test]
    fn test_dry_run_does_not_write() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("output.ts");
        let content = "export const test = z.string();";

        let writer = FileWriter::new(true);
        let result = writer.write(&path, content).unwrap();

        assert!(matches!(result, WriteResult::DryRun { .. }));
        assert!(!path.exists());

        if let WriteResult::DryRun {
            content: dry_content,
            ..
        } = result
        {
            assert_eq!(dry_content, content);
        }
    }

    #[test]
    fn test_write_result_path() {
        let path = PathBuf::from("/test/path.ts");

        let written = WriteResult::Written {
            path: path.clone(),
            bytes: 100,
        };
        assert_eq!(written.path(), &path);

        let dry_run = WriteResult::DryRun {
            content: "test".to_string(),
            path: path.clone(),
        };
        assert_eq!(dry_run.path(), &path);
    }

    #[test]
    fn test_write_result_was_written() {
        let written = WriteResult::Written {
            path: PathBuf::from("/test"),
            bytes: 100,
        };
        assert!(written.was_written());

        let dry_run = WriteResult::DryRun {
            content: "test".to_string(),
            path: PathBuf::from("/test"),
        };
        assert!(!dry_run.was_written());
    }

    #[test]
    fn test_write_result_bytes() {
        let written = WriteResult::Written {
            path: PathBuf::from("/test"),
            bytes: 100,
        };
        assert_eq!(written.bytes(), 100);

        let dry_run = WriteResult::DryRun {
            content: "test".to_string(),
            path: PathBuf::from("/test"),
        };
        assert_eq!(dry_run.bytes(), 0);
    }
}
