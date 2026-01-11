// =============================================================================
// OpenAPI Documentation Types
// =============================================================================
// TypeScript types matching the Rust backend schema structure for API documentation.

/**
 * Procedure type enumeration matching Rust ProcedureTypeSchema.
 */
export type ProcedureType = "query" | "mutation" | "subscription";

/**
 * Type schema for describing data types.
 * Matches the Rust TypeSchema structure.
 */
export interface TypeSchema {
  /** Type name (e.g., "string", "number", "object", "array") */
  type: string;
  /** For object types, the properties map */
  properties?: Record<string, TypeSchema>;
  /** Required property names for object types */
  required?: string[];
  /** For array types, the item type schema */
  items?: TypeSchema;
  /** Human-readable description */
  description?: string;
  /** Example value */
  example?: unknown;
  /** Enum values for constrained types */
  enum?: unknown[];
  /** Format hint (e.g., "email", "uuid", "date-time") */
  format?: string;
  /** Minimum value for numbers */
  minimum?: number;
  /** Maximum value for numbers */
  maximum?: number;
  /** Minimum length for strings/arrays */
  minLength?: number;
  /** Maximum length for strings/arrays */
  maxLength?: number;
  /** Regex pattern for strings */
  pattern?: string;
  /** Whether the value can be null */
  nullable?: boolean;
}

/**
 * Schema for a single RPC procedure.
 * Matches the Rust ProcedureSchema structure.
 */
export interface ProcedureSchema {
  /** Procedure type (query, mutation, subscription) */
  procedure_type: ProcedureType;
  /** Human-readable description */
  description?: string;
  /** Input type schema */
  input?: TypeSchema;
  /** Output type schema */
  output?: TypeSchema;
  /** Whether the procedure is deprecated */
  deprecated: boolean;
  /** Tags for categorization */
  tags: string[];
  /** Additional metadata */
  metadata?: unknown;
}

/**
 * Schema for a complete router.
 * Matches the Rust RouterSchema structure.
 */
export interface RouterSchema {
  /** Schema version */
  version: string;
  /** Router name/title */
  name?: string;
  /** Router description */
  description?: string;
  /** All procedures in the router, keyed by path */
  procedures: Record<string, ProcedureSchema>;
  /** Additional metadata */
  metadata?: unknown;
}

/**
 * A procedure entry with its path for display purposes.
 */
export interface ProcedureEntry {
  /** Procedure path (e.g., "user.get") */
  path: string;
  /** Procedure schema data */
  schema: ProcedureSchema;
}

/**
 * A group of procedures sharing a common namespace.
 */
export interface ProcedureGroup {
  /** Namespace name (e.g., "user") or empty string for root-level */
  namespace: string;
  /** Procedures in this group */
  procedures: ProcedureEntry[];
}

/**
 * Filter state for the documentation component.
 */
export interface FilterState {
  /** Text search query */
  search: string;
  /** Procedure type filter */
  typeFilter: ProcedureType | "all";
}

/**
 * Result of filtering procedures.
 */
export interface FilterResult {
  /** Filtered procedure entries */
  procedures: ProcedureEntry[];
  /** Count of filtered procedures */
  count: number;
  /** Total count before filtering */
  totalCount: number;
}
