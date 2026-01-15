// =============================================================================
// Schema Module - Effect Schema-based Validation
// =============================================================================
// Provides compile-time type safety and runtime validation using Effect Schema.
//
// Note: Core schemas (RpcPathSchema, RpcRequestSchema, etc.) are available
// but not exported by default. Use the validation functions in call.ts
// (callWithSchema, resilientCallWithSchema) for schema-validated RPC calls.

export {
  // Error schema conversion (used internally by call.ts)
  createSchemaValidationError,
  mapSchemaError,
  schemaIssueToValidationIssue,
} from "./error-schemas";
