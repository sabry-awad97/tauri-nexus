//! Source file scanner for discovering Rust files.
//!
//! This module provides functionality to recursively scan directories
//! for Rust source files, respecting `.gitignore` patterns and custom filters.

use crate::error::{CliResult, ScanError};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// A discovered source file with its content.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Absolute path to the file.
    pub path: PathBuf,

    /// Path relative to the scan root.
    pub relative_path: PathBuf,

    /// File content.
    pub content: String,
}

/// Scanner for discovering Rust source files.
#[derive(Debug)]
pub struct SourceScanner {
    /// Root directory to scan.
    root: PathBuf,

    /// Whether to respect .gitignore files.
    respect_gitignore: bool,

    /// Optional glob filter pattern.
    filter: Option<glob::Pattern>,
}

impl SourceScanner {
    /// Create a new scanner for the given root directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            respect_gitignore: true,
            filter: None,
        }
    }

    /// Set whether to respect .gitignore files.
    pub fn with_gitignore(mut self, respect: bool) -> Self {
        self.respect_gitignore = respect;
        self
    }

    /// Set a glob filter pattern for files.
    ///
    /// Only files matching the pattern will be included.
    pub fn with_filter(mut self, pattern: &str) -> Result<Self, ScanError> {
        let glob_pattern = glob::Pattern::new(pattern)
            .map_err(|e| ScanError::invalid_pattern(pattern, e.to_string()))?;
        self.filter = Some(glob_pattern);
        Ok(self)
    }

    /// Scan the directory and return all discovered Rust files.
    pub fn scan(&self) -> CliResult<Vec<SourceFile>> {
        // Verify root exists
        if !self.root.exists() {
            return Err(ScanError::not_found(self.root.clone()).into());
        }

        let mut files = Vec::new();

        // Build walker with gitignore support
        let walker = WalkBuilder::new(&self.root)
            .git_ignore(self.respect_gitignore)
            .git_global(self.respect_gitignore)
            .git_exclude(self.respect_gitignore)
            .hidden(false) // Don't skip hidden files by default
            .build();

        for entry in walker {
            let entry = entry.map_err(ScanError::Walk)?;
            let path = entry.path();

            // Skip directories
            if !path.is_file() {
                continue;
            }

            // Only process .rs files
            if path.extension().map_or(true, |ext| ext != "rs") {
                continue;
            }

            // Apply filter if set
            if let Some(ref pattern) = self.filter {
                let relative = self.relative_path(path);
                if !pattern.matches_path(&relative) {
                    continue;
                }
            }

            // Read file content
            let content = std::fs::read_to_string(path).map_err(|e| ScanError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;

            files.push(SourceFile {
                path: path.to_path_buf(),
                relative_path: self.relative_path(path),
                content,
            });
        }

        // Warn if no files found
        if files.is_empty() {
            return Err(ScanError::no_rust_files(self.root.clone()).into());
        }

        Ok(files)
    }

    /// Scan without failing on empty results.
    ///
    /// Returns an empty vector if no files are found.
    pub fn scan_allow_empty(&self) -> CliResult<Vec<SourceFile>> {
        match self.scan() {
            Ok(files) => Ok(files),
            Err(crate::error::CliError::Scan(ScanError::NoRustFiles { .. })) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// Get the relative path from root.
    fn relative_path(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root).unwrap_or(path).to_path_buf()
    }

    /// Get the root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create some Rust files
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub mod foo;").unwrap();

        // Create a subdirectory with more files
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/foo.rs"), "pub fn foo() {}").unwrap();
        fs::write(dir.path().join("src/bar.rs"), "pub fn bar() {}").unwrap();

        // Create a non-Rust file
        fs::write(dir.path().join("README.md"), "# Test").unwrap();

        dir
    }

    #[test]
    fn test_scan_finds_all_rust_files() {
        let dir = create_test_dir();
        let scanner = SourceScanner::new(dir.path());

        let files = scanner.scan().unwrap();

        assert_eq!(files.len(), 4);

        let paths: Vec<_> = files
            .iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();
        assert!(paths.iter().any(|p| p.ends_with("main.rs")));
        assert!(paths.iter().any(|p| p.ends_with("lib.rs")));
        assert!(paths.iter().any(|p| p.contains("foo.rs")));
        assert!(paths.iter().any(|p| p.contains("bar.rs")));
    }

    #[test]
    fn test_scan_excludes_non_rust_files() {
        let dir = create_test_dir();
        let scanner = SourceScanner::new(dir.path());

        let files = scanner.scan().unwrap();

        for file in &files {
            assert!(file.path.extension().is_some_and(|ext| ext == "rs"));
        }
    }

    #[test]
    fn test_scan_with_filter() {
        let dir = create_test_dir();
        let scanner = SourceScanner::new(dir.path())
            .with_filter("**/foo.rs")
            .unwrap();

        let files = scanner.scan().unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string_lossy().contains("foo.rs"));
    }

    #[test]
    fn test_scan_nonexistent_directory() {
        let scanner = SourceScanner::new("/nonexistent/path");

        let result = scanner.scan();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CliError::Scan(ScanError::DirectoryNotFound { .. })
        ));
    }

    #[test]
    fn test_scan_empty_directory() {
        let dir = TempDir::new().unwrap();
        let scanner = SourceScanner::new(dir.path());

        let result = scanner.scan();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CliError::Scan(ScanError::NoRustFiles { .. })
        ));
    }

    #[test]
    fn test_scan_allow_empty() {
        let dir = TempDir::new().unwrap();
        let scanner = SourceScanner::new(dir.path());

        let files = scanner.scan_allow_empty().unwrap();

        assert!(files.is_empty());
    }

    #[test]
    fn test_source_file_content() {
        let dir = create_test_dir();
        let scanner = SourceScanner::new(dir.path());

        let files = scanner.scan().unwrap();
        let main_file = files
            .iter()
            .find(|f| f.relative_path.to_string_lossy().ends_with("main.rs"))
            .unwrap();

        assert_eq!(main_file.content, "fn main() {}");
    }
}
