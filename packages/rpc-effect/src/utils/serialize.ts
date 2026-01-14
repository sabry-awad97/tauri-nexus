// =============================================================================
// Serialization Utilities
// =============================================================================

/**
 * Stable JSON stringify that sorts object keys for consistent output.
 */
export const stableStringify = (value: unknown): string => {
  if (value === null || value === undefined) {
    return JSON.stringify(value);
  }

  if (typeof value !== "object") {
    return JSON.stringify(value);
  }

  if (Array.isArray(value)) {
    return "[" + value.map(stableStringify).join(",") + "]";
  }

  const obj = value as Record<string, unknown>;
  const keys = Object.keys(obj).sort();

  if (keys.length === 0) {
    return "{}";
  }

  const pairs = keys.map(
    (key) => `${JSON.stringify(key)}:${stableStringify(obj[key])}`,
  );
  return "{" + pairs.join(",") + "}";
};
