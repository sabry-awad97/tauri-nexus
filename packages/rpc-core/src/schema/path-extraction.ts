// =============================================================================
// @tauri-nexus/rpc-core - Path Extraction Utilities
// =============================================================================
// Utilities for extracting procedure paths from schema contracts.

import type { ProcedureType } from "../core/types";
import type {
  SchemaProcedure,
  SchemaContract,
  ExtractEventsOptions,
  ExtractEventsType,
} from "./types";

// =============================================================================
// Type Guards
// =============================================================================

/** Check if value is a SchemaProcedure */
function isSchemaProcedure(value: unknown): value is SchemaProcedure {
  return (
    typeof value === "object" &&
    value !== null &&
    "type" in value &&
    "outputSchema" in value &&
    typeof (value as SchemaProcedure).type === "string" &&
    ["query", "mutation", "subscription"].includes(
      (value as SchemaProcedure).type,
    )
  );
}

// =============================================================================
// Path Extraction
// =============================================================================

/**
 * Extract all procedure paths from a schema contract.
 *
 * @example
 * ```typescript
 * const paths = extractPaths(contract);
 * // ['health', 'greet', 'user.get', 'user.list', ...]
 * ```
 */
export function extractPaths(
  contract: SchemaContract,
  prefix: string = "",
): string[] {
  const paths: string[] = [];

  for (const [key, value] of Object.entries(contract)) {
    const path = prefix ? `${prefix}.${key}` : key;

    if (isSchemaProcedure(value)) {
      paths.push(path);
    } else if (typeof value === "object" && value !== null) {
      paths.push(...extractPaths(value as SchemaContract, path));
    }
  }

  return paths;
}

/**
 * Extract subscription paths from a schema contract.
 *
 * @example
 * ```typescript
 * const subscriptionPaths = extractSubscriptionPaths(contract);
 * // ['stream.counter', 'stream.stocks', 'stream.chat']
 * ```
 */
export function extractSubscriptionPaths(
  contract: SchemaContract,
  prefix: string = "",
): string[] {
  const paths: string[] = [];

  for (const [key, value] of Object.entries(contract)) {
    const path = prefix ? `${prefix}.${key}` : key;

    if (isSchemaProcedure(value)) {
      if (value.type === "subscription") {
        paths.push(path);
      }
    } else if (typeof value === "object" && value !== null) {
      paths.push(...extractSubscriptionPaths(value as SchemaContract, path));
    }
  }

  return paths;
}

/**
 * Extract query paths from a schema contract.
 */
export function extractQueryPaths(
  contract: SchemaContract,
  prefix: string = "",
): string[] {
  const paths: string[] = [];

  for (const [key, value] of Object.entries(contract)) {
    const path = prefix ? `${prefix}.${key}` : key;

    if (isSchemaProcedure(value)) {
      if (value.type === "query") {
        paths.push(path);
      }
    } else if (typeof value === "object" && value !== null) {
      paths.push(...extractQueryPaths(value as SchemaContract, path));
    }
  }

  return paths;
}

/**
 * Extract mutation paths from a schema contract.
 */
export function extractMutationPaths(
  contract: SchemaContract,
  prefix: string = "",
): string[] {
  const paths: string[] = [];

  for (const [key, value] of Object.entries(contract)) {
    const path = prefix ? `${prefix}.${key}` : key;

    if (isSchemaProcedure(value)) {
      if (value.type === "mutation") {
        paths.push(path);
      }
    } else if (typeof value === "object" && value !== null) {
      paths.push(...extractMutationPaths(value as SchemaContract, path));
    }
  }

  return paths;
}

/**
 * Extract paths by procedure type from a schema contract.
 */
export function extractPathsByType(
  contract: SchemaContract,
  type: ProcedureType,
  prefix: string = "",
): string[] {
  const paths: string[] = [];

  for (const [key, value] of Object.entries(contract)) {
    const path = prefix ? `${prefix}.${key}` : key;

    if (isSchemaProcedure(value)) {
      if (value.type === type) {
        paths.push(path);
      }
    } else if (typeof value === "object" && value !== null) {
      paths.push(...extractPathsByType(value as SchemaContract, type, path));
    }
  }

  return paths;
}

// =============================================================================
// Event Extraction
// =============================================================================

/**
 * Default key transformer: "stream.counter" -> "COUNTER"
 */
function defaultKeyTransform(path: string): string {
  const segments = path.split(".");
  const lastSegment = segments[segments.length - 1];
  return lastSegment.toUpperCase().replace(/-/g, "_");
}

/**
 * Default value transformer: "stream.counter" -> "counter"
 */
function defaultValueTransform(path: string): string {
  const segments = path.split(".");
  return segments[segments.length - 1];
}

/**
 * Extract event names from subscription paths in a schema contract.
 *
 * @example
 * ```typescript
 * const Events = extractEvents(appContractSchema);
 * // Type: { readonly COUNTER: "counter"; readonly STOCKS: "stocks"; ... }
 *
 * Events.COUNTER // "counter" - fully typed!
 * ```
 */
export function extractEvents<T extends SchemaContract>(
  contract: T,
  options: ExtractEventsOptions = {},
): ExtractEventsType<T> {
  const {
    transformKey = defaultKeyTransform,
    transformValue = defaultValueTransform,
  } = options;

  const subscriptionPaths = extractSubscriptionPaths(contract);
  const events: Record<string, string> = {};

  for (const path of subscriptionPaths) {
    const key = transformKey(path);
    const value = transformValue(path);
    events[key] = value;
  }

  return Object.freeze(events) as ExtractEventsType<T>;
}
