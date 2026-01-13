//! File watcher for development mode.
//!
//! This module provides file system watching functionality
//! to automatically regenerate schemas when source files change.

use crate::error::{CliResult, WatchError};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// Event types for file changes.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A file was modified.
    Modified(PathBuf),
    /// A file was created.
    Created(PathBuf),
    /// A file was deleted.
    Deleted(PathBuf),
    /// An error occurred.
    Error(String),
}

/// File watcher for monitoring Rust source files.
pub struct FileWatcher {
    /// Root directory to watch.
    root: PathBuf,
    /// Debounce duration in milliseconds.
    debounce_ms: u64,
}

impl FileWatcher {
    /// Create a new file watcher for the given directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            debounce_ms: 500,
        }
    }

    /// Set the debounce duration in milliseconds.
    pub fn with_debounce(mut self, ms: u64) -> Self {
        self.debounce_ms = ms;
        self
    }

    /// Start watching for file changes.
    ///
    /// Returns a receiver that yields watch events.
    pub fn watch(&self) -> CliResult<(Debouncer<RecommendedWatcher>, Receiver<WatchEvent>)> {
        let (tx, rx) = channel::<WatchEvent>();
        let (_event_tx, event_rx) = channel::<()>();

        // Create debouncer
        let debouncer = new_debouncer(
            Duration::from_millis(self.debounce_ms),
            move |result: Result<Vec<DebouncedEvent>, notify::Error>| {
                match result {
                    Ok(events) => {
                        for event in events {
                            let path = event.path;

                            // Only process .rs files
                            if path.extension().map_or(true, |ext| ext != "rs") {
                                continue;
                            }

                            let watch_event = if path.exists() {
                                WatchEvent::Modified(path)
                            } else {
                                WatchEvent::Deleted(path)
                            };

                            let _ = tx.send(watch_event);
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(WatchEvent::Error(e.to_string()));
                    }
                }
            },
        )
        .map_err(|e| WatchError::Init(e.to_string()))?;

        // Forward events to the returned receiver
        std::thread::spawn(move || {
            while let Ok(_event) = event_rx.recv() {
                // This thread just keeps the channel alive
            }
        });

        // Start watching
        let mut debouncer = debouncer;
        debouncer
            .watcher()
            .watch(&self.root, RecursiveMode::Recursive)
            .map_err(|e| WatchError::Init(e.to_string()))?;

        // Create a new receiver that wraps the internal one
        let (final_tx, final_rx) = channel();
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                if final_tx.send(event).is_err() {
                    break;
                }
            }
        });

        Ok((debouncer, final_rx))
    }

    /// Get the root directory being watched.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl WatchEvent {
    /// Get the path associated with this event.
    pub fn path(&self) -> Option<&Path> {
        match self {
            WatchEvent::Modified(p) | WatchEvent::Created(p) | WatchEvent::Deleted(p) => Some(p),
            WatchEvent::Error(_) => None,
        }
    }

    /// Check if this is an error event.
    pub fn is_error(&self) -> bool {
        matches!(self, WatchEvent::Error(_))
    }

    /// Get the error message if this is an error event.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            WatchEvent::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_event_path() {
        let path = PathBuf::from("/test/file.rs");

        let modified = WatchEvent::Modified(path.clone());
        assert_eq!(modified.path(), Some(path.as_path()));

        let created = WatchEvent::Created(path.clone());
        assert_eq!(created.path(), Some(path.as_path()));

        let deleted = WatchEvent::Deleted(path.clone());
        assert_eq!(deleted.path(), Some(path.as_path()));

        let error = WatchEvent::Error("test error".to_string());
        assert_eq!(error.path(), None);
    }

    #[test]
    fn test_watch_event_is_error() {
        let modified = WatchEvent::Modified(PathBuf::from("/test"));
        assert!(!modified.is_error());

        let error = WatchEvent::Error("test".to_string());
        assert!(error.is_error());
    }

    #[test]
    fn test_watch_event_error_message() {
        let modified = WatchEvent::Modified(PathBuf::from("/test"));
        assert_eq!(modified.error_message(), None);

        let error = WatchEvent::Error("test error".to_string());
        assert_eq!(error.error_message(), Some("test error"));
    }

    #[test]
    fn test_file_watcher_new() {
        let watcher = FileWatcher::new("/test/path");
        assert_eq!(watcher.root(), Path::new("/test/path"));
        assert_eq!(watcher.debounce_ms, 500);
    }

    #[test]
    fn test_file_watcher_with_debounce() {
        let watcher = FileWatcher::new("/test/path").with_debounce(1000);
        assert_eq!(watcher.debounce_ms, 1000);
    }
}
