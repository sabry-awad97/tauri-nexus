//! Pattern matching utilities for cache invalidation

/// Check if a pattern matches a path
///
/// Supports:
/// - Exact match: "user.get" matches "user.get"
/// - Wildcard suffix: "user.*" matches "user.get", "user.create", etc.
/// - Global wildcard: "*" matches everything
pub fn pattern_matches(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix(".*") {
        // Exact match: "user.*" matches "user"
        if path == prefix {
            return true;
        }

        // Prefix match: "user.*" matches "user.get", "user.profile", etc.
        // Avoid allocation by checking length and using byte-level comparison
        if path.len() > prefix.len() + 1
            && path.starts_with(prefix)
            && path.as_bytes()[prefix.len()] == b'.'
        {
            return true;
        }

        return false;
    }

    pattern == path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matches_exact() {
        assert!(pattern_matches("user.get", "user.get"));
        assert!(!pattern_matches("user.get", "user.create"));
    }

    #[test]
    fn test_pattern_matches_wildcard() {
        assert!(pattern_matches("user.*", "user.get"));
        assert!(pattern_matches("user.*", "user.create"));
        assert!(pattern_matches("user.*", "user"));
        assert!(!pattern_matches("user.*", "post.get"));
    }

    #[test]
    fn test_pattern_matches_global() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("*", "user.get"));
    }
}
