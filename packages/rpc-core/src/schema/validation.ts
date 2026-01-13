// =============================================================================
// @tauri-nexus/rpc-core - Schema Validation
// =============================================================================
// Validation interceptor and utilities for Zod schema validation.

import { z } from "zod";
import type { ProcedureType, RpcError } from "../core/types";
import type { LinkInterceptor, LinkRequestContext } from "../link/types";
import type {
  SchemaProcedure,
  SchemaContract,
  ValidationConfig,
  ValidationErrorDetails,
} from "./types";

// =============================================================================
// Schema Map Builder
// =============================================================================

/** Entry in the schema map */
interface SchemaMapEntry {
  inputSchema: z.ZodTypeAny | null;
  outputSchema: z.ZodTypeAny;
  type: ProcedureType;
}

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

/**
 * Build a map of path -> schemas from a SchemaContract.
 */
export function buildSchemaMap(
  contract: SchemaContract,
  prefix: string = "",
): Map<string, SchemaMapEntry> {
  const map = new Map<string, SchemaMapEntry>();

  for (const [key, value] of Object.entries(contract)) {
    const path = prefix ? `${prefix}.${key}` : key;

    if (isSchemaProcedure(value)) {
      map.set(path, {
        inputSchema: value.inputSchema,
        outputSchema: value.outputSchema,
        type: value.type,
      });
    } else if (typeof value === "object" && value !== null) {
      const nestedMap = buildSchemaMap(value as SchemaContract, path);
      for (const [nestedPath, entry] of nestedMap) {
        map.set(nestedPath, entry);
      }
    }
  }

  return map;
}

// =============================================================================
// Validation Error Creation
// =============================================================================

/**
 * Create a validation error with Zod details.
 */
function createValidationError(
  type: "input" | "output",
  path: string,
  error: z.ZodError,
): RpcError {
  return {
    code: "VALIDATION_ERROR",
    message: `${type === "input" ? "Input" : "Output"} validation failed for ${path}`,
    details: {
      type,
      path,
      issues: error.issues.map((issue) => ({
        path: issue.path.join("."),
        message: issue.message,
        code: issue.code,
      })),
    } satisfies ValidationErrorDetails,
  };
}

// =============================================================================
// Validation Interceptor
// =============================================================================

/**
 * Create a validation interceptor for a schema contract.
 *
 * @example
 * ```typescript
 * const interceptor = createValidationInterceptor(contract, {
 *   validateInput: true,
 *   validateOutput: true,
 * });
 * ```
 */
export function createValidationInterceptor<T extends SchemaContract>(
  contract: T,
  config: ValidationConfig = {},
): LinkInterceptor {
  const {
    validateInput = true,
    validateOutput = true,
    strict = false,
    onValidationError,
  } = config;

  const schemaMap = buildSchemaMap(contract);

  return async <T>(
    ctx: LinkRequestContext,
    next: () => Promise<T>,
  ): Promise<T> => {
    const schemas = schemaMap.get(ctx.path);

    // Input validation
    if (
      validateInput &&
      schemas?.inputSchema &&
      ctx.input !== null &&
      ctx.input !== undefined
    ) {
      const schema = strict
        ? ((schemas.inputSchema as z.ZodObject<z.ZodRawShape>).strict?.() ??
          schemas.inputSchema)
        : schemas.inputSchema;
      const result = schema.safeParse(ctx.input);

      if (!result.success) {
        onValidationError?.(result.error, { path: ctx.path, type: "input" });
        throw createValidationError("input", ctx.path, result.error);
      }

      ctx.input = result.data;
    }

    const response = await next();

    // Output validation
    if (validateOutput && schemas?.outputSchema) {
      const schema = strict
        ? ((schemas.outputSchema as z.ZodObject<z.ZodRawShape>).strict?.() ??
          schemas.outputSchema)
        : schemas.outputSchema;
      const result = schema.safeParse(response);

      if (!result.success) {
        onValidationError?.(result.error, { path: ctx.path, type: "output" });
        throw createValidationError("output", ctx.path, result.error);
      }

      return result.data as T;
    }

    return response;
  };
}
