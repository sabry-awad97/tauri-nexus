// =============================================================================
// @tauri-nexus/rpc-core - Schema Module
// =============================================================================

// Types
export type {
  SchemaProcedure,
  SchemaContract,
  InferSchemaInput,
  InferSchemaOutput,
  InferSchemaProcedureType,
  InferContractInputs,
  InferContractOutputs,
  InferProcedureInput,
  InferProcedureOutput,
  SchemaContractToContract,
  ExtractEventsType,
  InferEventName,
  EventPayload,
  ExtractEventsOptions,
  ValidationErrorDetails,
  ValidationConfig,
} from "./types";

// Builder
export { ProcedureBuilder, procedure, router, mergeRouters } from "./builder";

// Validation
export { buildSchemaMap, createValidationInterceptor } from "./validation";

// Path extraction
export {
  extractPaths,
  extractSubscriptionPaths,
  extractQueryPaths,
  extractMutationPaths,
  extractPathsByType,
  extractEvents,
} from "./path-extraction";

// Client factories
export {
  createValidatedClient,
  createClientFromSchema,
  type SchemaClientConfig,
} from "./client-factory";
