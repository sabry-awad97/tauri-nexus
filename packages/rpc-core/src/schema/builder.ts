// =============================================================================
// @tauri-nexus/rpc-core - Schema Builder
// =============================================================================
// Fluent builder for creating schema-validated procedures.

import type { z } from "zod";
import type { SchemaProcedure, SchemaContract } from "./types";

// =============================================================================
// Procedure Builder
// =============================================================================

/**
 * Fluent builder for creating schema-validated procedures.
 *
 * @example
 * ```typescript
 * const getUserProcedure = procedure()
 *   .input(z.object({ id: z.number() }))
 *   .output(z.object({ id: z.number(), name: z.string() }))
 *   .query();
 * ```
 */
export class ProcedureBuilder<
  TInputSchema extends z.ZodTypeAny | null = null,
  TOutputSchema extends z.ZodTypeAny | null = null,
> {
  private _inputSchema: TInputSchema;
  private _outputSchema: TOutputSchema;

  constructor(
    inputSchema: TInputSchema = null as TInputSchema,
    outputSchema: TOutputSchema = null as TOutputSchema,
  ) {
    this._inputSchema = inputSchema;
    this._outputSchema = outputSchema;
  }

  /**
   * Define the input schema for this procedure.
   */
  input<T extends z.ZodTypeAny>(schema: T): ProcedureBuilder<T, TOutputSchema> {
    return new ProcedureBuilder(schema, this._outputSchema);
  }

  /**
   * Define the output schema for this procedure.
   */
  output<T extends z.ZodTypeAny>(schema: T): ProcedureBuilder<TInputSchema, T> {
    return new ProcedureBuilder(this._inputSchema, schema);
  }

  /**
   * Create a query procedure (for reading data).
   */
  query(): TOutputSchema extends z.ZodTypeAny
    ? SchemaProcedure<"query", TInputSchema, TOutputSchema>
    : never {
    if (!this._outputSchema) {
      throw new Error("Output schema is required before calling query()");
    }
    return {
      type: "query",
      inputSchema: this._inputSchema,
      outputSchema: this._outputSchema,
    } as TOutputSchema extends z.ZodTypeAny
      ? SchemaProcedure<"query", TInputSchema, TOutputSchema>
      : never;
  }

  /**
   * Create a mutation procedure (for writing data).
   */
  mutation(): TOutputSchema extends z.ZodTypeAny
    ? SchemaProcedure<"mutation", TInputSchema, TOutputSchema>
    : never {
    if (!this._outputSchema) {
      throw new Error("Output schema is required before calling mutation()");
    }
    return {
      type: "mutation",
      inputSchema: this._inputSchema,
      outputSchema: this._outputSchema,
    } as TOutputSchema extends z.ZodTypeAny
      ? SchemaProcedure<"mutation", TInputSchema, TOutputSchema>
      : never;
  }

  /**
   * Create a subscription procedure (for streaming data).
   */
  subscription(): TOutputSchema extends z.ZodTypeAny
    ? SchemaProcedure<"subscription", TInputSchema, TOutputSchema>
    : never {
    if (!this._outputSchema) {
      throw new Error(
        "Output schema is required before calling subscription()",
      );
    }
    return {
      type: "subscription",
      inputSchema: this._inputSchema,
      outputSchema: this._outputSchema,
    } as TOutputSchema extends z.ZodTypeAny
      ? SchemaProcedure<"subscription", TInputSchema, TOutputSchema>
      : never;
  }
}

/**
 * Start building a new procedure with Zod schema validation.
 *
 * @example
 * ```typescript
 * const health = procedure()
 *   .output(z.object({ status: z.string() }))
 *   .query();
 * ```
 */
export function procedure(): ProcedureBuilder {
  return new ProcedureBuilder();
}

// =============================================================================
// Router Utilities
// =============================================================================

/**
 * Create a router from procedures and nested routers.
 *
 * @example
 * ```typescript
 * const contract = router({
 *   health: procedure().output(z.object({ status: z.string() })).query(),
 *   user: router({
 *     get: procedure()
 *       .input(z.object({ id: z.number() }))
 *       .output(UserSchema)
 *       .query(),
 *   }),
 * });
 * ```
 */
export function router<T extends SchemaContract>(routes: T): T {
  return routes;
}

/**
 * Merge multiple routers into one.
 * Later routers override earlier ones on key conflicts.
 */
export function mergeRouters<T extends SchemaContract[]>(
  ...routers: T
): T[number] {
  return Object.assign({}, ...routers);
}
