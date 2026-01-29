//! Cache entry and key generation

use std::time::{Duration, Instant};

/// A cached entry with expiration tracking
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached value
    pub value: serde_json::Value,
    /// When the entry was created
    pub created_at: Instant,
    /// Time-to-live for this entry
    pub ttl: Duration,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(value: serde_json::Value, ttl: Duration) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            ttl,
        }
    }

    /// Check if the entry has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.ttl
    }

    /// Get the remaining TTL
    pub fn remaining_ttl(&self) -> Duration {
        self.ttl.saturating_sub(self.created_at.elapsed())
    }
}

/// Generate a deterministic cache key from path and input
pub fn generate_cache_key(path: &str, input: &serde_json::Value) -> String {
    // Normalize the input to ensure deterministic key generation
    let normalized_input = normalize_json(input);
    format!("{}:{}", path, normalized_input)
}

/// Normalize JSON for deterministic key generation
fn normalize_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(normalize_json).collect();
            format!("[{}]", items.join(","))
        }
        serde_json::Value::Object(obj) => {
            // Sort keys for deterministic ordering
            let mut pairs: Vec<_> = obj.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            let items: Vec<String> = pairs
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, normalize_json(v)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cache_key_determinism() {
        let input1 = json!({"b": 2, "a": 1});
        let input2 = json!({"a": 1, "b": 2});

        let key1 = generate_cache_key("test", &input1);
        let key2 = generate_cache_key("test", &input2);

        // Keys should be the same regardless of object key order
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_different_inputs() {
        let input1 = json!({"id": 1});
        let input2 = json!({"id": 2});

        let key1 = generate_cache_key("test", &input1);
        let key2 = generate_cache_key("test", &input2);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_key_different_paths() {
        let input = json!({"id": 1});

        let key1 = generate_cache_key("user.get", &input);
        let key2 = generate_cache_key("post.get", &input);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_normalize_json_primitives() {
        assert_eq!(normalize_json(&json!(null)), "null");
        assert_eq!(normalize_json(&json!(true)), "true");
        assert_eq!(normalize_json(&json!(false)), "false");
        assert_eq!(normalize_json(&json!(42)), "42");
        assert_eq!(normalize_json(&json!(3.1)), "3.1");
        assert_eq!(normalize_json(&json!("hello")), "\"hello\"");
    }

    #[test]
    fn test_normalize_json_array() {
        assert_eq!(normalize_json(&json!([1, 2, 3])), "[1,2,3]");
        assert_eq!(normalize_json(&json!(["a", "b"])), "[\"a\",\"b\"]");
    }

    #[test]
    fn test_normalize_json_object_sorted() {
        let obj = json!({"z": 1, "a": 2, "m": 3});
        let normalized = normalize_json(&obj);
        assert_eq!(normalized, "{\"a\":2,\"m\":3,\"z\":1}");
    }
}
